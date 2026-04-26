#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FramePacingMode {
    #[default]
    VSync,
    Unlimited,
}

impl FramePacingMode {
    pub fn is_vsync(self) -> bool {
        matches!(self, Self::VSync)
    }

    pub fn is_unlimited(self) -> bool {
        matches!(self, Self::Unlimited)
    }

    pub fn toggled_vsync(self) -> Self {
        if self.is_vsync() {
            Self::Unlimited
        } else {
            Self::VSync
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::VSync => "vsync",
            Self::Unlimited => "unlimited (500 fps cap)",
        }
    }
}
