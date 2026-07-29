use cosmic_text::{Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};

#[test]
fn test_cosmic_text_cjk_font_loading() {
    let mut font_system = FontSystem::new();
    font_system.db_mut().load_system_fonts();

    // Check if CJK fonts exist
    let cjk_fonts = vec![
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simsun.ttc",
    ];

    let mut loaded_any = false;
    for path in &cjk_fonts {
        if std::path::Path::new(path).exists() {
            if font_system.db_mut().load_font_file(path).is_ok() {
                println!("Loaded: {}", path);
                loaded_any = true;
            }
        }
    }

    // Print all available font families
    println!("=== Available font families ===");
    for face in font_system.db().faces() {
        println!("  {:?}", face.families);
    }
    println!("=== Total faces: {} ===", font_system.db().faces().count());

    // Try to shape Chinese text
    let metrics = Metrics::new(16.0, 20.0);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    let attrs = cosmic_text::Attrs::new().family(Family::Name("Microsoft YaHei"));
    buffer.set_text(&mut font_system, "简历", attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let mut found_glyph = false;
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            println!("Glyph: id={}, font_id={:?}, x={}, y={}", glyph.glyph_id, glyph.font_id, glyph.x, glyph.y);
            if let Some(face) = font_system.db().face(glyph.font_id) {
                println!("  Matched font: {:?}", face.families);
                found_glyph = true;
            }
        }
    }

    println!("Found glyph: {}", found_glyph);
    assert!(loaded_any, "No CJK fonts loaded");
}
