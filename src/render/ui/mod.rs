// UI code is grouped by responsibility:
// - components/: concrete UI widgets and dialogs
// - system/: shared text/texture helpers
// - passes/: full-screen compositing effects
#[path = "passes/backdrop.rs"]
pub(crate) mod backdrop;
#[path = "passes/backdrop_state.rs"]
mod backdrop_state;
#[path = "system/bind_groups.rs"]
mod bind_groups;
#[path = "components/canvas_context_menu/mod.rs"]
mod canvas_context_menu;
#[path = "components/chrome/mod.rs"]
mod chrome;
#[path = "components/codec_notice_runtime.rs"]
mod codec_notice_runtime;
#[path = "components/codec_notice_setup.rs"]
mod codec_notice_setup;
#[path = "components/codec_notice_state.rs"]
mod codec_notice_state;
#[path = "system/contracts.rs"]
mod contracts;
#[path = "system/font.rs"]
mod font;
#[path = "system/geometry.rs"]
mod geometry;
#[path = "system/notice_texture/mod.rs"]
mod notice_texture;
#[path = "system/raster.rs"]
mod raster;
#[path = "components/sidebar/mod.rs"]
mod sidebar;
#[path = "system/text.rs"]
mod text;
#[path = "system/tokens.rs"]
mod tokens;
#[path = "system/vertex.rs"]
mod vertex;
#[path = "components/wallpaper/mod.rs"]
mod wallpaper;
#[path = "components/wallpaper_preview/mod.rs"]
mod wallpaper_preview;
pub use backdrop_state::BackdropBlurUi;
pub use canvas_context_menu::CanvasContextMenuUi;
pub use chrome::{ChromeTabView, WindowChromeUi};
pub use codec_notice_state::CodecNoticeUi;
pub(crate) use contracts::UiRenderable;
pub(crate) use contracts::{UiClickable, UiUpdatable, UiUpdateCtx};
pub use sidebar::{SidebarSavedWallpaperItem, SidebarUi};
use tokens::*;
pub(crate) use vertex::UiVertex;
pub use wallpaper::WallpaperUi;
pub use wallpaper_preview::WallpaperPreviewUi;

pub(crate) const DEBUG_SLOT_TOGGLE_ENABLED: bool = cfg!(feature = "debug_slots_ui");

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiAction {
    Consume,
    NewTab,
    SelectTab(usize),
    CloseTab(usize),
    StartWindowDrag,
    ToggleWindowMaximize,
    ToggleWindowFullscreen,
    MinimizeWindow,
    CloseWindow,
    ToggleVsync,
    ToggleGraphicsBackend,
    ToggleDebugSlotBackdrop,
    OpenCanvasImportDialog,
    OpenCacheFolder,
    ClearCurrentCanvas,
    OpenEmptySlotFillDialog(u64),
    MoveSlotToTrash(u64),
    ShowInExplorer(u64),
    OpenWallpaperDialog,
    OpenSavedWallpaper(u64),
    WallpaperPreviewToggleBlur,
    WallpaperPreviewApply,
    WallpaperPreviewCancel,
}
