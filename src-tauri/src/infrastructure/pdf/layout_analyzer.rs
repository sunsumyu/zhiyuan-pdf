use crate::infrastructure::pdf::layout_engine::LayoutGraphAnalyzer as CoreAnalyzer;
use crate::infrastructure::pdf::models::{LayoutInferenceResult, StyledRun};

/// 甯冨眬鍒嗘瀽寮曟搸 - V3 (Tauri 瀹夸富浠ｇ悊)
pub struct LayoutGraphAnalyzer {
    inner: CoreAnalyzer,
}
impl LayoutGraphAnalyzer {
    pub fn new(page_index: u16, width: f32, height: f32) -> Self {
        Self {
            inner: CoreAnalyzer::new(page_index, width, height),
        }
    }

    /// 鎵ц涓夐樁娈垫帓鐗堟帹鏂?(濮旀淳缁欐牳蹇冨簱)
    pub fn analyze(&self, runs: Vec<StyledRun>) -> LayoutInferenceResult {
        // [V3] 鏍稿績閫昏緫宸蹭笅娌夎嚦 pdf-viewer-core
        let layout_runs = runs
            .iter()
            .map(|r| pdf_viewer_core::models::TextRun::from_styled(r).to_layout_run())
            .collect();
        self.inner.resolve_regions(layout_runs)
    }

    /// 鎺ㄦ祴鍒楀甫 (濮旀淳缁欐牳蹇冨簱)
    pub fn detect_column_bands(&self, _runs: &[StyledRun]) -> Vec<f32> {
        // TODO: 鍚庣画灏?detect_column_bands 鐨勫疄鐜颁篃鎼縼鑷虫牳蹇冨簱
        Vec::new()
    }
}
