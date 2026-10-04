# 缩放双重曝光 / 静止态重影 — backCanvas 陈旧视口补丁（2026-10-03）

会话：sess_b28adc61（用户录屏 → 逐帧取证 → 代码归因）。取证完成于 2026-10-03
（当时只分析未修复）；**修复已于 2026-10-04 全部闭环 → ADR-0024
（DetailOverlayOwner，修复方向 1）+ E2E 帧级契约（P4，含变异校验）。
P1~P4 全部关闭。**

## 素材

- 录屏：`pdf-viewer-standalone_r51dNMZ5vl.mp4`（ShareX，11.43s，30fps，
  2560×1360）。分析时按 10fps 抽帧 114 张（临时目录，已删）+ 关键帧裁剪测量。
- 关键前提（mtime 实证）：`target/debug/pdf-viewer-standalone.exe` 构建于
  13:19:17，视频录制于 13:20:41 —— **跑的是含 ADR-0023 修复的最新构建**。
- 文档：简历 PDF（矢量，同 ADR-0023 会话取证的 WPS 导出件）。

## 结论（TL;DR）

**缩放过程中的"双重曝光"与静止后的持续重影，不是主 canvas、也不是瓦片层，
而是 detail overlay（backCanvas，`#pdf-vector-detail-canvas`，z-index 2）上
残留的"视口瓦片补丁"。** 手势中每次 reknock 提交把中间档位的视口补丁 present
到 backCanvas；backCanvas 是唯一**不做逐帧几何补偿**的表面（ADR-0009 统一公式
只覆盖主 canvas 与瓦片），且**没有人在手势结束时隐藏它**——补丁冻结在提交
时刻的档位与容器空间，缩小时比页面大、溢出页面边界，静止后停留 0.5–1.4s，
直到 settle 的 base 渲染落地才被顺带隐藏。

ADR-0023 的"settle 后持续模糊"在本次录屏**未复现**（各 settle 点最终原生
清晰）；ADR-0017 的忙环冻结也未复现。这是另一条未被覆盖的缺陷链。

## 帧时间线（10fps 实测，帧号 f_N ≈ (N−1)/10 s）

| 时间 | 缩放控制 | 画面 |
|---|---|---|
| 0.65–1.5s | 100→71→65→…→42% | 滚轮快速缩小 |
| 1.55s / 1.95s | 42%（静止） | **两帧完全相同**：页面右侧/底部伸出 ≈1.5× 目标档（约 0.6–0.65 档位）的白底渲染碎片——静止状态下陈旧表面卡住 |
| 2.0–3.2s | 42→84→100% | 反向放大；中间帧文字发虚（旧位图 CSS 拉伸，设计内）；84% 处干净（放大的补丁被变大的页面盖住） |
| 5.9–7.4s | 100→141% | 6.55s 中间帧轻微重影；7.35s settle 后清晰 |
| 8.8–9.3s | 141→100% | 9.25s 同款重影：≈1.19× 档位白底内容伸出 100% 页面右缘 |
| 9.55–10.4s | 100%（静止） | 重影逐帧缩小（后续 follow-up 补丁逐档逼近目标），~10.2–10.4s 消失（settle base present 落地） |

关键判据：重影是**带白底的整页渲染碎片**（排除透明文字层），静止后位置
**纹丝不动**（排除动画中间态），且**只在缩小方向明显**——放大时补丁比页面
小被盖住，缩小时补丁比页面大从边缘露出。

## 机制链（代码定位）

1. **reknock 帧只渲视口补丁并 show 到 backCanvas**。
   `resolve_present_policy`（`crates/pdf-viewer-core/src/render/present_plan.rs:22`）
   在 preview 期间给出 `reuse_active_base_layer=true`、`use_viewport_tile=true`、
   `show_detail_overlay=true`；TS 侧（`src/bridge/render/vector_host.ts`
   renderLayer → `presentViewportCanvasFromSource` + `presentViewportCanvas`
   → `surface.showDetail()`）把补丁 present 到 backCanvas。reknock 节流 60ms、
   blur 阈值 2%（`crates/pdf-viewer-core/src/render/zoom/zoom_render.rs:21-23`）、
   worker 串行 → **可见补丁落后 visual 1–3 个档位**。
