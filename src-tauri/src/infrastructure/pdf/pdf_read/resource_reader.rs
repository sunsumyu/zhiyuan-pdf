use lopdf::Document;
use std::collections::HashMap;
pub type FlatResources = HashMap<Vec<u8>, HashMap<Vec<u8>, lopdf::ObjectId>>;

pub fn read_resources(doc: &Document, page_id: lopdf::ObjectId) -> FlatResources {
    let mut flat: FlatResources = HashMap::new();
    let mut curr_id = page_id;
    let mut visited = std::collections::HashSet::new();

    while let Ok(dict) = doc.get_dictionary(curr_id) {
        if let Ok(res_obj) = dict.get(b"Resources") {
            if let Ok(res_dict) = res_obj
                .as_dict()
                .or_else(|_| res_obj.as_reference().and_then(|r| doc.get_dictionary(r)))
            {
                for (cat_key, cat_val) in res_dict.iter() {
                    let cat_map = flat.entry(cat_key.clone()).or_insert_with(HashMap::new);
                    if let Ok(sub_dict) = cat_val
                        .as_dict()
                        .or_else(|_| cat_val.as_reference().and_then(|r| doc.get_dictionary(r)))
                    {
                        for (res_name, res_val) in sub_dict.iter() {
                            if let Ok(id) = res_val.as_reference() {
                                cat_map.entry(res_name.clone()).or_insert(id);
                            }
                        }
                    }
                }
            }
        }
        if let Ok(parent_ref) = dict.get(b"Parent").and_then(|o| o.as_reference()) {
            if visited.contains(&parent_ref) {
                break;
            }
            visited.insert(parent_ref);
            curr_id = parent_ref;
        } else {
            break;
        }
    }
    flat
}

pub fn find_xobject_by_name(
    doc: &Document,
    flat_resources: &FlatResources,
    name: &[u8],
) -> Option<lopdf::ObjectId> {
    if let Some(xobjects) = flat_resources.get(b"XObject" as &[u8]) {
        if let Some(&id) = xobjects.get(name) {
            return Some(id);
        }
    }
    // Fallback: search all other pages' resources for this XObject name
    for (_, page_obj_id) in doc.get_pages() {
        let other_resources = read_resources(doc, page_obj_id);
        if let Some(xobjects) = other_resources.get(b"XObject" as &[u8]) {
            if let Some(&id) = xobjects.get(name) {
                return Some(id);
            }
        }
    }
    None
}
