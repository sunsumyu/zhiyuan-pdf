use cosmic_text::FontSystem;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use swash::proxy::MetricsProxy;

lazy_static! {
    static ref FONT_SYSTEM: Mutex<FontSystem> = Mutex::new(FontSystem::new());
    static ref METRIC_CACHE: Mutex<HashMap<String, HashMap<char, f32>>> =
        Mutex::new(HashMap::new());
}
pub fn get_character_width_pdf_units(family_name: &str, ch: char) -> Option<f32> {
    // 1. Check cache
    if let Ok(cache) = METRIC_CACHE.lock() {
        if let Some(family_cache) = cache.get(family_name) {
            if let Some(&width) = family_cache.get(&ch) {
                return Some(width);
            }
        }
    }

    // 2. Fetch from system
    let mut fs = FONT_SYSTEM.lock().ok()?;

    // [V5 Sovereign Architecture] Strip PDF subset prefix (e.g. "ABCDEF+")
    let normalized_family = if family_name.len() > 7 && family_name.as_bytes()[6] == b'+' {
        &family_name[7..]
    } else {
        family_name
    };

    // [V3] Normalization: Many PDF PSNames (e.g. MicrosoftYaHei) lack spaces
    // that system font family names expect (e.g. Microsoft YaHei).
    let mut matching_names = vec![normalized_family.to_string()];
    if !normalized_family.contains(' ') {
        if normalized_family.starts_with("Microsoft") {
            matching_names.push(format!("Microsoft {}", &normalized_family[9..]));
        }
    }

    let mut resolved_width = None;
    for name in matching_names {
        let attrs = cosmic_text::Attrs::new().family(cosmic_text::Family::Name(&name));

        // Weuse a dummy buffer to trigger font matching
        let mut buffer = cosmic_text::Buffer::new(&mut fs, cosmic_text::Metrics::new(10.0, 12.0));
        buffer.set_text(
            &mut fs,
            &ch.to_string(),
            attrs,
            cosmic_text::Shaping::Advanced,
        );

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                if let Some(face) = fs.db().face(glyph.font_id) {
                    let data_vec;
                    let data = match &face.source {
                        cosmic_text::fontdb::Source::Binary(arc) => Some(arc.as_ref().as_ref()),
                        cosmic_text::fontdb::Source::File(path) => {
                            data_vec = std::fs::read(path).ok();
                            data_vec.as_deref()
                        }
                        _ => None,
                    };

                    if let Some(bytes) = data {
                        if let Some(font_ref) =
                            swash::FontRef::from_index(bytes, face.index as usize)
                        {
                            let metrics = MetricsProxy::from_font(&font_ref);
                            let upem = metrics.units_per_em() as f32;

                            let advance = font_ref.glyph_metrics(&[]).advance_width(glyph.glyph_id);

                            // Convert to PDF units (1/1000 of em)
                            let pdf_width = (advance / upem) * 1000.0;
                            resolved_width = Some(pdf_width);
                        }
                    }
                }
                if resolved_width.is_some() {
                    break;
                }
            }
            if resolved_width.is_some() {
                break;
            }
        }
        if resolved_width.is_some() {
            break;
        }
    }

    // 3. Update cache
    if let Some(w) = resolved_width {
        if let Ok(mut cache) = METRIC_CACHE.lock() {
            cache
                .entry(family_name.to_string())
                .or_insert_with(HashMap::new)
                .insert(ch, w);
        }
    }

    resolved_width
}