2. **backCanvas 无几何补偿（已知债，第三次被录像实锤）**。ADR-0009 统一公式
   `scale(visual/renderZoom)` 只作用于主 canvas（CanvasTransformOwner，经
   `tile_layer.ts:544-550` syncVisualTransforms）与瓦片（`refreshTileTransforms`）；
   backCanvas 的 CSS box 与位图冻结在提交时刻。HANDOFF 三处记录该债（2026-09-30
   续/续三、ADR-0010 遗留）。
3. **手势结束无人隐藏补丁**。`hideDetail` 的唯一活跃触发点是"下一次 base
   present 顺带隐藏"（`presentViewportCanvas` 的 `showDetailOverlay=false` 分支）；
   `applyViewportCanvasFrame` 里另一处隐藏因 `deferVisiblePresent=true` 恒定
   （`vector_host.ts:360`）是死代码。settle 走 skip-render / zoom-state-commit
   （`src/bridge/render/render_flow.ts:694-707`）时无任何 present → 补丁滞留。
   本次录屏它最终消失是因为 settle base 渲染（easing 收敛 ~0.3s +
   `SETTLE_DRAWING_DELAY_MS=30ms` + 串行 worker 整页渲染）约 1s 后落地。
4. **补丁 box 可超出页面框**。补丁的 viewport rect（含 overscan）在"当时"的
   容器空间计算；容器随后按新档位收缩，补丁的绝对 CSS box 不跟随 → 白色
   区域溢出到深色背景，即"页面外的白色假页面"。

## 问题清单

- **P1**（高）静止状态显示错档位陈旧补丁（双重曝光/白色假页面），持续
  0.5–1.4s。存在**永久变体**：settle 落在"合法复用已显示 base"分支
  （`frame_plan_requires_render=false`，`crates/pdf-viewer-core/src/render/workflow.rs:35`）
  时无任何帧调度 → 补丁无限期滞留（42% 段在录屏中被下一次手势打断，未能
  证实是否自愈）。
- **P2**（高）手势进行中补丁落后 visual 1–3 档（60ms 节流 + 串行 worker），
  快速手势全程可见两份不同档位内容叠加。
- **P3**（根因）backCanvas 无 scale/box 补偿——P1/P2 的几何根因。
- **P4**（验证缺口）缩小方向远糟于放大（补丁>页面时溢出边界）。现有 E2E
  契约（瓦片网格对齐、canvas 宽度漂移）抓不到此类帧级/静止态缺陷，需新增
  "快速缩小后静止 N 秒逐帧采样"契约。

## 修复方向（2026-10-04 已实施方向 1 → ADR-0024）

1. ✅ **backCanvas 纳入统一几何**：`DetailOverlayOwner`
   （`src/bridge/render/detail_overlay_owner.ts`）记录补丁 base rect +
   displayZoom，present 同 turn 与每 tick 重写
   `left = rect.left × (visual/zoom)` + `scale(s)`——与 ADR-0010 主 canvas
   同构。P1/P2/P3 一并消解。
2. 手势结束主动 hideDetail —— **未采用**：会破坏 settle `retainDetailOverlay`
   的合法复用路径，且对齐修复后已无必要。
3. settle 复用跳渲染分支强制 hide/present —— **未采用**：方向 1 落地后
   补丁逐帧对齐，永久变体自然消解（不需要在 skip-render 分支加特判）。
4. ✅ P4 新 E2E 契约（2026-10-04 补齐）：
   `tests/e2e/specs/zoom_detail_overlay_geometry.spec.ts`——像素无关帧级
   谓词"补丁可见帧必含于主 canvas 视觉矩形（8px 容差）"，快速缩小 ≥3 档
   + 静止 2s 逐帧采样，含检测器非空转与路径非空转自检；变异校验
   （s=1 冻结公式）确认测试有牙（红："2/43 帧补丁超出页面"，还原后绿）。

## 非问题确认

- 中间帧文字发虚 = 旧位图 CSS 拉伸（ADR-0009 设计内，settle 后消失）；
- ADR-0023 的持续模糊未复现（其修复已在本构建内）；
- ADR-0017 忙环冻结未复现。
