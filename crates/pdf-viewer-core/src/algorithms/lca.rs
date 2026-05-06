use serde::{Deserialize, Serialize};

/// 语义树节点，用于表示 PDF 区域或 Run 的层级关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNode {
    pub id: String,
    pub children: Vec<SemanticNode>,
}

impl SemanticNode {
    pub fn new(id: String) -> Self {
        Self {
            id,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: SemanticNode) {
        self.children.push(child);
    }
}

/// 最近共同祖先 (LCA) 查找算法
pub fn find_lca_in_tree(root: &SemanticNode, p_id: &str, q_id: &str) -> Option<String> {
    if root.id == p_id || root.id == q_id {
        return Some(root.id.clone());
    }

    let mut found_children = Vec::new();
    for child in &root.children {
        if let Some(found_id) = find_lca_in_tree(child, p_id, q_id) {
            found_children.push(found_id);
        }
    }

    if found_children.len() >= 2 {
        // 如果在不同的子树中找到了 p 和 q，则当前节点就是 LCA
        Some(root.id.clone())
    } else if found_children.len() == 1 {
        // 如果只在一个子树中找到，则返回该结果
        found_children.into_iter().next()
    } else {
        None
    }
}

/// 快速路径搜索：返回从根到目标的路径 ID 序列
pub fn find_path_to_node(root: &SemanticNode, target_id: &str, path: &mut Vec<String>) -> bool {
    path.push(root.id.clone());
    if root.id == target_id {
        return true;
    }

    for child in &root.children {
        if find_path_to_node(child, target_id, path) {
            return true;
        }
    }

    path.pop();
    false
}
