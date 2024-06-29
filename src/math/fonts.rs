use std::{collections::HashMap, fs::File, io::Read, sync::Mutex};

use fontdue::{layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle}, Font};
use ndarray::{arr2, Array2};
use lazy_static::lazy_static;
use rust_fontconfig::{FcFontCache, FcPattern};

use crate::{gr, Error};


static OSIFONT: &[u8] = include_bytes!("osifont-lgpl3fe.ttf");

pub fn dimension(text: &str, effects: &gr::Effects) -> Result<Array2<f32>, Error> {
    lazy_static! {
        static ref FONT_CACHE: FcFontCache = FcFontCache::build();
        static ref FONTS: Mutex<HashMap<String, Font>> = Mutex::new(HashMap::new());
    }

    let mut last = FONTS.lock().unwrap();
    let face = if let Some(face) = &effects.font.face {
        face.to_string()
    } else {
        String::from("osifont")
    };

    if !last.contains_key(&face) {
        let font = if face == "osifont" {
            OSIFONT.to_vec()
        } else {
            let Some(result) = FONT_CACHE.query(&FcPattern {
                name: Some(String::from(&face)),
                ..Default::default()
            }) else {
                return Err(Error(String::from("font-error"), format!("Unable to load font: {face}")));
            };

            let result = result.path.to_string();
            let Ok(mut f) = File::open(result) else {
                return Err(Error(String::from("font-error"), format!("Unable to load font: {face}")));
            };

            let mut font = Vec::new();
            f.read_to_end(&mut font)?;
            font
        };

        last.insert(
            face.clone(),
            Font::from_bytes(font, fontdue::FontSettings::default()).unwrap(),
        );
    }

    let fonts = &[last.get(&face).unwrap()];
    let mut layout = Layout::new(CoordinateSystem::PositiveYUp);
    layout.reset(&LayoutSettings {
        ..LayoutSettings::default()
    });
    layout.append(
        fonts,
        &TextStyle::new(
            text,
            effects.font.size.0 * 1.333333,
            0,
        ),
    );
    let width: usize = layout.glyphs().iter().map(|g| g.width).sum();

    Ok(arr2(&[[
        width as f32,
        effects.font.size.0 * 1.333333,
    ]]))
}
