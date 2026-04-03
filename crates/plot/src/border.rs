use crate::{
    theme::{Style, Theme},
    Paint, Plotter,
};
use types::gr::{Effects, Font, Justify, PaperSize, Pos, Pt, Rect, TitleBlock};

const BORDER_RASTER: f64 = 50.0;
const MARGIN: f64 = 10.0;

pub fn draw_border(
    plotter: &mut impl Plotter,
    paper: &PaperSize,
    tb: &TitleBlock,
    path: &str,
    sheet: &str,
    theme: &Theme,
) {
    let (width, height) = paper_dimensions(paper);

    let border_paint = Paint {
        color: theme.color(None, Style::Border),
        fill: None,
        width: theme.width(0.0, Style::Border),
        ..Default::default()
    };

    let inner_start = Pt {
        x: MARGIN,
        y: MARGIN,
    };
    let inner_end = Pt {
        x: width - MARGIN,
        y: height - MARGIN,
    };

    //Draw Main Border
    plotter.rect(
        Rect {
            start: inner_start,
            end: Pt {
                x: inner_end.x - inner_start.x,
                y: inner_end.y - inner_start.y,
            },
        },
        border_paint.clone(),
    );

    // Draw Horizontal Grid (Numbers)
    let cols = (width / BORDER_RASTER) as i32;
    for i in 0..cols {
        let x = (i as f64) * BORDER_RASTER;

        if x >= MARGIN && x <= width - MARGIN {
            for y in [MARGIN, height - MARGIN] {
                let offset = if y == MARGIN { 2.0 } else { -2.0 };
                plotter.move_to(Pt { x, y });
                plotter.line_to(Pt { x, y: y + offset });
                plotter.stroke(border_paint.clone());
            }
        }

        let text_x = x + (BORDER_RASTER / 2.0);
        if text_x < width - MARGIN {
            let label = (i + 1).to_string();
            draw_border_text(
                plotter,
                &label,
                Pt {
                    x: text_x,
                    y: MARGIN / 2.0,
                },
                theme,
            );
            draw_border_text(
                plotter,
                &label,
                Pt {
                    x: text_x,
                    y: height - (MARGIN / 2.0),
                },
                theme,
            );
        }
    }

    // Draw Vertical Grid (Letters)
    let rows = (height / BORDER_RASTER) as i32;

    for i in 0..rows {
        let y = (i as f64) * BORDER_RASTER;

        if y >= MARGIN && y <= height - MARGIN {
            for x in [MARGIN, width - MARGIN] {
                let offset = if x == MARGIN { 2.0 } else { -2.0 };
                plotter.move_to(Pt { x, y });
                plotter.line_to(Pt { x: x + offset, y });
                plotter.stroke(border_paint.clone());
            }
        }

        let text_y = y + (BORDER_RASTER / 2.0);
        if text_y < height - MARGIN && i < 26 {
            let label = ((b'A' + i as u8) as char).to_string();

            draw_border_text(
                plotter,
                &label,
                Pt {
                    x: MARGIN / 2.0,
                    y: text_y,
                },
                theme,
            );
            draw_border_text(
                plotter,
                &label,
                Pt {
                    x: width - (MARGIN / 2.0),
                    y: text_y,
                },
                theme,
            );
        }
    }

    draw_title_block(
        plotter,
        width,
        height,
        tb,
        path,
        sheet,
        theme,
        &border_paint,
        paper,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_title_block(
    plotter: &mut impl Plotter,
    sheet_w: f64,
    sheet_h: f64,
    tb: &TitleBlock,
    path: &str,
    sheet: &str,
    theme: &Theme,
    paint: &Paint,
    paper: &PaperSize,
) {
    let margin = MARGIN;
    let bottom = sheet_h - margin;
    let right = sheet_w - margin;

    // Define Layout Constants
    let tb_width = 110.0;
    let tb_height = 33.0;
    let start_x = right - tb_width;
    let start_y = bottom - tb_height;

    // Outer Box
    plotter.rect(
        Rect {
            start: Pt {
                x: start_x,
                y: start_y,
            },
            end: Pt {
                x: tb_width,
                y: tb_height,
            },
        },
        paint.clone(),
    );

    // --- Lines ---
    let line_y_sheet = bottom - 8.0;
    let line_y_title = bottom - 16.0;
    let line_y_path = bottom - 5.0;
    let rev_x = right - 45.0;
    let date_x = right - 22.0;

    // Horizontal Separators
    for y in [line_y_sheet, line_y_title, line_y_path] {
        plotter.move_to(Pt { x: start_x, y });
        plotter.line_to(Pt { x: right, y });
        plotter.stroke(paint.clone());
    }

    // Vertical Separator (Rev)
    plotter.move_to(Pt {
        x: rev_x,
        y: line_y_sheet,
    });
    plotter.line_to(Pt {
        x: rev_x,
        y: line_y_path,
    });
    plotter.stroke(paint.clone());

    // Vertical Separator (Date)
    plotter.move_to(Pt {
        x: date_x,
        y: line_y_sheet,
    });
    plotter.line_to(Pt {
        x: date_x,
        y: line_y_path,
    });
    plotter.stroke(paint.clone());

    // --- Text Content ---

    // Helpers for text positioning
    let padding = 1.2;

    // sheet
    // file

    // Title
    if let Some(title) = &tb.title {
        draw_tb_text(
            plotter,
            title,
            Pt {
                x: start_x + padding,
                y: line_y_title + 5.0,
            },
            1.5,
            theme,
        );
    }

    // Company
    if let Some(company) = &tb.company_name {
        draw_tb_text(
            plotter,
            company,
            Pt {
                x: start_x + padding,
                y: line_y_title - 2.0,
            },
            1.5,
            theme,
        );
    }

    // Revision

    // Sheet Size
    // Using format! debug or display is fine here if PaperSize implements Display
    draw_tb_text(
        plotter,
        &format!("Size: {}", paper),
        Pt {
            x: start_x + padding,
            y: bottom - 6.0,
        },
        1.5,
        theme,
    );

    // Revision
    if let Some(rev) = &tb.revision {
        draw_tb_text(
            plotter,
            &format!("Rev: {}", rev),
            Pt {
                x: rev_x + padding,
                y: bottom - 6.0,
            },
            1.5,
            theme,
        );
    }

    if let Some(date) = &tb.date {
        draw_tb_text(
            plotter,
            &format!("Date: {}", date),
            Pt {
                x: date_x + padding,
                y: bottom - 6.0,
            },
            1.5,
            theme,
        );
    }

    // sheet
    draw_tb_text(
        plotter,
        &format!("Sheet: {}", sheet),
        Pt {
            x: start_x + padding,
            y: bottom - 3.0,
        },
        1.5,
        theme,
    );

    //path
    draw_tb_text(
        plotter,
        &format!("Path: {}", path),
        Pt {
            x: start_x + padding,
            y: bottom - 1.2,
        },
        1.5,
        theme,
    );

    // Comments
    // Ensure we handle the index correctly regardless of map/vec type
    for (idx, txt) in &tb.comment {
        // Assuming idx is 1-based (KiCad standard).
        // If 0-based, logic needs adjustment.
        let idx_u = *idx as usize;
        if idx_u > 0 && idx_u <= 4 {
            let offset_y = line_y_title - padding - (3.0 * idx_u as f64 + 1.0);
            draw_tb_text(
                plotter,
                txt,
                Pt {
                    x: start_x + padding,
                    y: offset_y,
                },
                1.5,
                theme,
            );
        }
    }
}

fn draw_border_text(plotter: &mut impl Plotter, text: &str, pt: Pt, theme: &Theme) {
    let effects = Effects {
        font: Font {
            face: Some(theme.face()),
            size: theme.font_size((0.0, 0.0), Style::TextSheet),
            color: Some(theme.color(None, Style::TextSheet)),
            ..Default::default()
        },
        justify: vec![Justify::Center, Justify::Center],
        ..Default::default()
    };

    plotter.text(
        text,
        Pos {
            x: pt.x,
            y: pt.y,
            angle: 0.0,
        },
        effects,
    );
}

fn draw_tb_text(plotter: &mut impl Plotter, text: &str, pt: Pt, size: f32, theme: &Theme) {
    let effects = Effects {
        font: Font {
            face: Some(theme.face()),
            size: theme.font_size((size, size), Style::TextSheet),
            color: Some(theme.color(None, Style::TextSheet)),
            ..Default::default()
        },
        justify: vec![Justify::Left, Justify::Center], // Align left usually
        ..Default::default()
    };

    plotter.text(
        text,
        Pos {
            x: pt.x,
            y: pt.y,
            angle: 0.0,
        },
        effects,
    );
}

fn paper_dimensions(size: &PaperSize) -> (f64, f64) {
    // Correct Rust idiom: Match on enum variants, not string representation
    match size {
        PaperSize::A4 => (297.0, 210.0),
        PaperSize::A3 => (420.0, 297.0),
        PaperSize::A2 => (594.0, 420.0),
        PaperSize::A1 => (841.0, 594.0),
        PaperSize::A0 => (1189.0, 841.0),
        // Handle UserDefined or others
        _ => (297.0, 210.0), // Default to A4
    }
}
