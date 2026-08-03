use pdf_viewer_standalone::infrastructure::pdf::models::{PageDisplayList, RenderObject};
use std::path::PathBuf;

fn find_resume_pdf() -> PathBuf {
    let exact = PathBuf::from(r"H:\myUserData\Documents\刘---20250514 - 副本 (2).pdf");
    if exact.exists() {
        return exact;
    }
    PathBuf::from(r"C:\Users\AREN\Documents\刘---20250514 - 副本 (2).pdf")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== VelloRenderer Standalone Verification ===");
    let pdf_path = find_resume_pdf();
    println!("Loading PDF: {:?}", pdf_path);

    let bytes = std::fs::read(&pdf_path)?;
    let doc = lopdf::Document::load_mem(&bytes)?;

    let (objects, text_runs, width, height) =
        pdf_viewer_standalone::infrastructure::pdf::pdf_read::path_resolver::resolve_paths(
            &doc, 0,
        )?;

    let display_list = PageDisplayList {
        page_index: 0,
        width,
        height,
        objects,
        text_runs,
    };

    let page_model =
        pdf_viewer_standalone::infrastructure::pdf::vector_engine::build_vector_page_model_from_display_list(
            &display_list,
        )?;

    println!("Total objects in page model: {}", page_model.objects.len());

    for (idx, obj) in page_model.objects.iter().enumerate() {
        match obj {
            RenderObject::Text(t) => {
                println!(
                    "  #{}: TEXT '{}' font='{}' mode={} color='{}' tx={:.2} ty={:.2}",
                    idx,
                    t.text,
                    t.font_name,
                    t.rendering_mode,
                    t.color,
                    t.tx,
                    t.ty
                );
            }
            RenderObject::Path(p) => {
                println!("  #{}: PATH fill={:?} color='{:?}' segs={:?}", idx, p.fill, p.fill_color, p.segments);
            }
            RenderObject::Image(img) => {
                println!("  #{}: IMAGE at x={:.1}, y={:.1}, w={:.1}, h={:.1}", idx, img.x, img.y, img.width, img.height);
            }
        }
    }

    let mut renderer = pdf_viewer_standalone::infrastructure::pdf::vello_renderer::VelloRenderer::new().await?;
    let target_w = (page_model.width * 2.0) as u32;
    let target_h = (page_model.height * 2.0) as u32;
    let png_bytes = renderer.render_objects_to_png(&page_model.objects, target_w, target_h, page_model.width, page_model.height)?;

    let out_path = PathBuf::from(r"C:\Users\Aren\.gemini\antigravity-ide\brain\01e21ebc-aa96-443c-8d9f-dc8ccb82be49\media__render_test.png");
    std::fs::write(&out_path, png_bytes)?;
    println!("Saved rendered PNG to artifact media: {:?}", out_path);

    Ok(())
}
