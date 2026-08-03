# Manual E2E Verification Runbook — Zoom Without Double-Scaling Flash

> 对应 Local tracker: `.scratch/zoom-layout-refactor-verification/issues/03-manual-e2e-verification.md`
> 目标：人工确认 zoom 缩放不再出现"先闪大图再弹回"（double-scaling flash）。

## 前置

- [ ] 已在本分支 `codex/refactor-split`（HEAD 含 `87bf89a` 单测 + `a8e7f49` 流程文档）
- [ ] WASM 已构建（`npm run wasm:pdf-viewer-ui`）
- [ ] 打开一个多页 PDF（建议含文字与图形，便于观察重绘）

## 启动

```bash
npm run tauri:dev      # 桌面 app（Tauri + Vite）
# 或纯前端（只测布局/缩放逻辑，不测 Tauri 层）：
npm run dev            # Vite-only，浏览器访问 http://localhost:5000
```

> 建议先跑 `npm run dev`（Vite-only）做布局/缩放验证，再用 `npm run tauri:dev` 复核桌面封装。

## 验证步骤

### A. Dropdown 缩放（100% → 125% → 150%）

1. 缩放下拉框选 **125%**。
2. **观察**：页面不应先放大到 1.5625 倍再弹回 1.25 倍；应一次平滑过渡到位。
3. 选 **150%**，同样观察。
4. 再往回选 125% → 100%，确认无反向弹跳。

**通过标准**：任意档位切换时页面尺寸单调逼近目标值，无"放大→弹回"瞬态；切换期间文字/图形无模糊重绘跳动。

### B. 工具栏缩放按钮（+/−）

1. 连续点击 **+** 数档（建议到 200%）。
2. 连续点击 **−** 数档（回到 100%）。
3. 观察同上。

**通过标准**：逐档平滑，无闪图、无尺寸跳变。

### C. Ctrl + 鼠标滚轮

1. 在页面内容上按住 **Ctrl** 滚动滚轮向上（放大）。
2. 再向下（缩小）。
3. 快速连续滚动，观察动画插值是否平滑。

**通过标准**：缩放连续平滑，无 pop/flash；缩放过程中锚点（鼠标位置附近的内容）大致保持稳定，无跳动。

### D. 滚动与缩放组合（回归）

1. 缩放到 150%，上下滚动长页。
2. 切换页面（翻页按钮 / 滚动到底部）。
3. 缩放回 100%，确认滚动条位置、页面居中正常。

**通过标准**：滚动、翻页、缩放组合无布局崩溃、无残留白边或错位。

## 结果记录

- [ ] A 通过　B 通过　C 通过　D 通过
- [ ] 若任一失败，记录：失败项 / 复现步骤 / 控制台错误（DevTools console 截图或文本）

## 通过后收尾

1. 回填 `docs/plans/2026-07-30-zoom-layout-refactor-plan.md` Task 3 Step 3/4 为 `- [x]`。
2. 提交验证 marker：

```bash
git commit --allow-empty -m "test(zoom): verify smooth zoom without double-scaling flash"
```

3. 关闭 Ticket 03（`.scratch/zoom-layout-refactor-verification/issues/03-manual-e2e-verification.md` 顶部 `Status: ready-for-agent` → `ready-for-human` 或 `wontfix` 视结果）。
