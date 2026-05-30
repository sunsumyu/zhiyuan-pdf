# 蓝色长块视觉伪影 —— 根因与修复（Postmortem）

> **更新时间**: 2026-05-18  
> **影响版本**: 在 `LayoutRun::from_styled` 修复 (`scale_x = horizontal_scaling / 100.0`) 之前的所有版本  
> **关联文件**: `crates/pdf-viewer-core/src/models/layout.rs`、`crates/pdf-viewer-ui/src/render/canvas.rs`

---

## 现象

打开 PDF 后，列表项 / 段落上方出现一个**蓝色长块**（视觉上像一个填充矩形，宽度远超文本，覆盖在文字之上或之下），尤其在编辑器激活（caret 出现）时可见。

肉眼以为是「PDF 自带的装饰矩形」或「列表项背景填充」，但实际**不是**。

---

## 根因（一句话）

**`RunStyle.scale_x` 被错误地用 PDF 的 `horizontal_scaling`（百分比，100 = 正常）直接赋值，但下游 Canvas 渲染把它当作比例（1.0 = 正常）使用。** 当 `LayoutRun.char_origins` 为空时，`draw_text_run_core` 走回退分支 `ctx.scale(scale_x, y_scale)`，导致单个字符被水平拉伸 **100 倍**，看上去就像一个填充色块。

> 修复点：`crates/pdf-viewer-core/src/models/layout.rs` 的 `LayoutRun::from_styled`

```rust
// 错误（旧）
scale_x: run.horizontal_scaling,            // 100.0 当成比例用 → ×100 拉伸

// 正确（新）
scale_x: run.horizontal_scaling / 100.0,   // PDF 百分比转 Canvas 比例
```

---

## 完整链路（为什么"蓝"、为什么"块状"）

1. **PDF 解析**（`src-tauri/src/infrastructure/pdf/pdf_read.rs`）  
   `StyledRun.horizontal_scaling` 来自 PDF 的 `Tz` 算子，单位是百分比（默认 100.0）。

2. **Layout 构造**（`crates/pdf-viewer-core/src/models/layout.rs::LayoutRun::from_styled`）  
   把 `StyledRun` 映射到 `LayoutRun` 时，**未做单位换算**，把 100.0 直接塞进了 `RunStyle.scale_x`。  
   `RunStyle.scale_x` 的契约是 Canvas 比例（默认 `default_scale_x = 1.0`，见 `models/glyph.rs`）。

3. **Draft Layout 规范化**（`crates/pdf-viewer-core/src/edit/draft_layout.rs::normalize_style_run`）  
   清空了 `char_origins` 和 `char_widths`，准备让下游用 `ctx.scale()` 走回退渲染分支。

4. **Canvas 回退渲染**（`crates/pdf-viewer-ui/src/render/canvas.rs::draw_text_run_core`）  
   ```rust
   if char_origins.is_empty() {
       ctx.scale(scale_x as f64, y_scale);  // scale_x = 100.0 → ×100 拉伸
       ctx.fill_text(text, 0.0, 0.0);
   }
   ```
   一个字宽 ~10px 的字符 ×100 → 屏幕上变成 ~1000px 宽的"长块"。颜色是 `run.style.color`（多数 PDF 列表 marker / 主题文字是蓝色），所以呈现为**蓝色长块**。

5. **为什么编辑器激活时更明显**  
   `draw_active_editor_shell_overlay_page` 走 "caret only" 分支时，依赖 path 抑制隐藏 PDF 装饰，但**不会**额外覆盖白色背景；而 `draw_persisted_paragraph_overlay_page`（编辑后）会铺一层白色 `fill_rect`，遮住了这个伪影。所以"刚打开未编辑"时蓝块可见，"编辑后"反而看不到。

---

## 为什么 `sanitize_draft_run_style` 没拦住

`crates/pdf-viewer-core/src/edit/draft_layout.rs::sanitize_draft_run_style` 已经把 `scale_x` 夹紧到 `[0.5, 2.0]`：

