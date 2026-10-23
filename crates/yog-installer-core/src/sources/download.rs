//! Downloads: resumed where they stopped — after a pause, a dropped
//! connection or a crash, the partial file waits in the cache — verified,
//! and, for whoever installs while it downloads, streamed to them as the
//! bytes arrive.

use crate::plans::progress::{CANCELLED, Cancel};
use crate::sources::http::{agent, hex, ok};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

/// HTTP's answer to a `Range` request it honors, and to one past the end (all there).
const PARTIAL_CONTENT: u16 = 206;
const RANGE_NOT_SATISFIABLE: u16 = 416;
/// Bytes read between progress reports and pause checks.
const CHUNK: usize = 64 * 1024;
/// Chunks the download may be ahead of the one installing it.
const PIPE_DEPTH: usize = 64;
/// The installer stopped reading (its own error says why).
const PIPE_CLOSED: &str = "the installing side stopped";
/// The server can't resume a streamed download: it has to start over.
const NO_RESUME: &str = "the server can't resume this download — it starts over next time";

/// The download's bytes, for the one installing it while it downloads.
pub struct Pipe {
    rx: Receiver<Vec<u8>>,
    chunk: Vec<u8>,
    at: usize,
}

impl Read for Pipe {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.at == self.chunk.len() {
            match self.rx.recv() {
                Ok(chunk) => (self.chunk, self.at) = (chunk, 0),
                Err(_) => return Ok(0),
            }
        }
        let n = buf.len().min(self.chunk.len() - self.at);
        buf[..n].copy_from_slice(&self.chunk[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

/// Downloads `url` to `dest` (see [`stream`]).
pub fn download(
    url: &str,
    dest: &Path,
    sha256: Option<&str>,
    cancel: &Cancel,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<(), String> {
    stream(
        url,
        dest,
        sha256,
        cancel,
        progress,
        None::<fn(Pipe) -> Result<(), String>>,
    )
}

/// Downloads `url` to `dest` — on from what `dest` already has — telling
/// `progress(done, total)`; `consume`, when given, gets every byte as it
/// comes (on its own thread). When `sha256` (hex) is given the file must
/// have it. The partial file stays for next time unless the download was
/// cancelled or is damaged.
pub fn stream<F>(
    url: &str,
    dest: &Path,
    sha256: Option<&str>,
    cancel: &Cancel,
    progress: &dyn Fn(u64, Option<u64>),
    consume: Option<F>,
) -> Result<(), String>
where
    F: FnOnce(Pipe) -> Result<(), String> + Send,
{
    let result = std::thread::scope(|scope| {
        let (tx, consumer) = match consume {
            Some(f) => {
                let (tx, rx) = sync_channel(PIPE_DEPTH);
                (
                    Some(tx),
                    Some(scope.spawn(move || {
                        f(Pipe {
                            rx,
                            chunk: Vec::new(),
                            at: 0,
                        })
                    })),
                )
            }
            None => (None, None),
        };
        let fetched = fetch(url, dest, sha256, cancel, progress, tx.as_ref());
        drop(tx);
        let consumed = consumer.map_or(Ok(()), |c| {
            c.join()
                .unwrap_or_else(|_| Err("installing stopped".into()))
        });
        match (fetched, consumed) {
            (Ok(()), consumed) => consumed,
            (Err(e), Err(why)) if e == PIPE_CLOSED => Err(why),
            (Err(e), _) => Err(e),
        }
    });
    if let Err(e) = &result
        && (e == CANCELLED || e.contains(DAMAGED) || e == NO_RESUME)
    {
        let _ = std::fs::remove_file(dest);
    }
    result
}

/// What a download that doesn't have its hash is called.
const DAMAGED: &str = "the download is damaged";

fn fetch(
    url: &str,
    dest: &Path,
    sha256: Option<&str>,
    cancel: &Cancel,
    progress: &dyn Fn(u64, Option<u64>),
    pipe: Option<&SyncSender<Vec<u8>>>,
) -> Result<(), String> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let at = |e: std::io::Error| format!("{}: {e}", dest.display());
    let mut file = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dest)
        .map_err(at)?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    // What a run before this one left: hashed, and handed on first.
    let mut done = 0u64;
    loop {
        let n = file.read(&mut buf).map_err(at)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        send(pipe, &buf[..n])?;
        done += n as u64;
    }
    let mut total = None;
    // One connection per stretch between pauses: a pause closes it, so the
    // download really stops; going on asks for the rest.
    loop {
        cancel.check()?;
        let mut req = agent().get(url);
        if done > 0 {
            req = req.header("Range", &format!("bytes={done}-"));
        }
        let resp = req.call().map_err(|e| format!("{url}: {e}"))?;
        let status = resp.status().as_u16();
        if done > 0 && status == RANGE_NOT_SATISFIABLE {
            return verify(url, hash, sha256);
        }
        let resp = ok(url, resp)?;
        if done > 0 && status != PARTIAL_CONTENT {
            // The server sends it all again: fine for a file, not for a stream already handed on.
            if pipe.is_some() {
                return Err(NO_RESUME.into());
            }
            file.set_len(0).map_err(at)?;
            file.seek(SeekFrom::Start(0)).map_err(at)?;
            (hash, done) = (Sha256::new(), 0);
        }
        let length: Option<u64> = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        total = total.or(length.map(|l| l + done));
        let mut body = resp.into_body().into_reader();
        while !cancel.is_paused() {
            cancel.check()?;
            let n = body.read(&mut buf).map_err(|e| format!("{url}: {e}"))?;
            if n == 0 {
                return verify(url, hash, sha256);
            }
            file.write_all(&buf[..n]).map_err(at)?;
            hash.update(&buf[..n]);
            send(pipe, &buf[..n])?;
            done += n as u64;
            progress(done, total);
        }
    }
}

fn send(pipe: Option<&SyncSender<Vec<u8>>>, bytes: &[u8]) -> Result<(), String> {
    match pipe {
        Some(tx) => tx.send(bytes.to_vec()).map_err(|_| PIPE_CLOSED.to_string()),
        None => Ok(()),
    }
}

/// The finished download's hash against the one it must have.
fn verify(url: &str, hash: Sha256, sha256: Option<&str>) -> Result<(), String> {
    let Some(want) = sha256 else { return Ok(()) };
    let got = hex(&hash.finalize());
    if got.eq_ignore_ascii_case(want) {
        Ok(())
    } else {
        Err(format!("{url}: {DAMAGED} (sha256 {got}, not {want})"))
    }
}
