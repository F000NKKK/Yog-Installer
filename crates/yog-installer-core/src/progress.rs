//! What a running operation tells: its progress, step by step, and the
//! flag that cancels it.

use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

/// Cancels an operation from another thread; its steps check it between chunks.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

/// Why a step stopped: the user cancelled, or something failed.
pub const CANCELLED: &str = "cancelled";

impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// `Err(CANCELLED)` once cancelled.
    pub fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err(CANCELLED.into())
        } else {
            Ok(())
        }
    }
}
