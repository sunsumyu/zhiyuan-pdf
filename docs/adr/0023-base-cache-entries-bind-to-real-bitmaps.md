# ADR-0023: base 缓存条目必须绑定真实位图（修复缩放后持续模糊）

## Status

Accepted

## Context

### 用户报告（2026-10-03 截图）

「为什么不是矢量化的？」——简历 PDF 在 119% 下文字发虚，像位图拉伸。

### 排查（全部程序化取证）

1. **文档本身是矢量**：解析用户 PDF（WPS 导出）——649 个文本块、3341 个
   文本显示算子、5 种嵌入字体、5470 条矢量直线；内嵌图像仅 5 个小图标。
2. **当前构建 settle 后渲染是原生清晰的**：同文档同缩放（119%）实测
   `R = 位图CSS宽/显示CSS宽 = 1.001`（位图 886×1252 设备 px = 708 CSS × dpr1.25）。
3. **但用户截图里的文字边缘过渡宽度是原生渲染的 ~2×**（同 dpr=1.25、同文档、
   同缩放逐像素比对：5.13px vs 2.63px；5.52px vs 2.75px）——即截图捕获的是
   **一张被约 2× 放大的位图**，而非矢量光栅化帧。
4. **复现**：缩到 0.66 再快速滚回 ~1.23，settle 后 **R=0.812 持续不恢复**
   （base 位图停在初始 1.0 的 744×1052，被拉到 733 CSS；瓦片也是旧 band），
   而 `lastRenderedZoom=1.231`——状态声称已渲染，像素从未重画。

### 根因：幻影 base 缓存条目（phantom cache entries）

链条：

1. 手势中每个 reknock 帧提交时，`settle_render_frame_inner` 按
   `render_base_layer || reuse_active_base_layer` 调 `remember_base_layer`。
   mid-gesture 的 plan 因 `preview_active` 恒有 `reuse_active_base_layer=true`，
   且其 `base_cache_zoom = quantize(visual)`（无可复用层时）——于是**每个
   mid-gesture visual 都被记成一条 base 缓存条目，尽管那个 zoom 的 base 位图
   从未渲染**（plan 走的是"复用旧底图 + 视口瓦片"路径）。
2. `remember_base_layer` 无条件把这些**幻影条目**设为 `active_base_layer`
   并推入 `recent_base_layers`。
3. settle（如 1.231）时 `find_reusable_base_layer(quantize=1.23)` **命中最后
   一条幻影条目**；`reusable_base_layer_is_displayed(Some(幻影), Some(幻影))`
   key 相等 → true → `reuse_active_base_layer=true` →
   `render_base_layer=false` → `frame_plan_requires_render=false` →
   **settle 不调度任何帧**，仅走 zoom-state-commit（`lastRendered=1.231`）。
4. 屏幕上是旧 base 位图的 CSS 拉伸 → 持续模糊，直到下次缩放重复中毒循环。

本质是**单一事实源违规**：base 缓存条目应当表示"一张以该 zoom 真实渲染、
且仍存在的位图"，但 remember 的触发条件（reuse 也记）让"从未渲染的 zoom"
也产生了条目；而 reuse 判定又不校验位图是否真的存在（Rust present_state
与 TS frame cache 脱节）。

## Decision

### 1. 只在真渲染时登记（断毒源）

`settle_render_frame_inner`：`remember_base_layer` 的条件从
`render_base_layer || reuse_active_base_layer` 收紧为 **`render_base_layer`**。
（真复用路径无需重记——被复用条目本就是 active；mid-gesture 路径不再产毒。）

### 2. 复用必须校验位图真实存在（清存量 + 防复发）

`build_frame_plan_result` 增加 `frame_cache: &HostFrameCacheState` 参数；
`reusable_base_layer` 命中后校验其 `key ∈ frame_cache.stored_base_frame_keys`
（TS 渲染位图后经 `storeFrameCacheEntry` 登记，含 LRU 逐出）——不存在则视为
无可用底图 → `render_base_layer=true` 强制重渲。已存在的幻影条目由此失效
（自愈），`has_displayed_base_layer`/`effective_base_cache_zoom`/`base_cache_key`
均基于过滤后的结果。

## 红灯契约

`crates/pdf-viewer-ui/src/present/plan_tests.rs` 与 render 层测试：

1. **产毒断链**：提交一个 mid-gesture 形态的帧
   （`render_base_layer=false, reuse_active_base_layer=true,
   base_cache_zoom=quantize(visual)`）后，`active_base_layer` 不得被
   创建/覆盖。修复前红（被写入幻影条目）。
2. **幻影免疫**：present_state 预置 `active_base_layer{cache_zoom:1.23,
   key:K}` 而 frame cache 中**无** K 时，1.231 的 plan 必须
   `render_base_layer=true`。修复前红（false）。
3. **合法复用不回退**：frame cache 中**有** K 时，plan 保持
   `render_base_layer=false`（2% 内复用优化保留）。

## Consequences

### Positive

1. 缩放 settle 后文字恒为原生分辨率矢量光栅化（R≈1.0），"缩放后持续模糊"
   消失——这是用户报告"不是矢量化"的直接根因。
2. 缓存条目回归单一事实源：条目 ⇔ 真实存在的位图。

### Negative / 技术债

1. base 复用（2% 容差）从"Rust 条目命中即可"收紧为"且位图仍在 TS frame
   cache"——LRU 逐出后会多一次重渲（正确性优先，量级为偶发一次全页渲染）。

## Verification

- 红灯：3 契约修复前红 → 绿；wasm / core / vitest / clippy 全绿。
- E2E：全量 12 spec（打包产物）全绿。
- CDP 复现脚本：0.66 → 快速滚回 1.231，settle 后 R 从 0.812 → ~1.0。
