# pdf-viewer-ui · zoom（缩放编排与 RAF 循环）

> 范围：`crates/pdf-viewer-ui/src/zoom/`（13 个文件，约 1360 行，含测试文件 `authority_tests.rs`）。
> 上游：`pdf_viewer_core::render`（`zoom_state` 值类型 HostZoomState 等、`zoom/animation` 动画插值与 wheel 解析、`zoom_host` tick 核心、`zoom/decision` reknock 决策、`plan_builder` 视口布局）、`crate::present`（plan_builder / present_store 帧计划构建）、`crate::render::render_store`（RENDER_STATE 在飞帧令牌）、`crate::viewer`（viewer_controller::set_zoom 投影、viewer_store 会话）、`crate::host`（command::apply_zoom_selection、layout::sync_host_layout）、`crate::editor::session::render_scene_key`。
> 下游：`application.rs`（会话状态汇总、运行时重置）、`viewer/viewer_controller.rs`（set_zoom 薄委托、文档切换清预览）、`host/`（command / layout / scroll）、`render/free_api.rs`、`present/plan_builder.rs` 与 `render/workflow.rs`（HostZoomState 类型）、`editor/`（editor_controller、host_snapshot、editor_api::helpers、overlay::visual）、`page/context.rs`；TS 侧经 `free_api.rs` 的 wasm 导出接入（onWheelEvent / commitRenderedFrameToQueue / startZoomRafLoop / applyZoomSelection / markRenderedZoom 等，桥接代码见 `src/bridge/`）。

## 职责
持有缩放权威状态 ZOOM_STATE 并维护 ADR-0001 的单一写入入口，编排缩放子系统的全部运行时行为：wheel 虚拟缩放的即时几何反馈、Rust 驱动的 RAF 动画循环、渲染管线提交帧的排队与应用、落定后经固定全局函数敲门 TS 渲染循环（Settle 信封）。纯决策与插值算法均在 core 侧 `render/zoom/`（见 docs/modules/core-render.md），本模块只做状态存取、DOM 写入与 RAF 编排。

## 文件与方法
（可见性列中 wasm 指 `#[wasm_bindgen]` 导出；thread_local! 状态单独成行。）

### `mod.rs` — 模块清单与旧路径兼容 re-export
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `pub mod` ×11 | 挂载 free_api / raf_* / zoom_* 子模块 |
| 私有 | `mod authority_tests` | ADR-0001 不变量测试（仅测试构建） |
| pub | `pub use zoom_controller::{…}` 21 符号 | 兼容旧导入路径 `zoom::zoom_controller::*` |

### `zoom_store.rs` — 缩放权威状态容器：ZOOM_STATE 与会话状态枚举
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `pub use pdf_viewer_core::render::zoom_state::*` | re-export HostZoomState 等缩放值类型 |
| pub | `ZoomSessionState` (enum) | 会话状态机：Idle / Animating / Previewing |
| pub | `ZoomSessionState::as_str()` | 返回三态的英文字符串名 |
| pub | `read_zoom_session_state()` | 由 visual/target 差值导出会话状态 |
| pub | thread_local `ZOOM_STATE: RefCell<HostZoomState>` | 缩放权威：target/visual/last_rendered zoom + visual_layout + preview_host + drawing_delay |
| pub | `read_zoom_state()` | 克隆整个 HostZoomState 快照 |
| pub | `with_zoom_state(f)` | 只读借用访问 ZOOM_STATE |
| pub | `with_zoom_state_mut(f)` | 可变借用访问 ZOOM_STATE |
| pub | `reset_zoom_state(initial_zoom)` | sanitize 后整体重建权威状态 |
| 私有 | `sanitize_zoom(value)` | 非有限/非正数回退 1.0 |

