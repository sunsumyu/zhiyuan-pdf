use crate::infrastructure::pdf::layout_engine::LayoutGraphAnalyzer as CoreAnalyzer;
use crate::infrastructure::pdf::models::{LayoutInferenceResult, StyledRun};

/// 布局分析引擎 - V3 (Tauri 宿主代理)
pub struct LayoutGraphAnalyzer {
    inner: CoreAnalyzer,
}
impl LayoutGraphAnalyzer {
    pub fn new(page_index: u16, width: f32, height: f32) -> Self {
        Self {
            inner: CoreAnalyzer::new(page_index, width, height),
        }
    }

    /// 执行三阶段排版推断 (委派给核心库)
    pub fn analyze(&self, runs: Vec<StyledRun>) -> LayoutInferenceResult {
        // [V3] 核心逻辑已下沉至 pdf-viewer-core
        let layout_runs = runs
            .iter()
            .map(pdf_viewer_core::models::LayoutRun::from_styled)
            .collect();
        self.inner.resolve_regions(layout_runs)
    }

    /// 推测列带 (委派给核心库)
    pub fn detect_column_bands(&self, _runs: &[StyledRun]) -> Vec<f32> {
        // TODO: 后续将 detect_column_bands 的实现也迁移至核心库
        Vec::new()
    }
}
