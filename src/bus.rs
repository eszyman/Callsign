//! Messages from the speech and key threads back to the window.

use std::path::PathBuf;

use crate::cues::CueKind;

pub enum WorkerEvent {
    Transcript {
        epoch: u64,
        result: Result<String, String>,
    },
    Injected {
        epoch: u64,
        cue: CueKind,
        headline: String,
        detail: String,
        result: Result<(), String>,
    },
    DownloadProgress {
        bytes: u64,
        total: Option<u64>,
    },
    DownloadFinished {
        result: Result<PathBuf, String>,
    },
}
