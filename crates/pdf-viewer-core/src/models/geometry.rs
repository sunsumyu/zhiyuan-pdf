use serde::{Deserialize, Serialize};

/// 表示 2D 空间内的绝对物理包围盒。
///
/// # Invariants & Coordinate Systems (重要坐标系声明)
/// `BoundingBox` 被设计为兼容两种极性空间，但**一旦被实例化进入流转层，必须约定其处于 Y-Down 规范**。
/// 在 `Y-Down` 的前提下，`top` 始终表示视觉上方的边际，其数值 **严格小于** `bottom`。
///
/// 违反 `top < bottom` 的包围盒被视为非法坍缩体，将直接导致碰撞检测抛出和选区渲染错误。
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl BoundingBox {
    /// 执行原地 Y 轴极性反转。通常在数据从解析器（Parser）送入呈现层（Presentation Pipeline）时触发。
    ///
    /// # Arguments
    /// * `h` - 作为反射轴基准的页面总高度。
    ///
    /// # Thread Safety
    /// 原地就地修改，开销仅为几个 f32 指令。
    pub fn flip_y(&mut self, h: f32) {
        let old_top = self.top;
        let old_bottom = self.bottom;
        self.top = h - old_bottom;
        self.bottom = h - old_top;
    }
}
