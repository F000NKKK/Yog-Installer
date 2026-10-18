//! Running an installer's tool (rustup, cargo, a setup program): its output
//! line by line, stopped when the operation is cancelled.

use crate::progress::{CANCELLED, Cancel};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// How often a running tool is checked for a cancel.
const POLL: Duration = Duration::from_millis(100);

/// Runs `program args`, each line it prints to `line`; an error when it fails or is cancelled.
pub fn tool(
    program: &Path,
    args: &[String],
    cancel: &Cancel,
    line: &dyn Fn(String),
) -> Result<(), String> {
    let name = program.display().to_string();
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{name}: {e}"))?;
    let (tx, rx) = mpsc::channel::<String>();
    let readers: Vec<_> = [
        child
            .stdout
            .take()
            .map(|o| Box::new(o) as Box<dyn std::io::Read + Send>),
        child.stderr.take().map(|e| Box::new(e) as _),
    ]
    .into_iter()
    .flatten()
    .map(|stream| {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for l in BufReader::new(stream).lines().map_while(Result::ok) {
                let _ = tx.send(l);
            }
        })
    })
    .collect();
    drop(tx);
    let status = loop {
        while let Ok(l) = rx.try_recv() {
            line(l);
        }
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CANCELLED.into());
        }
        match child.try_wait().map_err(|e| format!("{name}: {e}"))? {
            Some(status) => break status,
            None => std::thread::sleep(POLL),
        }
    };
    for r in readers {
        let _ = r.join();
    }
    for l in rx.try_iter() {
        line(l);
    }
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} {} — {status}", args.join(" ")))
    }
}
