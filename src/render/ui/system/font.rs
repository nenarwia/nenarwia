use std::sync::{
    mpsc::{self, TryRecvError},
    Mutex, OnceLock,
};

use ab_glyph::{Font, FontArc, GlyphId};

const EMBEDDED_INTER_SEMIBOLD_TTF: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/Inter-SemiBold.ttf"
));

static TEXT_FALLBACK_FONTS: OnceLock<Vec<FontArc>> = OnceLock::new();
static TEXT_FALLBACK_PRELOAD: OnceLock<Mutex<TextFallbackFontPreload>> = OnceLock::new();

#[derive(Default)]
struct TextFallbackFontPreload {
    rx: Option<mpsc::Receiver<Vec<FontArc>>>,
}

pub(super) fn load_font() -> Option<FontArc> {
    static FONT: OnceLock<Option<FontArc>> = OnceLock::new();
    FONT.get_or_init(|| {
        if let Ok(font) = FontArc::try_from_slice(EMBEDDED_INTER_SEMIBOLD_TTF) {
            return Some(font);
        }

        for path in fallback_font_candidates() {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = FontArc::try_from_vec(bytes) {
                    return Some(font);
                }
            }
        }
        log::warn!("UI font not found (Inter + fallbacks); text overlays disabled.");
        None
    })
    .clone()
}

pub(super) fn font_for_char<'a>(primary: &'a FontArc, ch: char) -> (&'a FontArc, GlyphId, usize) {
    let primary_id = primary.glyph_id(ch);
    if primary_id.0 != 0 {
        return (primary, primary_id, 0);
    }

    let _ = poll_text_fallback_font_preload();
    let Some(fallback_fonts) = TEXT_FALLBACK_FONTS.get() else {
        schedule_text_fallback_font_preload();
        return (primary, primary_id, 0);
    };

    for (index, font) in fallback_fonts.iter().enumerate() {
        let id = font.glyph_id(ch);
        if id.0 != 0 {
            return (font, id, index + 1);
        }
    }

    (primary, primary_id, 0)
}

pub(super) fn schedule_text_fallback_font_preload() {
    if TEXT_FALLBACK_FONTS.get().is_some() {
        return;
    }

    let mut preload = text_fallback_preload_state()
        .lock()
        .expect("text fallback font preload lock");
    if preload.rx.is_some() || TEXT_FALLBACK_FONTS.get().is_some() {
        return;
    }

    let (tx, rx) = mpsc::channel();
    preload.rx = Some(rx);
    let spawn_result = std::thread::Builder::new()
        .name("ui-text-fallback-fonts".to_string())
        .spawn(move || {
            let _ = tx.send(load_text_fallback_fonts_from_disk());
        });

    if let Err(err) = spawn_result {
        preload.rx = None;
        let _ = TEXT_FALLBACK_FONTS.set(Vec::new());
        log::warn!("Failed to start text fallback font preload worker: {err}");
    }
}

pub(super) fn poll_text_fallback_font_preload() -> bool {
    if TEXT_FALLBACK_FONTS.get().is_some() {
        return false;
    }

    let mut preload = text_fallback_preload_state()
        .lock()
        .expect("text fallback font preload lock");
    let Some(rx) = preload.rx.as_ref() else {
        return false;
    };

    let fonts = match rx.try_recv() {
        Ok(fonts) => Some(fonts),
        Err(TryRecvError::Disconnected) => Some(Vec::new()),
        Err(TryRecvError::Empty) => None,
    };

    let Some(fonts) = fonts else {
        return false;
    };
    preload.rx = None;
    let has_fonts = !fonts.is_empty();
    let _ = TEXT_FALLBACK_FONTS.set(fonts);
    has_fonts
}

fn text_fallback_preload_state() -> &'static Mutex<TextFallbackFontPreload> {
    TEXT_FALLBACK_PRELOAD.get_or_init(|| Mutex::new(TextFallbackFontPreload::default()))
}

fn load_text_fallback_fonts_from_disk() -> Vec<FontArc> {
    text_fallback_font_candidates()
        .into_iter()
        .filter_map(|path| {
            std::fs::read(path)
                .ok()
                .and_then(|bytes| FontArc::try_from_vec(bytes).ok())
        })
        .collect()
}

fn fallback_font_candidates() -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    {
        // Keep deterministic fallback order if embedded Inter fails to load.
        vec![
            "C:\\Windows\\Fonts\\arialbd.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
        ]
    }
    #[cfg(target_os = "linux")]
    {
        return vec![
            "/usr/share/fonts/truetype/inter/Inter-SemiBold.ttf",
            "/usr/share/fonts/truetype/inter/Inter-Medium.ttf",
            "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
            "/usr/share/fonts/truetype/msttcorefonts/Arial.ttf",
            "/usr/share/fonts/truetype/msttcorefonts/arial.ttf",
        ];
    }
    #[cfg(target_os = "macos")]
    {
        return vec![
            "/Library/Fonts/Inter-SemiBold.ttf",
            "/Library/Fonts/Inter-Medium.ttf",
            "/Library/Fonts/Inter-Regular.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ];
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        vec![]
    }
}

fn text_fallback_font_candidates() -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    {
        vec![
            "C:\\Windows\\Fonts\\seguisym.ttf",
            "C:\\Windows\\Fonts\\seguiemj.ttf",
            "C:\\Windows\\Fonts\\SegoeIcons.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arialuni.ttf",
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
            "/usr/share/fonts/truetype/noto/NotoEmoji-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
    }
    #[cfg(target_os = "macos")]
    {
        vec![
            "/System/Library/Fonts/Apple Color Emoji.ttc",
            "/System/Library/Fonts/Supplemental/Apple Symbols.ttf",
            "/System/Library/Fonts/SFNS.ttf",
        ]
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use ab_glyph::Font;

    use super::{
        font_for_char, load_font, poll_text_fallback_font_preload,
        schedule_text_fallback_font_preload, TEXT_FALLBACK_FONTS,
    };

    #[test]
    fn font_for_char_keeps_regular_text_on_primary_font() {
        let font = load_font().expect("embedded UI font should load");

        let (_, glyph_id, font_index) = font_for_char(&font, 'A');

        assert_ne!(glyph_id.0, 0);
        assert_eq!(font_index, 0);
    }

    #[test]
    fn font_for_char_uses_fallback_for_common_emoji_when_available() {
        let font = load_font().expect("embedded UI font should load");
        wait_for_text_fallback_font_preload();

        let (_, glyph_id, _) = font_for_char(&font, '\u{263A}');
        let ready_fonts_support_emoji = TEXT_FALLBACK_FONTS
            .get()
            .map(|fonts| fonts.iter().any(|font| font.glyph_id('\u{263A}').0 != 0))
            .unwrap_or(false);

        assert_eq!(
            glyph_id.0 != 0,
            font.glyph_id('\u{263A}').0 != 0 || ready_fonts_support_emoji
        );
    }

    fn wait_for_text_fallback_font_preload() {
        schedule_text_fallback_font_preload();
        let deadline = Instant::now() + Duration::from_secs(2);
        while TEXT_FALLBACK_FONTS.get().is_none() && Instant::now() < deadline {
            let _ = poll_text_fallback_font_preload();
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = poll_text_fallback_font_preload();
    }
}
