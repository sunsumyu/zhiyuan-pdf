use serde::{Deserialize, Serialize};

const DETAIL_TILE_REUSE_MARGIN: f32 = 96.0;
const MAX_RECENT_BASE_LAYERS: usize = 4;
const MAX_RECENT_DETAIL_TILES: usize = 8;
const MAX_STORED_BASE_FRAME_KEYS: usize = 4;
const MAX_STORED_DETAIL_FRAME_KEYS: usize = 12;
const BASE_LAYER_PREVIEW_REUSE_RATIO: f32 = 0.18;
const BASE_LAYER_SETTLED_REUSE_RATIO: f32 = 0.05;
const DETAIL_TILE_PREVIEW_REUSE_RATIO: f32 = 0.12;
const DETAIL_TILE_SETTLED_REUSE_RATIO: f32 = 0.035;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BaseLayerCacheEntry {
    pub key: String,
    pub cache_zoom: f32,
    #[serde(default)]
    pub scene_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DetailTileCacheEntry {
    pub key: String,
    pub cache_zoom: f32,
    #[serde(default)]
    pub scene_key: String,
    pub tile_left: f32,
    pub tile_top: f32,
    pub tile_width: f32,
    pub tile_height: f32,
}

#[derive(Debug, Clone, Default)]
pub struct HostPresentState {
    pub active_base_layer: Option<BaseLayerCacheEntry>,
    pub active_detail_tile: Option<DetailTileCacheEntry>,
    pub recent_base_layers: Vec<BaseLayerCacheEntry>,
    pub recent_detail_tiles: Vec<DetailTileCacheEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct HostFrameCacheState {
    pub stored_base_frame_keys: Vec<String>,
    pub stored_detail_frame_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrameCacheStoreResult {
    pub evicted_keys: Vec<String>,
}

pub fn build_base_cache_key(
    session_path: &str,
    session_page: u16,
    scene_key: &str,
    cache_zoom: f32,
    device_pixel_ratio: f32,
) -> String {
    format!(
        "{}|{}|{}|base|{:.4}|{:.4}",
        session_path, session_page, scene_key, cache_zoom, device_pixel_ratio
    )
}

pub fn build_detail_cache_key(
    session_path: &str,
    session_page: u16,
    scene_key: &str,
    cache_zoom: f32,
    device_pixel_ratio: f32,
    tile_left: f32,
    tile_top: f32,
    tile_width: f32,
    tile_height: f32,
) -> String {
    format!(
        "{}|{}|{}|detail|{:.4}|{:.4}|{}|{}|{}|{}",
        session_path,
        session_page,
        scene_key,
        cache_zoom,
        device_pixel_ratio,
        tile_left.round(),
        tile_top.round(),
        tile_width.round(),
        tile_height.round()
    )
}

pub fn find_reusable_base_layer(
    state: &HostPresentState,
    scene_key: &str,
    cache_zoom: f32,
    preview_settled: bool,
) -> Option<BaseLayerCacheEntry> {
    let exact = state
        .active_base_layer
        .as_ref()
        .filter(|layer| {
            layer.scene_key == scene_key && cache_zoom_matches(layer.cache_zoom, cache_zoom)
        })
        .cloned()
        .or_else(|| {
            state
                .recent_base_layers
                .iter()
                .find(|layer| {
                    layer.scene_key == scene_key && cache_zoom_matches(layer.cache_zoom, cache_zoom)
                })
                .cloned()
        });
    if exact.is_some() {
        return exact;
    }

    let allowed_ratio = if preview_settled {
        BASE_LAYER_SETTLED_REUSE_RATIO
    } else {
        BASE_LAYER_PREVIEW_REUSE_RATIO
    };

    best_matching_base_layer(
        &state.active_base_layer,
        &state.recent_base_layers,
        scene_key,
        cache_zoom,
        allowed_ratio,
    )
}

pub fn find_reusable_detail_tile(
    state: &HostPresentState,
    scene_key: &str,
    cache_zoom: f32,
    visible_left: f32,
    visible_top: f32,
    visible_right: f32,
    visible_bottom: f32,
    preview_settled: bool,
) -> Option<DetailTileCacheEntry> {
    let exact = state
        .active_detail_tile
        .as_ref()
        .filter(|tile| {
            tile.scene_key == scene_key
                && detail_tile_covers_viewport(
                    tile,
                    cache_zoom,
                    visible_left,
                    visible_top,
                    visible_right,
                    visible_bottom,
                )
        })
        .cloned()
        .or_else(|| {
            state
                .recent_detail_tiles
                .iter()
                .find(|tile| {
                    tile.scene_key == scene_key
                        && detail_tile_covers_viewport(
                            tile,
                            cache_zoom,
                            visible_left,
                            visible_top,
                            visible_right,
                            visible_bottom,
                        )
                })
                .cloned()
        });
    if exact.is_some() {
        return exact;
    }

    let allowed_ratio = if preview_settled {
        DETAIL_TILE_SETTLED_REUSE_RATIO
    } else {
        DETAIL_TILE_PREVIEW_REUSE_RATIO
    };

    best_matching_detail_tile(
        &state.active_detail_tile,
        &state.recent_detail_tiles,
        scene_key,
        cache_zoom,
        visible_left,
        visible_top,
        visible_right,
        visible_bottom,
        allowed_ratio,
    )
}

pub fn remember_base_layer(state: &mut HostPresentState, layer: BaseLayerCacheEntry) {
    state.active_base_layer = Some(layer.clone());
    push_recent_base_layer(&mut state.recent_base_layers, layer);
}

pub fn remember_detail_tile(state: &mut HostPresentState, tile: DetailTileCacheEntry) {
    state.active_detail_tile = Some(tile.clone());
    push_recent_detail_tile(&mut state.recent_detail_tiles, tile);
}

pub fn clear_detail_tiles(state: &mut HostPresentState) {
    state.active_detail_tile = None;
    state.recent_detail_tiles.clear();
}

pub fn touch_frame_cache_key(
    state: &mut HostFrameCacheState,
    is_detail: bool,
    key: &str,
) -> bool {
    if key.is_empty() {
        return false;
    }

    let keys = if is_detail {
        &mut state.stored_detail_frame_keys
    } else {
        &mut state.stored_base_frame_keys
    };

    if let Some(position) = keys.iter().position(|existing| existing == key) {
        let existing = keys.remove(position);
        keys.insert(0, existing);
        true
    } else {
        false
    }
}

pub fn store_frame_cache_key(
    state: &mut HostFrameCacheState,
    is_detail: bool,
    key: String,
) -> FrameCacheStoreResult {
    if key.is_empty() {
        return FrameCacheStoreResult::default();
    }

    let (keys, max_keys) = if is_detail {
        (
            &mut state.stored_detail_frame_keys,
            MAX_STORED_DETAIL_FRAME_KEYS,
        )
    } else {
        (
            &mut state.stored_base_frame_keys,
            MAX_STORED_BASE_FRAME_KEYS,
        )
    };

    keys.retain(|existing| existing != &key);
    keys.insert(0, key);

    let mut evicted_keys = Vec::new();
    while keys.len() > max_keys {
        if let Some(evicted) = keys.pop() {
            evicted_keys.push(evicted);
        } else {
            break;
        }
    }

    FrameCacheStoreResult { evicted_keys }
}

pub fn clear_frame_cache_keys(state: &mut HostFrameCacheState) -> Vec<String> {
    let mut evicted_keys = Vec::new();
    evicted_keys.append(&mut state.stored_base_frame_keys);
    evicted_keys.append(&mut state.stored_detail_frame_keys);
    evicted_keys
}

fn detail_tile_covers_viewport(
    tile: &DetailTileCacheEntry,
    cache_zoom: f32,
    visible_left: f32,
    visible_top: f32,
    visible_right: f32,
    visible_bottom: f32,
) -> bool {
    if !cache_zoom_matches(tile.cache_zoom, cache_zoom) {
        return false;
    }

    let tile_right = tile.tile_left + tile.tile_width;
    let tile_bottom = tile.tile_top + tile.tile_height;
    visible_left >= tile.tile_left + DETAIL_TILE_REUSE_MARGIN
        && visible_top >= tile.tile_top + DETAIL_TILE_REUSE_MARGIN
        && visible_right <= tile_right - DETAIL_TILE_REUSE_MARGIN
        && visible_bottom <= tile_bottom - DETAIL_TILE_REUSE_MARGIN
}

fn cache_zoom_matches(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.0001
}

fn cache_zoom_ratio_delta(left: f32, right: f32) -> f32 {
    let safe_left = left.max(0.0001);
    let safe_right = right.max(0.0001);
    ((safe_left / safe_right) - 1.0).abs()
}

fn best_matching_base_layer(
    active: &Option<BaseLayerCacheEntry>,
    recent: &[BaseLayerCacheEntry],
    scene_key: &str,
    target_zoom: f32,
    allowed_ratio: f32,
) -> Option<BaseLayerCacheEntry> {
    let mut candidates = Vec::new();
    if let Some(active) = active.as_ref() {
        candidates.push(active.clone());
    }
    candidates.extend(recent.iter().cloned());
    candidates
        .into_iter()
        .filter(|layer| {
            layer.scene_key == scene_key
                && cache_zoom_ratio_delta(layer.cache_zoom, target_zoom) <= allowed_ratio
        })
        .min_by(|left, right| {
            cache_zoom_ratio_delta(left.cache_zoom, target_zoom)
                .partial_cmp(&cache_zoom_ratio_delta(right.cache_zoom, target_zoom))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn best_matching_detail_tile(
    active: &Option<DetailTileCacheEntry>,
    recent: &[DetailTileCacheEntry],
    scene_key: &str,
    target_zoom: f32,
    visible_left: f32,
    visible_top: f32,
    visible_right: f32,
    visible_bottom: f32,
    allowed_ratio: f32,
) -> Option<DetailTileCacheEntry> {
    let mut candidates = Vec::new();
    if let Some(active) = active.as_ref() {
        candidates.push(active.clone());
    }
    candidates.extend(recent.iter().cloned());
    candidates
        .into_iter()
        .filter(|tile| {
            tile.scene_key == scene_key
                && detail_tile_covers_viewport_geometry(
                    tile,
                    visible_left,
                    visible_top,
                    visible_right,
                    visible_bottom,
                )
                && cache_zoom_ratio_delta(tile.cache_zoom, target_zoom) <= allowed_ratio
        })
        .min_by(|left, right| {
            cache_zoom_ratio_delta(left.cache_zoom, target_zoom)
                .partial_cmp(&cache_zoom_ratio_delta(right.cache_zoom, target_zoom))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn push_recent_base_layer(layers: &mut Vec<BaseLayerCacheEntry>, layer: BaseLayerCacheEntry) {
    layers.retain(|existing| existing.key != layer.key);
    layers.insert(0, layer);
    if layers.len() > MAX_RECENT_BASE_LAYERS {
        layers.truncate(MAX_RECENT_BASE_LAYERS);
    }
}

fn push_recent_detail_tile(tiles: &mut Vec<DetailTileCacheEntry>, tile: DetailTileCacheEntry) {
    tiles.retain(|existing| existing.key != tile.key);
    tiles.insert(0, tile);
    if tiles.len() > MAX_RECENT_DETAIL_TILES {
        tiles.truncate(MAX_RECENT_DETAIL_TILES);
    }
}

fn detail_tile_covers_viewport_geometry(
    tile: &DetailTileCacheEntry,
    visible_left: f32,
    visible_top: f32,
    visible_right: f32,
    visible_bottom: f32,
) -> bool {
    let tile_right = tile.tile_left + tile.tile_width;
    let tile_bottom = tile.tile_top + tile.tile_height;
    visible_left >= tile.tile_left + DETAIL_TILE_REUSE_MARGIN
        && visible_top >= tile.tile_top + DETAIL_TILE_REUSE_MARGIN
        && visible_right <= tile_right - DETAIL_TILE_REUSE_MARGIN
        && visible_bottom <= tile_bottom - DETAIL_TILE_REUSE_MARGIN
}