### `zoom_authority.rs` — ADR-0001 单一写入入口（所有函数直访 ZOOM_STATE）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `reset_zoom_runtime(initial_zoom)` | 级联重置 zoom / render / present 三运行时 |
| pub | `read_zoom_state()` | 读权威快照（与 zoom_store 同名重复） |
| pub | `set_target_zoom(target_zoom)` | 写 target_zoom 并复位动画时钟 |
| pub | `set_target_zoom_instant(target_zoom)` | 程序化跳变：visual 立即吸附 target（防 css_scale 反转） |
| pub | `set_target_zoom_authoritative(target_zoom)` | 唯一权威写入入口显式别名（转发 set_target_zoom） |
| pub | `mark_rendered_zoom(rendered_zoom)` | 提交已渲染 zoom 并复位动画时钟 |
| pub | `cancel_drawing_delay()` | wheel 事件时取消绘制延迟计时 |
| pub | `set_visual_layout(display_zoom, left, top)` | sanitize 后写入视觉布局快照 |

### `zoom_controller.rs` — wheel 缩放与预览步进的薄编排层（兼 re-export hub）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `pub use zoom_authority / zoom_frame / zoom_preview::*` | 全量 re-export 保旧路径 |
| pub | `WheelZoomHostRequest` / `WheelZoomHostResult` (struct) | wheel 编排入参/出参（wheel 请求 + 帧计划 + 渲染决策） |
| pub | `PreviewHostStepRequest` / `PreviewHostStepResult` (struct) | 预览步进入参/出参（帧计划 + 预览帧 + tick 决策） |
| pub | `execute_wheel_zoom(request)` | wheel→zoom 结果 + 帧计划 + 渲染决策并置预览标志 |
| pub | `step_preview_host(request)` | 步进预览帧并解析预览 tick 决策置标志 |
| pub | `resolve_wheel_zoom(request)` | 解析 wheel 增量写权威 target 并投影 set_zoom |
| pub | `tick_zoom_state(input)` | 在 ZOOM_STATE 上驱动 core tick 状态机 |
| pub | `clear_preview_host_with_anchor(do_clear_anchor)` | 清预览宿主（anchor 形参已废弃） |
| pub | `settle_zoom_preview_at_target()` | visual 吸附 target 并清预览状态 |
| pub | `reset_zoom_preview_host(target_zoom)` | 标记已渲染 zoom 并清预览宿主 |

### `zoom_preview.rs` — 预览宿主标志位读写（仅本地状态，无 transform）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `clear_zoom_preview_host_state()` | preview_host 整体复位 |
| pub | `clear_preview_settle_state()` | visual 吸附 target 并复位 preview_host |
| pub | `set_wheel_render_pending(pending)` | 写 wheel 渲染挂起标志 |
| pub | `set_preview_active(active)` | 写预览激活标志 |
| pub | `set_cancel_pending_render(cancel)` | 写取消挂起渲染标志 |
| pub | `take_cancel_pending_render()` | 取走并清零取消渲染标志 |
| pub | `is_preview_active()` | 读预览激活标志 |
| pub | `is_wheel_render_pending()` | 读 wheel 渲染挂起标志 |

### `zoom_frame.rs` — 动画帧步进与旧提交帧单槽暂存
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `step_zoom_animation()` | 无时间戳推进一步动画状态 |
| pub | `step_zoom_frame_plan(request)` | 推进动画→按 visual 构建预览帧计划（联动 viewer/present/场景键） |
| pub | `queue_committed_frame(frame_plan)` | 暂存待提交帧到 preview_host 单槽 |
| pub | `take_ready_committed_frame()` | 差值 <0.001 落定时才取走待提交帧 |

