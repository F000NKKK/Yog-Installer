//! Running an installer's tool (rustup, cargo, a setup program): its output
//! line by line; paused, resumed and stopped with the operation. On Unix
//! the tool runs in a process group of its own, so what it starts (cargo's
//! rustc) is paused and stopped with it; elsewhere a pause waits for the
//! step to end.

use crate::plans::progress::{CANCELLED, Cancel};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Stdio;
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
    let mut command = substrate_platform::process::command(program);
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
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
    let mut stopped = false;
    let status = loop {
        while let Ok(l) = rx.try_recv() {
            line(l);
        }
        if cancel.is_cancelled() {
            group::kill(&mut child);
            let _ = child.wait();
            return Err(CANCELLED.into());
        }
        if cancel.is_paused() != stopped {
            stopped = cancel.is_paused();
            group::pause(&child, stopped);
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

/// The tool's process group: paused, resumed and killed as one.
mod group {
    use std::process::Child;

    #[cfg(unix)]
    fn signal(child: &Child, signal: libc::c_int) {
        let Ok(pid) = libc::pid_t::try_from(child.id()) else {
            return;
        };
        // SAFETY: a signal to the group the child leads (spawned with
        // `process_group(0)`); no memory is touched.
        unsafe { libc::kill(-pid, signal) };
    }

    pub fn pause(child: &Child, paused: bool) {
        #[cfg(unix)]
        signal(child, if paused { libc::SIGSTOP } else { libc::SIGCONT });
        #[cfg(not(unix))]
        let _ = (child, paused);
    }

    pub fn kill(child: &mut Child) {
        #[cfg(unix)]
        {
            // A stopped group has to go on to die.
            signal(child, libc::SIGCONT);
            signal(child, libc::SIGKILL);
        }
        let _ = child.kill();
    }
}
