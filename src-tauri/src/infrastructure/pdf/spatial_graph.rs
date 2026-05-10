//! Spatial adjacency graph used by `layout_engine`.
//!
//! Moved from `pdf_viewer_core::algorithms::graph` (architecture-review §2.2 /
//! Phase 2: P3 single-side module relocation). The graph is only used by the
//! layout-inference pass, which is itself only invoked from the Tauri side
//! (`vector_engine` -> `LayoutGraphAnalyzer`), so it does not need to live in
//! the cross-side `pdf-viewer-core` crate.

use pdf_viewer_core::models::LayoutRun;
use std::collections::{HashMap, HashSet};

/// 空间邻接图：用于建模文本块之间的 2D 拓扑关系
#[derive(Default)]
pub struct SpatialGraph {
    pub nodes: Vec<LayoutRun>,
    pub adjacency: HashMap<usize, Vec<usize>>,
}

impl SpatialGraph {
    pub fn new(nodes: Vec<LayoutRun>) -> Self {
        Self {
            nodes,
            adjacency: HashMap::new(),
        }
    }

    /// 建立邻接关系（基于 2D 距离阈值）
    pub fn build_adjacency(&mut self, horizontal_threshold: f32, vertical_threshold: f32) {
        let n = self.nodes.len();
        for i in 0..n {
            for j in i + 1..n {
                if self.are_neighbors(i, j, horizontal_threshold, vertical_threshold) {
                    self.add_edge(i, j);
                }
            }
        }
    }

    /// 判定两个文本块是否为物理邻居
    fn are_neighbors(&self, i: usize, j: usize, h_thresh: f32, v_thresh: f32) -> bool {
        let a = &self.nodes[i].bbox;
        let b = &self.nodes[j].bbox;

        // 水平连通性：高度重叠且间距较小
        let v_overlap = f32::min(a.bottom, b.bottom) - f32::max(a.top, b.top);
        let h_gap = if a.right < b.left {
            b.left - a.right
        } else if b.right < a.left {
            a.left - b.right
        } else {
            0.0 // 重叠
        };

        if v_overlap > 0.5 && h_gap <= h_thresh {
            return true;
        }

        // 垂直连通性：宽度重叠且间距符合行高
        let h_overlap = f32::min(a.right, b.right) - f32::max(a.left, b.left);
        let v_gap = if a.bottom < b.top {
            b.top - a.bottom
        } else if b.bottom < a.top {
            a.top - b.bottom
        } else {
            0.0
        };

        if h_overlap > 0.5 && v_gap <= v_thresh {
            return true;
        }

        false
    }

    fn add_edge(&mut self, i: usize, j: usize) {
        self.adjacency.entry(i).or_default().push(j);
        self.adjacency.entry(j).or_default().push(i);
    }

    /// 寻找连通分量 (Connected Components)，即初步的语义聚类区域
    pub fn find_components(&self) -> Vec<Vec<usize>> {
        let mut visited = HashSet::new();
        let mut components = Vec::new();

        for i in 0..self.nodes.len() {
            if !visited.contains(&i) {
                let mut component = Vec::new();
                let mut queue = vec![i];
                visited.insert(i);

                while let Some(node_idx) = queue.pop() {
                    component.push(node_idx);
                    if let Some(neighbors) = self.adjacency.get(&node_idx) {
                        for &neighbor in neighbors {
                            if !visited.contains(&neighbor) {
                                visited.insert(neighbor);
                                queue.push(neighbor);
                            }
                        }
                    }
                }
                components.push(component);
            }
        }
        components
    }
}