### `raf_loop.rs` — Rust 驱动的缩放 RAF 循环（虚拟缩放 + 落定敲门）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | thread_local `RAF_HANDLE` / `RAF_CLOSURE` / `LAST_PREVIEW_KNOCK` | RAF 句柄（存在即在跑）/ 闭包 / 上次动画中敲门时间戳 |
| 私有 | `SETTLE_DRAWING_DELAY_MS` (const) | 落定后绘制延迟 30ms |
| pub | `start_zoom_raf_loop()` | 幂等启动循环；隐藏 raster 兄弟面（ADR-0002 I3） |
| pub | `stop_zoom_raf_loop()` | 摘除 RAF 句柄/闭包并清 DOM 缓存 |
| pub | `is_raf_loop_running()` | 句柄存在即循环在跑 |
| pub | `WheelEventInput` / `WheelEventOutput` (struct) | wheel 事件 TS 原始 DOM 入参 / 双 zoom 出参 |
| pub | `pub use raf_committed::{commit_rendered_frame, CommittedFrame}` | 提交帧入口与类型的 re-export |
| pub | `on_wheel_event(input)` | 虚拟缩放：解析 wheel 写权威并立即按 target 直写容器几何 |
| pub | `ensure_raf_loop_after_wheel()` | wheel 后保证 RAF 循环续跑 |
| pub(super) | `GESTURE_THRESHOLD` (const) | 手势判定阈值 0.001（raf_committed 共用） |
| 私有 | `schedule_next_frame()` | 请求下一帧并保存句柄与闭包 |
| 私有 | `tick(timestamp_ms)` | 五步循环：推进动画 / 动画中 reknock / 应用提交帧 / 落定绘制延迟 / 续排或自停 |

### `raf_dispatch.rs` — Settle 信封敲门（Rust 直呼固定全局函数）
| 可见性 | 方法 | 功能 |
|---|---|---|
| 私有 | `DRAIN_KNOCK_GLOBAL` (const) | 固定全局函数名 `__pdfDrainPendingRenderFrame` |
| pub | `dispatch_settle_envelope()` | 落定后敲门 TS 渲染循环取走停泊信封 |
| 私有 | `knock_render_loop()` | Reflect 取全局函数并无参调用（非注册回调） |

### `raf_committed.rs` — 提交帧队列与应用（RAF 循环消费端）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `CommittedFrame` (struct) | 提交帧几何：display/render zoom + host 尺寸 + content/scroll 偏移 |
| pub | thread_local `COMMITTED_FRAME_QUEUE: RefCell<Vec<CommittedFrame>>` | 待应用提交帧队列 |
| pub | `commit_rendered_frame(frame)` | 循环未跑则立即应用，否则入队 |
| pub | `pop_committed_frame()` | 弹出一帧（Vec::pop，LIFO） |
| pub | `apply_committed_frame(frame)` | 写 last_rendered/visual_layout 并直写容器尺寸与滚动；手势期跳过几何写 |

### `raf_dom_cache.rs` — DOM 元素引用缓存（每循环会话解析一次）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(super) | `DomCache` (struct) | container / scroll_container / raster 三元素引用 |
| pub(super) | `VECTOR_CONTAINER_ID` / `SCROLL_CONTAINER_ID` / `RASTER_TARGET_ID` (const) | 与 TS 桥约定的三个元素 ID |
| pub(super) | thread_local `DOM_CACHE: RefCell<Option<DomCache>>` | 每循环会话一次的 DOM 引用缓存 |
| pub(super) | `with_dom_cache(f)` | 只读访问缓存 |
| pub(super) | `with_dom_cache_mut(f)` | 可变访问（预留失效缝，未接线） |
| pub(super) | `clear_dom_cache()` | 清空缓存 |
| 私有 | `get_element_by_id(id)` | document.getElementById 薄封装 |
| pub(super) | `init_dom_cache()` | 按 ID 解析三元素并一次性设 transform-origin |

### `raf_settle.rs` — 落定清理 RAF（一次性自移除）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(super) | thread_local `SETTLE_CLEANUP: RefCell<Option<(i32, JsValue)>>` | 清理帧的句柄与闭包 |
| pub(super) | `cancel_settle_cleanup()` | 取消已排清理帧并清 thread_local |
| pub(super) | `schedule_settle_cleanup()` | 排一帧清理（ADR-0006 后无 transform 可清，仅收尾） |

