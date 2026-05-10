use std::collections::HashSet;
use crate::persistence::models::{PersistableRegionPatch, RegionTextReflow, PersistableSavePlan};
use crate::document::page_region_context::PageRegionContextOutput;
use crate::persistence::state_manager::GLOBAL_PATCH_STATE;

pub fn collect_persistable_region_patches(
    context: &PageRegionContextOutput,
    page_index: u16,
) -> Vec<PersistableRegionPatch> {
    let mut patches = Vec::new();
    let state = match GLOBAL_PATCH_STATE.read() {
        Ok(s) => s,
        Err(_) => return patches,
    };

    // 1. Process Field Rows
    for row in &context.field_rows {
        for group in &row.groups {
            let patch_key = group.pair.id.clone();
            if let Some(new_text) = state.field_group_texts.get(&patch_key) {
                let snapshot = state.field_group_snapshots.get(&patch_key).cloned();
                patches.push(PersistableRegionPatch {
                    patch_key: patch_key.clone(),
                    page_index,
                    region_id: row.id.clone(),
                    original_text: format!("{}{}", group.pair.key_text, group.pair.value_text),
                    new_text: new_text.clone(),
                    source: "field-row".to_string(),
                    snapshot: serde_json::to_value(snapshot).ok(),
                    group_id: Some(group.id.clone()),
                    target_indices: group.object_indices.clone(),
                    displacement_y: Some(0.0),
                    wrap_width: None,
                    align: Some(crate::models::LayoutAlignment::Left),
                    char_spacing: group.pair.value_style.char_spacing,
                    horizontal_scaling: group.pair.value_style.scale_x,
                    ..Default::default()
                });
            }
        }
    }

    // 2. Process Paragraphs
    let mut process_para = |id: &String, text: &String, indices: &Vec<usize>, source_name: &str, wrap_width: Option<f32>, char_spacing: f32, scale_x: f32| {
        if let Some(new_text) = state.paragraph_texts.get(id) {
            let snapshot = state.paragraph_snapshots.get(id).cloned();
            patches.push(PersistableRegionPatch {
                patch_key: id.clone(),
                page_index,
                region_id: id.clone(),
                original_text: text.clone(),
                new_text: new_text.clone(),
                source: source_name.to_string(),
                snapshot: serde_json::to_value(snapshot).ok(),
                group_id: None,
                target_indices: indices.clone(),
                displacement_y: Some(0.0),
                wrap_width,
                align: Some(crate::models::LayoutAlignment::Left),
                char_spacing,
                horizontal_scaling: scale_x,
                ..Default::default()
            });
        }
    };

    for reg in &context.paragraph_regions {
        process_para(&reg.id, &reg.text, &reg.object_indices, "paragraph-region", Some(reg.wrap_width), reg.char_spacing, reg.scale_x);
    }
    for reg in &context.list_item_regions {
        process_para(&reg.id, &reg.text, &reg.object_indices, "list-item-region", Some(reg.wrap_width), reg.char_spacing, reg.scale_x);
    }

    patches
}

pub fn collect_legacy_text_reflows(
    context: &PageRegionContextOutput,
    page_index: u16,
    covered_field_row_object_ids: &HashSet<String>,
    covered_paragraph_object_ids: &HashSet<String>,
) -> Vec<RegionTextReflow> {
    let mut reflows = Vec::new();
    let state = match GLOBAL_PATCH_STATE.read() {
        Ok(s) => s,
        Err(_) => return reflows,
    };

    for obj in &context.text_objects {
        let _obj_key_field = format!("{}_{}", page_index, obj.object_indices.first().unwrap_or(&0)); // Check if this matches TS ID
        // Actually TS uses obj.id. In Rust model, NativeTextModel has an id field.
        let obj_id = &obj.id;

        if covered_field_row_object_ids.contains(obj_id) { continue; }
        if covered_paragraph_object_ids.contains(obj_id) { continue; }

        // Check for run-level patches
        let mut has_run_patch = false;
        if !obj.runs.is_empty() {
            for (i, _) in obj.runs.iter().enumerate() {
                let run_key = format!("{}_{}", obj_id, i);
                if state.patched_run_texts.contains_key(&run_key) {
                    has_run_patch = true;
                    break;
                }
            }
        }

        if has_run_patch {
            // In a real scenario, we'd compose the text here. 
            // For now, if we have a run patch, we'll try to find a combined text if possible, 
            // but the TS logic used a helper. 
            // We'll just look for a patched text for the whole object as a fallback.
            if let Some(new_text) = state.patched_texts.get(obj_id) {
                reflows.push(RegionTextReflow {
                    page_index,
                    target_indices: obj.object_indices.clone(),
                    new_text: new_text.clone(),
                    source: "run".to_string(),
                });
            }
        } else if let Some(new_text) = state.patched_texts.get(obj_id) {
            reflows.push(RegionTextReflow {
                page_index,
                target_indices: obj.object_indices.clone(),
                new_text: new_text.clone(),
                source: "object".to_string(),
            });
        }
    }

    reflows
}

pub fn build_persistable_save_plan(
    region_patches: Vec<PersistableRegionPatch>,
    legacy_text_reflows: Vec<RegionTextReflow>,
) -> PersistableSavePlan {
    let mut region_text_reflows = Vec::new();
    let mut covered_field_row_object_ids = HashSet::new();
    let mut covered_paragraph_object_ids = HashSet::new();

    for patch in &region_patches {
        if patch.source == "field-row" {
            for idx in &patch.target_indices {
                covered_field_row_object_ids.insert(format!("{}_{}", patch.page_index, idx));
            }
        } else {
            for idx in &patch.target_indices {
                covered_paragraph_object_ids.insert(format!("{}_{}", patch.page_index, idx));
            }
        }

        region_text_reflows.push(RegionTextReflow {
            page_index: patch.page_index,
            target_indices: patch.target_indices.clone(),
            new_text: patch.new_text.clone(),
            source: patch.source.clone(),
        });
    }

    // Merging logic (Simplified from TS)
    let mut effective_text_reflows = Vec::new();
    let mut suppressed_text_reflows = Vec::new();
    
    let mut region_owned_keys = HashSet::new();
    for reflow in &region_text_reflows {
        region_owned_keys.insert(get_reflow_key(reflow));
    }

    for reflow in region_text_reflows {
        effective_text_reflows.push(reflow);
    }

    for reflow in legacy_text_reflows {
        let key = get_reflow_key(&reflow);
        if region_owned_keys.contains(&key) {
            suppressed_text_reflows.push(reflow);
        } else {
            effective_text_reflows.push(reflow);
        }
    }

    PersistableSavePlan {
        region_patches,
        text_reflows: effective_text_reflows,
        suppressed_text_reflows,
        covered_field_row_object_ids,
        covered_paragraph_object_ids,
    }
}

fn get_reflow_key(reflow: &RegionTextReflow) -> String {
    format!("{}::{}", reflow.page_index, reflow.target_indices.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","))
}
