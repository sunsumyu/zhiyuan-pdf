use std::collections::HashMap;
use crate::models::LayoutInferenceResult;
use crate::state_manager::GLOBAL_PATCH_STATE;

pub fn calculate_reflow_displacements(
    v3_model: &LayoutInferenceResult
) -> HashMap<String, f32> {
    let mut displacements = HashMap::new();
    let state = match GLOBAL_PATCH_STATE.read() {
        Ok(s) => s,
        Err(_) => return displacements,
    };

    // 1. Collect flowable units
    struct ReflowUnit {
        id: String,
        top: f32,
        height: f32,
        new_height: f32,
    }

    let mut units = Vec::new();

    for region in &v3_model.regions {
        let original_height = region.bbox.bottom - region.bbox.top;
        let mut new_height = original_height;

        for para in &region.paragraphs {
            let patch_key = para.id.clone(); // para.id is often used as patch key directly or similar
            
            if let Some(snapshot) = state.get_paragraph_snapshot(&patch_key) {
                if !snapshot.lines.is_empty() {
                    let original_line_count = para.runs.len().max(1) as f32;
                    let avg_line_height = original_height / original_line_count;
                    let snapshot_height = snapshot.lines.len() as f32 * avg_line_height;
                    new_height = new_height.max(snapshot_height);
                }
            }
        }

        units.push(ReflowUnit {
            id: region.id.clone(),
            top: region.bbox.bottom, // higher bottom means "higher" on page
            height: original_height,
            new_height,
        });
    }

    // 2. Sort by Y descending (Top-to-Bottom in PDF Y-up)
    units.sort_by(|a, b| b.top.partial_cmp(&a.top).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Accumulate Displacement
    let mut cumulative_delta_y = 0.0;
    for unit in units {
        displacements.insert(unit.id, cumulative_delta_y);
        
        let delta_h = unit.new_height - unit.height;
        if delta_h.is_finite() {
            cumulative_delta_y -= delta_h; // Growth downwards subtracts from Y
        }
    }

    displacements
}
