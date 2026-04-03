use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    sync::{Arc, Mutex},
};

use fontdue::{
    layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
    Font,
};
use glam::DVec2;
use rust_fontconfig::{FcFontCache, FcPattern};
use std::sync::LazyLock;

use types::{
    constants::el,
    error::RecadError,
    gr::{self, Effects},
};

pub static OSIFONT: &[u8] = include_bytes!("osifont-lgpl3fe.ttf");
static FONT_CACHE: LazyLock<FcFontCache> = LazyLock::new(FcFontCache::build);
static FONTS: LazyLock<Mutex<HashMap<String, Font>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

static LOADING_LOCKS: LazyLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[inline(always)]
fn get_face(effects: &Effects) -> String {
    if let Some(face) = &effects.font.face {
        face.to_string()
    } else {
        String::from(el::OSIFONT)
    }
}

pub fn load_font(face: &str) -> Result<(), RecadError> {
    if FONTS.lock().unwrap().contains_key(face) {
        return Ok(());
    }

    let font_lock = {
        let mut locks = LOADING_LOCKS.lock().unwrap();
        locks
            .entry(face.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };

    let _guard = font_lock.lock().unwrap();

    if FONTS.lock().unwrap().contains_key(face) {
        return Ok(());
    }

    let font_bytes = if face == el::OSIFONT {
        OSIFONT.to_vec()
    } else {
        let Some(result) = FONT_CACHE.query(
            &FcPattern {
                name: Some(face.to_string()),
                ..Default::default()
            },
            &mut Vec::new(),
        ) else {
            return Err(RecadError::Font(format!("Unable to load font: {face}")));
        };

        if let Some(source) = FONT_CACHE.get_font_by_id(&result.id) {
            match source {
                rust_fontconfig::FontSource::Disk(path) => {
                    let Ok(mut f) = File::open(&path.path) else {
                        return Err(RecadError::Font(format!("Unable to load font: {face}")));
                    };

                    let mut font = Vec::new();
                    f.read_to_end(&mut font)?;
                    font
                }
                rust_fontconfig::FontSource::Memory(font) => {
                    todo!("Memory font not implemented: {:?}", font);
                }
            }
        } else {
            return Err(RecadError::Font(format!(
                "unable to load font: {:?}",
                result.id
            )));
        }
    };

    let parsed_font = Font::from_bytes(font_bytes, fontdue::FontSettings::default())
        .map_err(|e| RecadError::Font(format!("Failed to parse font '{face}': {e}")))?;

    FONTS.lock().unwrap().insert(face.to_string(), parsed_font);

    Ok(())
}

pub fn dimension(text: &str, effects: &gr::Effects) -> Result<DVec2, RecadError> {
    let face = get_face(effects);
    load_font(&face)?;
    let last = FONTS.lock().unwrap();
    let fonts = &[last.get(&face).unwrap()];

    let mut layout = Layout::new(CoordinateSystem::PositiveYUp);

    let mut max_width = 0.0_f32;
    for line in text.split('\n') {
        layout.reset(&LayoutSettings {
            ..LayoutSettings::default()
        });

        let layout_text = format!("{}|", line);
        layout.append(fonts, &TextStyle::new(&layout_text, effects.font.size.0, 0));

        if let Some(g) = layout.glyphs().last() {
            if g.x > max_width {
                max_width = g.x;
            }
        }
    }

    Ok(DVec2::new(max_width as f64, effects.font.size.1 as f64))
}