### `free_api.rs` — zoom 域自由 wasm 导出（原 zoom_api.rs，TS 桥保留面）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `ZoomSnapshot` (struct) | TS 桥单读快照 DTO（currentZoom 恒为 target 投影） |
| wasm | `resolveWheelZoom` | TS 传 wheel 请求，返回解析结果 |
| wasm | `resetZoomState` | 转发 viewer_controller::reset_zoom_view |
| wasm | `readZoomState` | 序列化 HostZoomState |
| wasm | `getZoomState` | 已废弃别名，转发 readZoomState |
| wasm | `setTargetZoom` | 写 target_zoom |
| wasm | `setTargetZoomInstant` | 程序化跳变（visual 吸附 target） |
| wasm | `markRenderedZoom` | 提交已渲染 zoom |
| wasm | `applyZoomSelection` | 转发 host::command 缩放选择命令 |
| wasm | `cancelDrawingDelay` | 取消绘制延迟计时 |
| wasm | `takeCancelPendingRender` | 取走取消渲染标志 |
| wasm | `syncHostLayout` | 转发 host::layout 布局同步 |
| wasm | `readZoomSnapshot` | 组装单读快照避免多次往返见中间态 |
| wasm | `tickZoomState` | 每帧 tick：TS 传 DOM 读数，回 DomOps/AsyncOps |
| wasm | `startZoomRafLoop` / `stopZoomRafLoop` / `isZoomRafLoopRunning` | Rust RAF 循环启停与状态查询 |
| wasm | `onWheelEvent` | wheel 总入口：处理后保证循环续跑 |
| wasm | `commitRenderedFrameToQueue` | 渲染管线推帧入口（解析失败静默丢弃） |

### `authority_tests.rs` — ADR-0001 不变量测试（仅列覆盖点）
| 可见性 | 方法 | 功能 |
|---|---|---|
| 测试 | `i1_session_snapshot_current_zoom_is_projection_of_target` | 快照 current_zoom 恒等于权威 target |
| 测试 | `i2_single_entry_write_updates_projection_immediately` | 唯一入口写后投影立即跟随 + 非法值 sanitize 1.0 |
| 测试 | `i3_wheel_path_no_longer_mirrors_into_session_store` | wheel 路径无向 session 存储的镜像写 |
| 测试 | `i4_instant_zoom_snaps_visual_and_commit_lands_at_scale_one` | 程序化跳变 visual 吸附 + 提交后无缩放反转 |

## 疑点
- 死代码候选：`execute_wheel_zoom` / `step_preview_host`（含 4 个 DTO struct）、`step_zoom_animation`、`queue_committed_frame` / `take_ready_committed_frame`、`set_cancel_pending_render`、`reset_zoom_preview_host`、`ZoomSessionState::as_str` 在 crate 内（含 TS 桥）均无调用者，仅 mod.rs 兼容 re-export 或零引用；zoom_store 头注称 ZoomController 会话句柄已删，这批疑似旧预览路径残留。
- 疑似重复：提交帧暂存有两套——`zoom_frame` 走 `preview_host.pending_committed_frame` 单槽，`raf_committed` 另有 `COMMITTED_FRAME_QUEUE` Vec 队列，活跃路径只有后者；`zoom_authority::read_zoom_state` 与 `zoom_store::read_zoom_state` 同实现重复。
- `read_zoom_session_state` 头注声称 Previewing 态（依赖已废除的 preview_transform，ADR-0006），实现只判 visual/target 差值，Previewing 分支永不可达。
- `pop_committed_frame` 用 `Vec::pop` 取队尾（LIFO），多帧积压时旧帧饿死，与"队列"命名不符（当前单帧场景无碍）。
- 命名与公约：`raf_*` 三文件游离于 `*_store` / `*_controller` / `*_api` 公约之外；`raf_committed.rs` 兼具存储（队列）与应用（DOM 写入）两职；`clear_preview_host_with_anchor` 的 `do_clear_anchor` 形参已死（ADR-0007 后无 anchor），保留仅为兼容；`raf_dom_cache::with_dom_cache_mut` 标注 dead_code 预留。
- 与 ADR 关系：未见冲突——权威写收敛有 authority_tests 覆盖（ADR-0001）、无 CSS transform 写入（ADR-0006，settle 清理已只剩标志位收尾）、raster/矢量单活跃面切换（ADR-0002 I3）。落定绘制延迟常量在本模块（30ms）与 core `zoom_tick.rs`（30ms）双份并存、当前数值一致（core-render.md 所记 50/30 漂移在工作区已收敛）。
