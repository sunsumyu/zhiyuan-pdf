# Nushell 分叉报告 — 2026-05-06

> 在排查"编辑态文字飘移、tofu 字符、蓝框过宽"等问题时发现：本项目（standalone）从 `E:\chain\nushell-enhanced` 分叉时，**编辑器的 4 处关键代码偏离了 nushell-enhanced 的设计**，导致用户报告的所有视觉缺陷。本文记录分叉点、影响、修复。

---

## 背景

用户在多次反馈"还是有偏移"、"链分叉了"后明确指出："**一定不要用浏览器的绘制啊，要用 PDF-Glyph**"。这与 nushell-enhanced 的"单一渲染链"设计完全一致。详见：

- [architecture-principles.md](architecture-principles.md) §1
- [editor-render-architecture.md](editor-render-architecture.md)

---

## 分叉点 1：Editor UI 架构（最严重）

**文件：** `@e:\chain\pdf-viewer-standalone\src\bridge\editor_host_view.ts`

| 元素 | nushell-enhanced（正确） | standalone（分叉后） | 影响 |
|------|------------------------|---------------------|------|
| `shell` | `background: transparent; border: none` | 白底 + 蓝边可见框 | 编辑态出现不该有的蓝框 |
| `textarea` | `position: fixed; left:-10000px; 1×1px; opacity:0` | 100% 覆盖 shell, opacity:1, color: 实色 | **textarea 用浏览器字体渲染所有可见文字 → tofu / 飘移** |
| `canvas` | `display: block`（Rust 绘制 draft）| `display: none` | Rust 绘制成果不可见 |
| 错误注释 | — | "no Rust paint_active_editor backend" | 误导后续修复者 |

### 修复

恢复为 nushell-enhanced 版本：

- shell 透明无边
- textarea 屏外 1×1 不可见，仅做输入捕获
- canvas 可见（display: block）
- `positionEditorShell` 不再设置实色 textarea 颜色

---

## 分叉点 2：`is_decorative_glyph` 字符表

**文件：** `@e:\chain\pdf-viewer-standalone\crates\pdf-viewer-core\src\text\glyph_layout.rs`

我在排查中误将其扩展为 30+ 字符 + PUA 范围（U+E000-U+F8FF），偏离 nushell。

### 还原

```rust
pub fn is_decorative_glyph(ch: char) -> bool {
    matches!(ch, '•' | '●' | '▪' | '◦' | '·' | '○' | '-' | '▶' | '➤')
}
```

PUA bullet 在 nushell 是通过 **`looks_like_symbolic_font`**（按字体名识别 Symbol/Wingdings/ZapfDingbats）来检测的，不是按字符 codepoint。后者由 `core::typography::font_resolver` 处理。

---

## 分叉点 3：`source_runs.rs` 运行时排序

**文件：** `@e:\chain\pdf-viewer-standalone\crates\pdf-viewer-ui\src\editor\source\source_runs.rs::resolve_preferred_editor_session`

我添加了按 `(origin_y, origin_x)` 的运行时排序，认为 PDF content-stream 顺序与视觉顺序不同。

### 还原

移除排序，保留 vector model 的原始顺序。nushell 不做这层排序——视觉顺序在更上游（`pdf-viewer-core` 解析阶段）已确定。

---

## 分叉点 4：`resolve_shell_bbox` 范围

**文件：** `@e:\chain\pdf-viewer-standalone\crates\pdf-viewer-ui\src\editor\session\document_plan.rs`

我把 shell_bbox 缩小到仅 body，去除 marker。但因为编辑器架构是 canvas-based（marker 由 Rust 重绘在 shell 内），shell 必须包含 marker 区域。

### 还原

```rust
fn resolve_shell_bbox(target_session: &EditorSession, split: &SessionSplit) -> BoundingBox {
    if let Some(marker) = split.marker.as_ref() {
        let mut shell_bbox = split.body_session.anchor_bbox;
        if let Some(marker_bbox) = bbox_from_runs(&marker.runs) {
            shell_bbox.left = shell_bbox.left.min(marker_bbox.left);
            shell_bbox.top = shell_bbox.top.min(marker_bbox.top);
            shell_bbox.right = shell_bbox.right.max(marker_bbox.right);
            shell_bbox.bottom = shell_bbox.bottom.max(marker_bbox.bottom);
        }
        return shell_bbox;
    }
    target_session.anchor_bbox
}
```

---

## 分叉点 5：`overlay_suppresses_text_source`

**文件：** `@e:\chain\pdf-viewer-standalone\crates\pdf-viewer-ui\src\render\effective_page_plan.rs`

我在该函数中加入了"`ActiveEditorShell` owner 也抑制原文"的分支，导致编辑器一打开原 PDF 文字就消失（暴露出后面的 textarea 渲染）。

### 还原

```rust
fn overlay_suppresses_text_source(overlay: &ParagraphRenderOverlay) -> bool {
    overlay.replaces_source
}
```

只有 draft 实际偏离原文（`replaces_source = true`）时才抑制。这样编辑刚打开尚未输入时，原 PDF 文字保持可见，**视觉与原文 100% 一致**。

---

## 教训（写入 [architecture-principles.md](architecture-principles.md) §5）

1. **症状端不能修。** 蓝框宽 → 不要去缩 shell；tofu → 不要去扩字符表；飘移 → 不要去对齐 textarea 字体。所有这些都是浏览器渲染链的副作用。**正解是关闭浏览器渲染链**。

2. **遇到"修这边坏那边"立即怀疑分叉。** 这次每次"修复"都引发新症状，就是典型分叉链信号。

3. **优先读 nushell-enhanced 的对应代码。** 如果两个项目处理同一个用例的代码不同，且 nushell 已知工作正常，**复制过来不要发明新方案**。

4. **错误注释比错误代码更危险。** "no Rust paint_active_editor backend" 这条注释让我去 Rust 端找问题，浪费数小时；实际 backend 一直工作正常，只是 CSS 把它隐藏了。

---

## 验证

修复后构建：

```
WASM: 1,672 KB（pdf_viewer_ui_bg-CS76D1VT.wasm）
JS:   205 KB（index-CguE5lu2.js）
CSS:  8.76 KB（index-BwZi6EYl.css）
```

视觉验证项（用户测试）：

- [ ] 进入编辑态后字体/颜色/位置与原 PDF 100% 一致
- [ ] 不再出现 "□" tofu 字符
- [ ] 不再出现过宽蓝色框
- [ ] 退出编辑态后页面恢复正常
- [ ] 输入新字符时，原字符被覆盖、新字符以 PDF 字体绘制

如有任一项不通过，**先用 F12 检查 shell/textarea/canvas 的 CSS 状态是否符合 [editor-render-architecture.md](editor-render-architecture.md) 的要求**，再考虑 Rust 端排查。
