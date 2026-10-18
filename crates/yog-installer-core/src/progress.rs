//! What a running operation tells: its progress, step by step, and the
//! flag that cancels it.

use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// One thing that happened, for the UI's progress (as VS Installer shows it).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    /// The plan starts: how many packages, how many bytes to fetch (as far as known).
    Started {
        packages: usize,
        bytes: u64,
    },
    /// Bytes of a package's download arrived.
    Downloading {
        component: String,
        done: u64,
        total: Option<u64>,
    },
    /// Package `index` (from 1) of the plan is being installed.
    Installing {
        component: String,
        index: usize,
        of: usize,
    },
    /// A line a tool printed while installing.
    Output {
        component: String,
        line: String,
    },
    /// A package is done.
    Installed {
        component: String,
        version: String,
    },
    Removed {
        component: String,
    },
    /// The plan ended: every package, or stopped at an error / a cancel.
    Finished {
        ok: bool,
        cancelled: bool,
        error: Option<String>,
    },
}

/// Who hears the events.
pub trait Sink: Send + Sync {
    fn event(&self, event: Event);
}

impl<F: Fn(Event) + Send + Sync> Sink for F {
    fn event(&self, event: Event) {
        self(event)
    }
}

/// Pauses, resumes and cancels an operation from another thread; its steps
/// check it between chunks (and wait there while it's paused).
#[derive(Debug, Clone, Default)]
pub struct Cancel {
    cancelled: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

/// Why a step stopped: the user cancelled, or something failed.
pub const CANCELLED: &str = "cancelled";
/// How often a paused step looks whether it may go on.
const PAUSE_POLL: Duration = Duration::from_millis(100);

impl Cancel {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    /// Waits while paused; then `Err(CANCELLED)` once cancelled.
    pub fn check(&self) -> Result<(), String> {
        while self.paused.load(Ordering::Relaxed) && !self.is_cancelled() {
            std::thread::sleep(PAUSE_POLL);
        }
        if self.is_cancelled() {
            Err(CANCELLED.into())
        } else {
            Ok(())
        }
    }
}
