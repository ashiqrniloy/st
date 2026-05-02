use crate::editor::BufferVersion;

use super::request::BackgroundTaskResponse;

pub fn should_discard_stale_response(
    response: &BackgroundTaskResponse,
    current: BufferVersion,
) -> bool {
    response.buffer_version != current
}
