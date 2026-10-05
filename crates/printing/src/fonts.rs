//! The bundled fonts. Printouts never use the computer's own fonts, so a receipt
//! looks the same, dot for dot, on Windows, macOS and Linux.

use std::sync::{Mutex, MutexGuard};

use cosmic_text::{Fallback, FontSystem, SwashCache, fontdb};

/// The family every printout uses (ADR-012).
pub(crate) const FAMILY: &str = "Noto Sans";

/// Noto Sans has no taka sign (৳), so a tiny cut of Noto Sans Bengali supplies it.
const CURRENCY_SIGNS_FAMILY: &str = "Noto Sans Bengali";

const FONT_FILES: [&[u8]; 4] = [
    include_bytes!("../fonts/NotoSans-Regular.ttf"),
    include_bytes!("../fonts/NotoSans-Bold.ttf"),
    include_bytes!("../fonts/NotoSansBengali-Regular-CurrencySigns.ttf"),
    include_bytes!("../fonts/NotoSansBengali-Bold-CurrencySigns.ttf"),
];

/// The loaded fonts and the cache of drawn letters. Load once at start-up and share it.
/// It can be shared between threads; one printout is drawn at a time.
pub struct Fonts {
    tools: Mutex<FontTools>,
}

/// What drawing text needs. `cosmic-text` needs to change these while it works,
/// so they sit behind the mutex in `Fonts`.
pub(crate) struct FontTools {
    pub(crate) system: FontSystem,
    /// Each letter shape drawn so far, so it's drawn only once.
    pub(crate) swash: SwashCache,
}

/// Loads the bundled fonts. Call once at start-up; it takes a moment.
pub fn load_fonts() -> Fonts {
    let mut database = fontdb::Database::new();
    for font in FONT_FILES {
        database.load_font_data(font.to_vec());
    }
    database.set_sans_serif_family(FAMILY);
    let system =
        FontSystem::new_with_locale_and_db_and_fallback("en-US".into(), database, OurFallback);
    Fonts {
        tools: Mutex::new(FontTools {
            system,
            swash: SwashCache::new(),
        }),
    }
}

impl Fonts {
    /// Takes the fonts for drawing one printout.
    pub(crate) fn lock(&self) -> MutexGuard<'_, FontTools> {
        // A panic while drawing leaves only a cache behind, which is still fine to use.
        self.tools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Which font to try when Noto Sans lacks a letter. `cosmic-text` normally picks
/// fallbacks by operating system; ours is the same everywhere, so the picture is too.
struct OurFallback;

impl Fallback for OurFallback {
    fn common_fallback(&self) -> &[&'static str] {
        &[CURRENCY_SIGNS_FAMILY]
    }

    fn forbidden_fallback(&self) -> &[&'static str] {
        &[]
    }

    fn script_fallback(&self, _script: unicode_script::Script, _locale: &str) -> &[&'static str] {
        &[]
    }
}
