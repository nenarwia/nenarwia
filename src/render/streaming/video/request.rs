use std::path::Path;

use crate::render::context::state::RenderContext;
use crate::render::streaming::contracts::{
    VideoPipeline, VideoRoute, VideoWorkInput, VisibleRequest,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct VideoRequestPipeline;

impl VideoPipeline for VideoRequestPipeline {
    fn process(&self, ctx: &mut RenderContext, request: VisibleRequest) -> VideoRoute {
        let is_video = ctx
            .slot_paths
            .get(request.item_idx)
            .and_then(|path| path.live_path())
            .map(is_video_path)
            .unwrap_or(false);
        if is_video {
            return VideoRoute::ToDecode(VideoWorkInput {
                id: request.id,
                item_idx: request.item_idx,
            });
        }

        VideoRoute::Consumed
    }
}

fn is_video_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp4" | "mov" | "mkv" | "avi" | "webm"
    )
}
