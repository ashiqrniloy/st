use std::{path::PathBuf, time::Duration};

use crate::editor::{BufferId, BufferSnapshot, BufferVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackgroundRequestId(pub u64);

#[derive(Debug, Clone)]
pub enum BackgroundTaskKind {
    CpuCountChars { snapshot: BufferSnapshot },
    IoReadFile { path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct BackgroundTaskRequest {
    pub request_id: BackgroundRequestId,
    pub buffer_id: BufferId,
    pub buffer_version: BufferVersion,
    pub timeout: Option<Duration>,
    pub kind: BackgroundTaskKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundTaskResult {
    CharCount(usize),
    FileBytes(usize),
    Cancelled,
    TimedOut,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundTaskResponse {
    pub request_id: BackgroundRequestId,
    pub buffer_id: BufferId,
    pub buffer_version: BufferVersion,
    pub result: BackgroundTaskResult,
}
