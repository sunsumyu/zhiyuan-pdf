# ADR-0001: 缩放状态单一权威；跨 WASM 边界禁止可注册回调链

日期：2026-08-26　状态：已接受

## 背景

缩放渲染曾长期存在双重状态权威：`ZOOM_STATE.target_zoom`（动画/意图）与 `VIEWER_SESSION.current_zoom`（会话快照）各自存储，靠调用方人肉镜像同步。两次线上事故均源于此：

1. wheel 路径漏调 `set_zoom` 镜像 → settle 渲染用旧 zoom → 位图拉伸不锐化。
2. settle 采用跨 WASM 的可注册回调（`onZoomSettle`），注册发生在模块求值时而 WASM 尚未初始化，异常被 try/catch 吞掉 → 回调槽永远为空 → 最终矢量重绘永不发生。

业界对标（PDF.js 单一 `_currentScaleValue`；Chromium PinchPhase 相位机 + `needs_reraster_`）均采用单一权威 + 拉取式调度，无可注册回调链。

## 决定

1. **ZOOM_STATE 是缩放事实的唯一可写存储**（Zoom Authority，见 CONTEXT.md）。`VIEWER_SESSION.current_zoom` 存储删除，仅在 read 快照中作为派生投影保留字段名（值 = target_zoom）。所有写入经单入口函数。
2. **settle 用信封投递**：RAF 循环构建 FramePlanRequest 并调度出 RenderFrameEnvelope 停泊于 RENDER_LOOP_STATE，由 Rust 直呼固定全局函数敲门交给 TS 渲染循环。推送帧而非推回调。
3. 旧链路整体删除：ON_SETTLE_CALLBACK / on_settle_callback / notify_settle / free_api 导出 onZoomSettle / TS registerZoomSettleCallback 及重试逻辑。

## 后果

- "过期镜像"与"回调未注册"两个故障类别被结构性消灭。
- 未来任何"在 WASM 与 JS 之间加可注册回调"的提议应默认否决，改用：固定全局函数直呼、或数据停泊 + 现有循环拉取。
- 补丁型回归测试（zoom_session_sync / zoom_settle_registration）随补丁退役，替换为不变量测试。

## 否决的替代方案

- 投影式双存储（收窄写点但保留两份存储）：故障类别仍在。
- TS 常驻 idle RAF 轮询 RENDER_STATE：引入常驻循环，违背现有自停设计。
