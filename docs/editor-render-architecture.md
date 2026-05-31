# 编辑态渲染架构（Editor Render Architecture）

> 本文是 [architecture-principles.md](architecture-principles.md) 中"单一渲染链"原则在编辑器场景的具体落地说明。**任何修改 `editor_host_view.ts` 或编辑器 canvas/textarea 相关代码前必读。**

---

## 核心：Canvas-based 编辑器 ✅

```
┌─ shell (background: transparent, border: none)  ←── 不可见容器
│   ┌─ canvas (display: block)                   ←── 由 Rust 绘制全部可见内容
│   │       ├─ 原 PDF 段落 glyph（保留未改部分）
│   │       ├─ Marker（项目符号，重绘）
│   │       ├─ Draft body（用 PDF 原字体绘制）
│   │       └─ Caret（光标）
│   │
│   └─ textarea (position:fixed; left:-10000px; 1×1px; opacity:0; pointer-events:none)
│           └─ 屏外不可见 — 只用于：
│              • 键盘事件捕获
│              • IME 输入
│              • 系统文本服务（剪贴板/拼写等）
│              • 提供 selection/caret 的逻辑位置
└─
```

**关键规则：**
- `shell.background = transparent` — 不画蓝框/白底
- `textarea.color = transparent`、`-webkit-text-fill-color: transparent !important` — 屏外也保持彻底不可见，防止任何残影
- `canvas.display = block` — 必须可见
- 由 Rust 端 `draw_active_editor_shell_overlay_page` / `draw_persisted_paragraph_overlay_page` / `draw_editor_marker_page` 绘制全部视觉

## 反模式：Textarea-based 编辑器 ❌

历史上 standalone 在分叉时采用过这种结构（已于 2026-05-06 修复）：

```
shell      → 白底 + 蓝边（visible 框）
textarea   → 100% 覆盖 shell, opacity:1, color: 段落实色
canvas     → display: none（被禁用，注释错误地写成"no Rust paint backend"）
```

**症状：**
- 编辑态文字与原 PDF 字形不一致（浏览器字体回退）
- PUA bullet（如 U+F0B7）渲染成 "□"
- 文字位置/颜色与原 PDF 偏移
- 无法保证缩放后的视觉一致性

**根本原因：** 浏览器的 HTML 文本渲染链与 canvas 的 PDF-Glyph 渲染链是**两条独立链**，永远无法精确对齐。这就是"链分叉"。

---

## 关键文件

### TypeScript

- `@/src/bridge/editor_host_view.ts`
  - `ensureEditorHostView()` — 创建/复用 shell + canvas + textarea 节点
  - `positionEditorShell()` — 仅设置 shell/canvas 尺寸位置；textarea 字体属性仅供 IME 候选框定位，颜色必须保持 transparent

### Rust（绘制后端）

- `@/crates/pdf-viewer-ui/src/render/canvas.rs`
  - `draw_active_editor_shell_overlay_page` — 编辑态 overlay 入口
  - `draw_persisted_paragraph_overlay_page` — 段落 overlay 重绘（含 draft）
  - `draw_editor_marker_page` — marker 重绘
- `@/crates/pdf-viewer-ui/src/render/effective_page_plan.rs`
  - `overlay_suppresses_text_source` — 决定原 PDF 文字是否抑制
    - **唯一抑制条件**：`overlay.replaces_source = true`（即 draft 已偏离原文）
    - **不要**因为"编辑器打开"就抑制（会让原文消失露出空白）
  - `overlay_suppresses_row_paths` — 抑制装饰 path

---

## CSS Selection 抑制规则

`editor_host_view.ts` 中 `hostSelectionCss` 必须保持以下结构（防止浏览器原生选区蓝条泄漏）：

```css
/* 1. 容器内禁止用户选择 */
#pdf-content-wrapper *, #vector-container *, ... {
    user-select: none !important;
    -webkit-user-select: none !important;
}

/* 2. 容器及其 ::selection 全部透明（防选区视觉泄漏） */
#pdf-content-wrapper *, ...,
#pdf-content-wrapper *::selection, ... {
    background: transparent !important;
    color: inherit !important;
    -webkit-text-fill-color: currentColor !important;
}

/* 3. 编辑器 textarea 自身彻底透明 */
#pdf-editor-textarea,
#pdf-editor-textarea::selection {
    background: transparent !important;
    color: transparent !important;
    caret-color: transparent !important;
    -webkit-text-fill-color: transparent !important;
}
```

---

## 抑制（Suppression）规则

源自 `origin/pdf-blue-bar-investigation-2026-04-24.md` 的"单一所有者"思想：

| 角色 | 职责 |
|------|------|
| **页面层** (`effective_page_plan` + `canvas`) | 决定原 PDF 对象（text/path/image）是否绘制 |
| **编辑层** (`editor::*`) | 只画 draft body / marker / caret，**不画白底遮罩**，**不抑制原 path** |

如果出现"原 PDF path 漏抑制 → 蓝色横条 / 装饰线泄漏"等问题，**到页面层去修**，绝不能在编辑层加白底遮罩补救。详见 `origin/pdf-blue-bar-investigation-2026-04-24.md`。

---

## 自检清单（编辑态视觉问题排查）

按顺序检查：

1. **canvas 是否 `display: block`？** F12 检查 `#pdf-editor-canvas` 元素。
2. **shell 是否 `background: transparent`？**
3. **textarea 是否在屏外（`left: -10000px`）？** 不应当 100% 覆盖 shell。
4. **textarea 颜色是否 transparent？**
5. **`overlay_suppresses_text_source` 是否只检查 `replaces_source`？** 不应当因 `ActiveEditorShell` owner 抑制。
6. **Rust 端 `draw_active_editor_shell_overlay_page` 是否被调用？** 通过 `[CANVAS-DBG]` / `dbg_event` 日志确认。
7. **Editor canvas 像素尺寸是否匹配 shell 像素尺寸 × DPR？** 否则 canvas 内容会拉伸。
8. **`renderActiveEditor()` 是否调用了 `deps.getWasmApi().render_active_editor_canvas(...)`？** ⚠️ **历史坑**：某次重构误删此调用（注释说"now handled by facade renderFrame"），导致 editor canvas 永远空白。facade 画的是主 page canvas，不是 shell 内 canvas。必须显式调用。

如果 1-4 任一为否，**先回到 nushell-enhanced 的 `src/plugins/pdf-viewer/editor_host_view.ts` 对照修正 CSS**，不要在 Rust 端打补丁。