```rust
if !run.style.scale_x.is_finite() || run.style.scale_x < 0.5 || run.style.scale_x > 2.0 {
    run.style.scale_x = 1.0;
}
```

这个夹紧**意外掩盖**了正文 body run 的 bug（100.0 → 1.0）。但是：

- **Marker run（列表符号"●"等）走 `synthesize_marker_from_paragraph` → 直接渲染，不经过 `sanitize_draft_run_style`**，所以 100.0 原值传到 canvas。这就是为什么**蓝块通常出现在列表项位置**——它就是被 ×100 的圆点 marker。

---

## 排查建议（下次出现类似伪影时）

1. **先确认是不是 PDF 原内容**  
   用 `lopdf` / Adobe Reader / 浏览器自带 PDF 查看器打开原文件对比。

2. **加 chain_trace 日志，对比"绘制路径"和"绘制位置"**  
   在 `crates/pdf-viewer-ui/src/render/canvas.rs` 的 `draw_vector_object`（path / image 分支）加：
   ```rust
   crate::chain_trace!(
       "render.path.drawn",
       "id" => path.id.as_str(),
       "fillColor" => path.fill_color.as_deref().unwrap_or("none"),
       "bbox" => format!("[{:.2},{:.2},{:.2},{:.2}]", b.left, b.top, b.right, b.bottom),
   );
   ```
   - **如果完全没有 path/image 日志匹配该位置** → 不是 vector 对象，**90% 是被拉伸的文字**（本次场景）。
   - **如果有 path 日志且 `fillColor` 是蓝色** → 真是 PDF path，检查 `decorative_object_should_be_suppressed_by_overlay`。

3. **重点检查任何「百分比/比例」单位换算的边界**  
   - `horizontal_scaling`（PDF Tz）：百分比 (100 = 1.0×)
   - `RunStyle.scale_x`：比例 (1.0 = normal)
   - `font_size`：page-space 点
   - `dpr` / `zoom`：屏幕比例
   
   每个 `From` / `from_styled` 转换函数都是单位换算的潜在 bug 点。

4. **确认 WASM 真的被重建了**  
   ```pwsh
   npm run wasm:pdf-viewer-ui
   ```
   或运行 `rebuild.ps1`。然后在 Tauri 窗口里 `Ctrl+R` 重新加载。否则改了 Rust 代码看到的仍是旧 WASM。

---

## 相关代码引用

- `crates/pdf-viewer-core/src/models/layout.rs:165` —— 修复点
- `crates/pdf-viewer-core/src/models/glyph.rs::default_scale_x` —— 默认值 1.0，确认 scale_x 是比例不是百分比
- `crates/pdf-viewer-ui/src/render/canvas.rs::draw_text_run_core` —— `ctx.scale()` 回退分支
- `crates/pdf-viewer-core/src/edit/draft_layout.rs::sanitize_draft_run_style` —— 夹紧到 `[0.5, 2.0]`（设计就是给比例用的）
- `crates/pdf-viewer-core/src/edit/draft_layout.rs::normalize_style_run` —— 清空 `char_origins` 触发回退分支
- `crates/pdf-viewer-core/src/edit/document_plan.rs::synthesize_marker_from_paragraph` —— 列表 marker 合成（绕过 sanitize）
- `src-tauri/src/infrastructure/pdf/pdf_read.rs` —— `horizontal_scaling` 从 PDF 读取，单位是百分比

---

## 回归测试建议

在 `crates/pdf-viewer-core/src/models/layout.rs` 加单元测试：

```rust
#[test]
fn from_styled_converts_horizontal_scaling_percentage_to_ratio() {
    let styled = StyledRun { horizontal_scaling: 100.0, ..Default::default() };
    let layout = LayoutRun::from_styled(&styled, /* ... */);
    assert!((layout.style.scale_x - 1.0).abs() < 1e-6,
            "scale_x must be a ratio, not a percentage");

    let styled_50 = StyledRun { horizontal_scaling: 50.0, ..Default::default() };
    let layout_50 = LayoutRun::from_styled(&styled_50, /* ... */);
    assert!((layout_50.style.scale_x - 0.5).abs() < 1e-6);
}
```
