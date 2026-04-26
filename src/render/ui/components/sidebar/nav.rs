use super::super::UiAction;

pub(in crate::render::ui::sidebar) const MAX_NAV_ITEMS: usize = 3;

const NAV_ITEMS: [SidebarNavItem; MAX_NAV_ITEMS] = [
    SidebarNavItem::AddFolder,
    SidebarNavItem::OpenCacheFolder,
    SidebarNavItem::ClearCanvas,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::render::ui::sidebar) enum SidebarNavItem {
    AddFolder,
    OpenCacheFolder,
    ClearCanvas,
}

impl SidebarNavItem {
    pub(in crate::render::ui::sidebar) const fn label(self) -> &'static str {
        match self {
            Self::AddFolder => "Add Folder",
            Self::OpenCacheFolder => "Open Cache Folder",
            Self::ClearCanvas => "Clear Canvas",
        }
    }

    pub(in crate::render::ui::sidebar) const fn action(self) -> UiAction {
        match self {
            Self::AddFolder => UiAction::OpenCanvasImportDialog,
            Self::OpenCacheFolder => UiAction::OpenCacheFolder,
            Self::ClearCanvas => UiAction::ClearCurrentCanvas,
        }
    }
}

pub(in crate::render::ui::sidebar) fn visible_nav_items() -> &'static [SidebarNavItem] {
    &NAV_ITEMS
}
