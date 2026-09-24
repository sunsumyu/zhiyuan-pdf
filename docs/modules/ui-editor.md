# pdf-viewer-ui · editor（编辑会话编排）

> 范围：`crates/pdf-viewer-ui/src/editor/`（36 个文件，约 5800 行，含子目录 `editor_api/`、`format/`、`orchestrator/`、`overlay/`、`session/`）。
> 上游：`pdf-viewer-core::edit`（领域库：bridge / active_target / paragraph_overlay / replacement_* / engine_state / debug_trace / editor_types / caret_geometry 等，多数经 1–2 行 shim 直通，详见 [core-edit.md](core-edit.md)）；crate 内 `page/page_store`、`zoom/*`、`present/*`、`render/*`、`document/*`、`find/host_find_store`、`viewer/viewer_store`、`ui_state_store`、`style_mapper`、`events`、`common/sanitize`。
> 下游：crate 内 `application.rs`（读会话状态机）、`api.rs`（`pub use EditorSession` 为唯一 WASM 出口）、`document/document_api`（replace_pipeline）、`document/patch_persistence`（reconcile_numbering_patches）、`render/canvas_overlay`（paragraph_overlay / debug_trace / replacement_region）、`ui_state_store`（ActiveEditorTarget）、`viewer/viewer_controller`（close/reset）；TS 侧 `src/bridge/editor/*`（EditorSession 全部方法）、`src/bridge/find/find_facade.ts`（searchFacadeReplace / BatchReplace）。
> 深入阅读：[editor-render-architecture.md](../editor-render-architecture.md)、[edit-save-architecture.md](../edit-save-architecture.md)、[editor-api-architecture-proposal.md](../editor-api-architecture-proposal.md)。

## 职责
编排 PDF 文本编辑会话的完整生命周期：WASM 会话门面（EditorSession 状态机）、会话活状态容器、点击激活与输入命令管线、格式化与 caret 几何、提交/替换的跨域副作用编排、覆盖层数据收集与 caret 绘制。纯领域计算（目标解析、文本增删、草稿排版）在 core 的 edit/ 模块；本目录以 thread_local 状态容器、WASM 边界与跨域编排为主，含少量 canvas 测量/绘制。

## 文件与方法
（各文件内联 `#[cfg(test)] mod tests` 不列入，测试覆盖点在对应小节末标注；wasm = `#[wasm_bindgen]` 导出给 TS。）

### `mod.rs` — 模块清单与 core 直通 re-export（保旧路径兼容）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod` × 17 + `pub mod` × 4 子目录 | 声明全部顶层与子目录模块 |
| pub | `pub use pdf_viewer_core::edit::{document_edit_ops, document_plan, document_runtime, draft_layout, edit_target, source_identity, source_runs, source_text}` | 旧 draft/、source/ shim 目录删除后的 core 直通别名 |
| pub | `pub use pdf_viewer_core::geometry::source_geometry` | 同上 |
| pub | `pub use overlay::{navigation, paragraph_overlay, projection, visual}` | overlay 子模块再导出保旧路径 |
| pub | `pub use format::{list_format, text_geometry}` | format 子模块再导出保旧路径 |

### `editor_api/` — WASM 会话边界：EditorSession 状态机门面（TS 唯一编辑入口）

#### `editor_api/mod.rs` — 子模块拆分说明与 EditorSession 再导出
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `mod types / session / helpers` | 按类型/方法/内部 helper 三分（closeout #06） |
| pub | `pub use types::EditorSession` | 会话对象唯一公开路径 |

#### `editor_api/types.rs` — 请求 DTO 与 EditorSession 结构定义
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(crate) | `HitTestRequest` (struct) | 命中测试请求：客户点 + 宿主参考矩形 + 页尺寸 |
| pub(crate) | `OpenBlockRequest` (struct) | 打开块请求：block_id + 点击坐标 + 回退页点 |
| pub(crate) | `MoveCaretRequest` (struct) | 移动 caret 请求：客户点 + 参考矩形 |
| pub(crate) | `CommitRequest` (struct) | 提交请求：草稿文本 + caret |
| wasm | `EditorSession` (struct) | 零字段 WASM 会话对象（`#[wasm_bindgen]` 导出） |
| pub(crate) | `Default::default()` | 默认即 `new()` |

#### `editor_api/session.rs` — EditorSession impl：全部 WASM 方法（1024 行，模块内最大文件）
| 可见性 | 方法 | 功能 |
|---|---|---|
| wasm | `new()` (constructor) | 构造空会话对象 |
| wasm | `begin()` | 进入 Editing 态，启用编辑模式并返回可编辑块列表 |
| wasm | `hit_test(request)` | 客户点命中测试，返回命中块 id 与页坐标 |
| wasm | `open_block(request)` | 打开块编辑；已在块内则先 commit 旧块再切换 |
| wasm | `move_caret(request)` | 块内移动 caret；点空白返回空 data 提示关闭 |
| wasm | `close_block()` | 关闭块回 Viewing；有 pending 编辑先强制 commit |
| wasm | `commit(request)` | 提交活动块编辑并退出编辑态（重入拒绝） |
| wasm | `end()` | 结束会话并自动 commit 未决块（save & exit 出口） |
| wasm | `discard()` | 丢弃全部编辑直接回 Viewing（Saving 态报错） |
| wasm | `read_snapshot(display_zoom)` | 读会话快照：状态/块 id/草稿/caret/未保存标志 |
| wasm | `get_snapshot(display_zoom)` | readSnapshot 的 deprecated 别名 |
| wasm | `is_active()` | 是否处于 Editing / EditingBlock 态 |
| wasm | `has_unsaved_changes()` | 是否存在可持久化补丁 |
| wasm | `sync_input(request)` | 同步 textarea 文本与 caret 进 Rust live state |
| wasm | `apply_command(request)` | 分发输入命令并返回草稿文本 + caret |
| wasm | `set_edit_mode(enabled)` | 开关编辑模式并同步状态机 |
| wasm | `read_legacy_snapshot(display_zoom)` | 读完整旧版快照（shell 投影/targets/DOM 坐标） |
| wasm | `paint_canvas(canvas, zoom, text, caret)` | 调 Rust glyph 后端绘制编辑器覆盖画布 |
| wasm | `utf16_to_char_index(text, offset)` | UTF-16 偏移转 Rust char 索引 |
| wasm | `char_to_utf16_offset(text, index)` | char 索引转 UTF-16 偏移 |
| wasm | `has_session_changes()` | 活动编辑器是否有未提交会话变更 |
| wasm | `open_region(request)` | 打开区域编辑器（文档审阅流） |
| wasm | `set_display_zoom(zoom)` | 记录最近显示缩放 |
| wasm | `read_diagnostics()` | 读活动编辑器诊断信息 |
| wasm | `save_session(path, page_index)` | 关闭编辑并将补丁写盘（async） |
| wasm | `insert_text(text)` | caret 处插入文本 |
| wasm | `delete_text(direction)` | 按方向删除文本 |
| wasm | `apply_format(action)` | 对活动块应用格式化动作 |
| wasm | `undo()` / `redo()` | 撤销 / 重做活动块文本编辑 |
| wasm | `can_undo()` / `can_redo()` | 是否可撤销 / 重做 |
| wasm | `read_text_blocks(page_index)` | 读指定页可编辑块（仅支持当前活动页） |
| wasm | `get_text_blocks(page_index)` | readTextBlocks 的 deprecated 别名 |
| wasm | `read_format_state()` | 读活动编辑器格式状态 |
| wasm | `get_format_state()` | readFormatState 的 deprecated 别名 |
| wasm | `on_state_change(callback)` | 注册状态迁移回调（null 注销） |
| wasm | `on_change(callback)` | 注册任意会话变更回调（null 注销） |
| wasm | `set_caret(char_index)` | 直接设置 caret 索引 |
| wasm | `set_selection(start, end)` / `select_all()` / `read_selection()` | 选区设置 / 全选 / 读取 |
| wasm | `cut()` / `copy()` / `paste(text)` 等 14 个 | 剪贴板、坐标换算、块增删、补丁导入导出等均为 NotImplemented 桩 |

#### `editor_api/helpers.rs` — 会话内部 helper（不导出 JS）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `EditorSession::commit_draft_internal()` | 提交当前草稿（转调 orchestrator::commit） |
| pub(crate) | `build_frame_request()` | 由 zoom/viewer 状态组装默认 FramePlanRequest |
| pub(crate) | `resolve_target_at_page_point(page_x, page_y)` | 4px 容差直接命中段落目标（无最近邻回退） |
| pub(crate) | `collect_text_blocks()` | 收集当前页可编辑块为 TextBlockInfo 列表 |

### `editor_store.rs` — 前端会话状态机（Viewing/Editing/EditingBlock/Saving）
thread_local 状态：`SESSION_STATE`（Cell，当前状态）、`ACTIVE_BLOCK_ID`（RefCell，活动块 id）、`STATE_CHANGE_CB` / `CHANGE_CB`（wasm32 回调单槽）。

| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `read_state()` | 读当前会话状态 |
| pub | `set_state(state)` | 写状态并按需触发回调与 EventBus 事件 |
| pub | `read_active_block_id()` / `set_active_block_id(id)` | 读写活动块 id，变更时通知 |
| wasm | `set_state_change_callback(cb)` | 安装状态迁移回调（§14.7） |
| wasm | `set_change_callback(cb)` | 安装任意会话变更回调 |
| 私有 | `notify_state_change(state)` | 单槽回调 + EventBus EDITOR_STATE_CHANGE 事件 |
| 私有 | `notify_change()` | 单槽回调 + EventBus EDITOR_CHANGE 事件 |
| 私有 | `state_camel_case(state)` | 状态枚举转 camelCase 字符串 |
| pub | `transition_to_editing()` | Viewing → Editing |
| pub | `transition_to_editing_block(id)` | Editing → EditingBlock 并记活动块 |
| pub | `transition_to_viewing()` | EditingBlock → Viewing（close/commit/discard） |
| pub | `transition_switch_block(id)` | 块间切换（状态保持 EditingBlock） |
| pub | `transition_to_saving()` / `transition_save_complete()` | 进入 Saving / 保存完成回 Viewing |
| 宏 | `guard_state!(expected, fn_name)` | 状态守卫：不匹配时返回 InvalidState 错误响应 |

### `editor_types.rs` — 响应 DTO 组装（core editor_types 直通 + JsValue 转换）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use pdf_viewer_core::edit::editor_types::*` | SessionState / EditorError / EditorResponse 及结果 DTO 直通 |
| 私有 | `response_to_js(resp)` | EditorResponse 序列化为 JsValue |
| pub | `ok_response(data, render)` | 组装带载荷成功响应 |
| pub | `ok_empty(render)` | 组装无载荷成功响应 |
| pub | `err_response(error)` | 组装错误响应 |
| pub | `parse_request::<T>(js, method)` | 反序列化请求，失败即产出 err_response |

### `session/` — 编辑会话活状态（EDITOR_MODE_STATE 容器与撤销历史）

#### `session/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod history` / `pub mod session` + `pub use session::*` | 保留 session::session 嵌套并平铺导出 |

#### `session/session.rs` — 编辑模式状态容器：开关、活动段落、live state、选区、undo 入口
thread_local 状态：`EDITOR_MODE_STATE`（RefCell<EditorModeState>，编辑会话唯一活状态容器）。

| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorModeState` (struct) | 编辑开关 + 活动段落 id + live_state + 历史 |
| pub | `ActiveEditorInputSyncResult` (struct) | 输入同步结果 DTO（text/caret/scene 变更标志） |
| pub | `pub use core::edit::active_target::ActiveEditorTarget` | 目标数据结构直通 |
| thread_local | `EDITOR_MODE_STATE` | 全局编辑模式状态容器 |
| pub | `reset_editor_mode()` | 重置状态容器为默认 |
| pub | `is_text_edit_enabled()` | 读文本编辑开关 |
| pub | `set_text_edit_enabled(enabled)` | 写开关；关闭时清 live_state（残留则告警） |
| pub | `active_edit_paragraph_id()` | 读活动段落 id |
| pub | `set_active_edit_paragraph(id)` | 写活动段落（None 时清 live_state） |
| pub | `active_editor_state()` / `active_editor_target()` | 读 live 状态 / 其目标 |
| pub | `open_paragraph_editor(id, target)` | 打开编辑：回放已存补丁的文本/样式/对齐/marker，置 clean 并清历史 |
| pub | `close_active_editor()` | 清空活动段落、live state 与历史 |
| pub | `active_editor_draft_text()` | 读当前草稿文本 |
| pub | `active_editor_has_session_changes()` | live state 是否有未提交会话变更 |
| pub | `active_editor_caret_index()` | 读当前 caret |
| pub | `set_active_editor_caret_index(index)` | 设置 caret（越界归一化，带调试事件） |
| pub | `set_active_editor_selection(s, e)` / `clear_active_editor_selection()` | 设置 / 清除选区 |
| pub | `active_editor_selection()` | 读选区（起止 + 文本） |
| pub | `sync_active_editor_input(text, caret)` | 文本 + caret 同步进 live state（变更前推 undo 快照） |
| pub | `undo_active_editor()` / `redo_active_editor()` | 撤销 / 重做并整体替换 live state |
| pub | `can_undo()` / `can_redo()` | 历史栈是否可撤销 / 重做 |
| pub | `render_scene_key()` | 组合文档/补丁/编辑场景版本生成渲染缓存键 |
| tests | `starts_disabled` | 覆盖：开关初始禁用并可启用 |

#### `session/history.rs` — 本地撤销/重做历史（时间窗合并）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `HistorySnapshot` (struct) | live state 快照 + 时间戳 |
| pub | `LocalEditHistory` (struct) | undo / redo 双栈 |
| pub | `new()` / `clear()` | 构造 / 清空双栈 |
| pub | `push_snapshot(state)` | 1 秒内且文本差 ≤2 字符且样式未变则合并；栈上限 100，清 redo |
| pub | `undo(current)` / `redo(current)` | 弹栈回退 / 前进，当前态压入对侧栈 |
| pub | `can_undo()` / `can_redo()` | 对应栈非空判定 |

### `mode.rs` — 编辑模式访问的透传薄层（1:1 转调 session/）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `read_active_edit_paragraph()` | 读活动段落 id（透传） |
| pub | `read_active_editor_target()` | 读活动编辑目标（透传） |
| pub | `read_active_editor_state()` | 读 live 状态（透传） |
| pub | `is_text_edit_mode_enabled()` | 读编辑开关（透传） |
| pub | `set_text_edit_mode_enabled(enabled)` | 写编辑开关（透传） |
| pub | `set_active_edit_paragraph(id)` | 写活动段落（透传） |
| pub | `close_active_editor()` | 关闭编辑器并返回此前是否有活动 |
| pub | `reset_editor_mode()` | 重置编辑模式状态（透传） |

### `host_mode.rs` — 文本编辑模式开关（带强制 commit 守卫）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ToggleEditorModeResult` (struct) | enabled + changed 结果 DTO |
| pub | `toggle_text_edit_mode()` | 翻转编辑模式；关闭前强制 commit pending 编辑 |
| pub | `set_text_edit_mode(enabled)` | 显式设置；enabled→disabled 前强制 commit |

### `host_runtime.rs` — 编辑器桥接运行态（提交重入保护 + 显示缩放）
thread_local 状态：`EDITOR_HOST_RUNTIME_STATE`（RefCell<EditorHostRuntimeState>）。

| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorHostRuntimeState` (struct) | committing 标志 + last_display_zoom |
| thread_local | `EDITOR_HOST_RUNTIME_STATE` | 全局桥接运行态容器 |
| pub | `read_state()` | 读运行态快照 |
| pub | `reset_state()` | 重置为默认 |
| pub | `set_display_zoom(zoom)` | 记录最近显示缩放（非法值回落 1.0） |
| pub | `begin_commit()` | 进入提交临界区（重入返回 false） |
| pub | `finish_commit()` | 退出提交临界区 |

### `activation.rs` — 点击激活管线：客户点 → 页点 → 目标命中 → 打开/关闭
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `OpenEditorAtClientPointRequest` (struct) | 打开请求 DTO（serde 双命名别名兼容） |
| pub | `MoveCaretToClientPointRequest` (struct) | 移动 caret 请求 DTO |
| pub | `SaveEditorSessionResult` (struct) | 保存结果 DTO（saved + 是否有补丁 + 错误） |
| 私有 | `resolve_page_point_from_client(...)` | 客户点→页点并 clamp 进段落 shell bbox |
| 私有 | `resolve_shell_center_page_point(bbox)` | 取 shell bbox 中心作为回退点 |
| 私有 | `point_in_bbox(x, y, bbox, tol)` | 带容差的点包含判定 |
| 私有 | `resolve_target_at_page_point(x, y)` | 4px 容差命中段落目标；点空白不回退近邻 |
| pub | `activate_editor_from_client_point(request)` | 激活入口：id 为空按点击命中；未命中则 commit+close 退出编辑 |
| pub | `activate_region_editor(page, region, kind, text)` | 区域编辑器激活（转调 open_region_editor） |
| pub | `move_caret_to_client_point(request)` | 客户点→shell 局部点→caret 索引并设置 |
| pub async | `save_editor_session(path, page_index)` | 关闭编辑器并将可持久化补丁写盘 |

### `editor_controller.rs` — 编辑器核心控制：打开、补丁构建、caret、格式转发
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorVisibilityAction` (struct) | changed + request_visibility_render 动作结果 |
| pub | `pub use editor_format::{…}` | 格式状态与动作类型再导出 |
| 私有 | `summarize_object_ids(ids)` | 调试摘要：取前 6 个对象 id 拼串 |
| pub | `collect_paragraph_targets()` | 收集段落交互目标并序列化为 JsValue |
| pub | `open_editor_at_page_point(id, x, y)` | 打开段落编辑：先 commit 旧块，统一首点/后续点击 caret 解析路径 |
| pub | `build_region_text_patch(page, region, kind, orig, new)` | 构建区域文本补丁（转调 workflow） |
| pub | `open_region_editor(page, region, kind, orig)` | 打开区域编辑器（校验 kind、先 commit 旧块） |
| pub | `build_active_editor_patch(new_text)` | 由 live state 构建提交补丁：样式/对齐/行高/marker/list 转换、快照、noop 抑制 |
| 私有 | `patch_is_noop(state, patch)` | 文本/样式/对齐/行高/marker 全未变则判 noop |
| pub | `find_paragraph_shell_bbox(id)` | 解析段落 shell 包围盒 |
| pub | `set_editor_caret(index)` | 设置活动编辑器 caret |
| pub | `sync_editor_input(text, caret)` | 同步输入到 live state |

### `workflow.rs` — PageState → 编辑目标/补丁的组装层（core bridge 薄封装）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `build_paragraph_interaction_targets(page_state, enabled)` | 收集交互目标序列化；未启用返回 null |
| pub | `open_paragraph_editor(page_state, id, x, y, zoom)` | 由 paint plan 构建活动编辑目标（zoom 参数当前未用） |
| pub | `resolve_paragraph_shell_bbox(page_state, id)` | 解析段落 shell 包围盒 |
| pub | `build_paragraph_patch(page_state, id, new_text)` | 构建段落文本替换补丁 |
| pub | `build_region_text_patch(page_state, page, region, kind, orig, new)` | 区域补丁：解析区域目标并回填 target_indices |
| pub | `build_active_editor_patch(page_state, active_id, new_text)` | 活动段落补丁构建（当前无调用者） |

### `command.rs` — 编辑输入命令分发（导航/插入/退格/前删）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorInputCommand` (enum) | Navigation / InsertText / DeleteBackward / DeleteForward |
| 私有 | `command_name(command)` | 命令转调试字符串 |
| 私有 | `effective_editor_state(host_text, host_caret)` | 取 live state；忽略 textarea 快照（Rust 为唯一事实源） |
| pub | `apply_editor_input_command(command)` | 无宿主输入时应用命令 |
| pub | `apply_input_with_host(command, text, caret)` | 分发四类命令：导航走 navigation，插入/删除走 core 纯操作后 sync |

### `editor_format.rs` — 活动编辑器格式化（读写 EDITOR_MODE_STATE.live_state）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ActiveEditorFormatState` (struct) | 当前格式状态 DTO（粗体/字号/对齐/列表等） |
| pub | `EditorFormatAction` (enum) | 13 种格式动作（切换/设置/步进） |
| 私有 | `build_active_editor_format_state(state, changed)` | live state → 格式状态 DTO |
| 私有 | `resolve_font_size_step(current, increase)` | 按 14 档字号阶梯取上/下一档 |
| 私有 | `parse_alignment(value)` / `parse_list_kind(value)` | 字符串解析为 LayoutAlignment / ListMarkerKind |
| pub | `toggle_active_editor_bold/italic/underline()` | 全文切换粗体 / 斜体 / 下划线 |
| pub | `set_active_editor_color/font_family/font_size/char_spacing/line_height(…)` | 设置对应格式属性 |
| pub | `step_active_editor_font_size(increase)` | 字号沿阶梯步进 |
| pub | `set_active_editor_paragraph_mode(mode)` | 设置段落模式 |
| pub | `set_active_editor_alignment(alignment)` | 设置对齐（解析失败不变更） |
| pub | `set_active_editor_list_kind(kind)` | 设置列表类型 |
| pub | `active_editor_format_state()` | 读当前格式状态 |
| pub | `apply_active_editor_format_action(action)` | 格式动作统一分发入口 |

### `format/` — 格式化辅助：列表 marker 编排与 caret 几何

#### `format/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod list_format / text_geometry` | 声明两个子模块 |

#### `format/list_format.rs` — 列表 marker 覆盖计算与编号补丁重排
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `EffectiveListState` (struct) | 生效列表类型 + marker 文本 |
| 私有 | `ParagraphListContext` (struct) | 段落列表上下文（base id + 生效态 + 源 marker） |
| pub | `resolve_active_marker_text(active_state, page_state)` | 解析活动段落应显示的 marker 文本 |
| pub | `collect_marker_overrides(plan, active_state)` | 按页面段落顺序计算全部 marker 覆盖表 |
| pub | `reconcile_numbering_patches(plan, vm, patches)` | 提交前重排编号补丁：更新/派生缺失的编号 marker |
| 私有 | `build_numbering_override_map(contexts)` | 编号序列推进与符号/无 marker 覆盖计算 |
| 私有 | `build_paragraph_list_context(state, paragraph, active)` | 活动态→已存补丁→源文本三级解析生效列表态 |
| 私有 | `resolve_symbolic_marker_text(effective, source)` | 符号 marker 文本兜底（默认 •） |
| 私有 | `collect_ordered_page_paragraphs(plan)` | 按 top/left/id 排序页面段落 |
| 私有 | `resolve_patch_for_base_paragraph(state, id)` | 查找段落对应已存补丁 |

#### `format/text_geometry.rs` — caret 几何：canvas 测量 + core 纯计算直通
thread_local 状态：函数内 `MULTI_COUNT` / `MW_COUNT`（诊断计数器，仅限前 5/10 次测量输出 trace）。

| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::text::caret_geometry::{…}` | 纯计算 caret API（行/导航/索引解析）直通 |
| pub | `measure_editor_layout_text_width(ctx, text, run)` | canvas measureText + char_origins/scale_x/char_spacing 修正的宽度 |
| 私有 | `create_measure_context()` | 创建屏外 canvas 2d 测量上下文 |
| 私有 | `dedupe_caret_stops_local(line)` | 去重相邻 caret stop（≤0.5px） |
| 私有 | `convert_render_plan_caret_lines(plan)` | 草稿渲染计划 → CaretLine 列表 |
| 私有 | `build_unified_draft_caret_lines(target, text)` | 统一草稿 caret stop 线构造（core 排版 + canvas 测量） |
| 私有 | `resolve_caret_index_for_draft_point(...)` | 由草稿线解析点击 caret 索引（无测量上下文回退 source text-plan） |
| 私有 | `build_draft_caret_lines(target, text)` | build_unified_draft_caret_lines 别名 |
| 私有 | `resolve_caret_visual_from_draft(target, text, index)` | caret 索引 → 可视位置（行尾兜底） |
| pub | `active_caret_visual(target, text, index)` | 公开 caret 可视位置解析 |
| pub | `move_caret_by_key(target, text, caret, key)` | 方向/Home/End 导航键移动 caret |
| pub | `active_caret_index_at_page_point(target, text, x, y)` | 页点→caret 索引（body 左侧恒 0） |
| pub | `active_caret_index_at_shell_point(target, text, x, y)` | shell 局部点→页点→caret 索引 |

### `engine_state.rs` — core re-export shim（LiveEditorParagraphState）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use pdf_viewer_core::edit::engine_state::*` | 单行直通，保旧路径 |

### `debug_trace.rs` — core re-export shim（编辑调试事件环形缓冲）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use pdf_viewer_core::edit::debug_trace::*` | 单行直通，保旧路径 |

### `replacement_region.rs` — core re-export shim（替换区域）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use pdf_viewer_core::edit::replacement_region::*` | 两行直通，保旧路径 |

### `replacement_snapshot.rs` — core re-export shim（替换快照）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use pdf_viewer_core::edit::replacement_snapshot::*` | 单行直通，保旧路径 |

### `orchestrator/` — 跨域副作用编排（模块文档自述：非纯编辑逻辑）
（mod.rs 内含诚实声明表：commit→document/state_manager；render_transaction→present；replace_pipeline→state_manager/document；并声明 overlay 内 4 处 apply_patch_with_history 为已知例外。）

#### `orchestrator/mod.rs` — 子模块清单与跨域副作用声明表
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod commit / render_transaction / replace_pipeline` | 声明三个编排子模块 |

#### `orchestrator/commit.rs` — 提交持久化（document 域副作用）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `commit_pending_edit_if_any()` | 有未提交 live state 时强制 commit（所有退出路径的统一入口） |
| pub | `commit_active_editor_text(new_text)` | 构建补丁→记忆替换目标→apply_document_patch_direct→关闭编辑器 |

#### `orchestrator/render_transaction.rs` — 编辑事务：编辑动作 + 渲染帧调度决策
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorRenderTransactionResult` (struct) | 事务结果：changed + render_frame |
| pub | `EditorInputRenderTransactionResult` (struct) | 输入事务结果：text/caret/scene 变更 + render_frame |
| 私有 | `schedule_editor_render(request, should)` | 默认 reason 的渲染帧调度 |
| 私有 | `schedule_editor_render_with_reason(...)` | 需要时带 reason 调度渲染帧 |
| pub | `open_editor_tx(request, frame)` | 打开编辑器事务（有意不调度帧，由 JS 主导渲染） |
| pub | `open_region_editor_tx(page, region, kind, text, frame)` | 区域打开事务（同上不调度帧） |
| pub | `sync_input_tx(text, caret, frame)` | 输入同步事务 |
| pub | `apply_input_tx(command, frame)` | 输入命令事务 |
| pub | `apply_host_input_tx(command, host_text, host_caret, frame)` | 带宿主输入的命令事务（当前无调用者） |
| pub | `commit_editor_tx(text, caret, frame)` | 提交事务：clean session 跳过宿主同步；不调度帧 |
| pub | `commit_editor_silent_tx(text, caret)` | 静默提交事务（当前无调用者） |
| pub | `close_editor_tx(frame)` | 关闭事务：有草稿强制 commit 防丢失 |
| 私有 | `format_render_tx(frame, changed)` | 格式变更后调度渲染帧 |
| pub | `apply_format_action_tx(action, frame)` | 格式化事务（唯一会调度渲染帧的编辑动作） |
| pub | `undo_active_editor_tx(frame)` / `redo_active_editor_tx(frame)` | 撤销 / 重做事务 |

#### `orchestrator/replace_pipeline.rs` — 查找替换批量应用管线
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `RegionTextReplaceRequest` (struct) | 替换请求 DTO（区域 + 查询 + 替换文本） |
| pub | `RegionTextReplaceResult` (struct) | 替换结果 DTO（成功/跳过计数 + 渲染帧） |
| pub | `apply_region_text_replacements_tx(requests, frame)` | 逐条替换匹配→补丁→apply_patch_with_history，成功后请求文档刷新 |

### `overlay/` — 覆盖层数据收集、投影与 caret 绘制

#### `overlay/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod` × 4 | navigation / paragraph_overlay / projection / visual |

#### `overlay/navigation.rs` — 键盘导航执行
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `execute_editor_navigation_key(key)` | 由活动状态解析导航键并写入新 caret |

#### `overlay/projection.rs` — 编辑目标 → DOM 视图坐标投影
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `EDITOR_Y_BUFFER` (const) | shell 投影 Y 缓冲 1.0 |
| pub | `ProjectedParagraphInteractionTarget` (struct) | 交互目标投影 DTO（几何 + 字体 + 颜色） |
| pub | `ProjectedEditorShell` (struct) | 活动 shell 投影 DTO（含初始 caret） |
| pub | `project_paragraph_interaction_targets(zoom, page_height)` | 全部交互目标 PDF→视图坐标投影 |
| pub | `project_active_editor_shell(zoom, page_height)` | 活动 shell 投影（含 Y buffer 与归一化 caret） |
| 私有 | `sanitize_display_zoom(value)` | 缩放值净化（非法回落 1.0） |

#### `overlay/visual.rs` — 编辑器覆盖画布绘制（caret-only）
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `EDITOR_Y_BUFFER` / `CARET_WIDTH` (const) | Y 缓冲 1.0 / caret 宽 1.5px |
| 私有 | `sanitize_projection_zoom(v, fallback)` / `resolve_editor_projection_zoom(req)` | 缩放解析（与 host_snapshot 重复实现） |
| 私有 | `scene_shell_width/height`、`body_left/top_offset` | shell 尺寸与 body 相对偏移提取 |
| 私有 | `source_line_bbox_for_caret(target, caret)` | 按 caret 基线取源行可视 bbox |
| 私有 | `resolve_caret_rect(target, caret)` | caret top/height（优先源行 bbox） |
| pub | `render_active_editor_canvas(canvas, zoom, text, caret)` | 同步画布尺寸、清屏并绘制 caret（文本/内容由页画布统一绘制） |

#### `overlay/paragraph_overlay.rs` — 段落渲染覆盖层收集（persisted + active）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::edit::paragraph_overlay::{ParagraphRenderOverlay, …Owner}` | 覆盖层数据结构直通 |
| 私有 | `target_source_object_indices(target)` | 收集目标的源对象索引 |
| 私有 | `persisted_patch_source_indices(patch, target)` | 补丁索引优先解析源对象索引 |
| pub | `collect_paragraph_render_overlays(plan, vm)` | 汇总已存补丁 overlay 与活动 shell overlay（marker 覆盖、图形 marker 透传） |
| tests | `persisted_overlay_tests`（wasm_bindgen_test） | 覆盖：patch 产出 overlay、commit 保编辑、跨页跳过、图形 marker 透传 |

### `host_snapshot.rs` — 编辑器宿主快照组装与诊断
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ActiveEditorRunDiagnostic` / `ActiveEditorSlotDiagnostic` (struct) | run / slot 级诊断 DTO |
| pub | `ActiveEditorDiagnostics` (struct) | 活动编辑器诊断汇总（runs/slots/debug trace） |
| pub | `EditorHostSnapshot` (struct) | 宿主快照：enabled/activeTarget/草稿/caret/targets/补丁标志 |
| 私有 | `sanitize_projection_zoom(v, fallback)` | 缩放值净化 |
| 私有 | `resolve_editor_projection_zoom(requested)` | 动画/预览期取 last_rendered_zoom，否则用请求值 |
| pub | `resolve_editor_host_snapshot(display_zoom)` | 组装完整宿主快照（投影 + 诊断 + 补丁标志） |
| pub | `resolve_active_editor_diagnostics()` | 汇总活动编辑器 runs/slots/调试事件诊断 |

### `search_facade.rs` — 查找/替换 WASM 门面（部分为桩）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use find::find_store::{ReplaceRequest, SearchBox, SearchMatch, SearchResult}` | 查找类型直通 |
| pub | `SearchPageRequest` / `SearchDocumentRequest` (struct) | 查找请求 DTO |
| pub | `FindSessionData` / `ReplaceResult` / `BatchReplaceRequest` / `BatchReplaceResult` / `SearchFacadeResult` / `FindNavigation` (struct) | 会话与替换结果 DTO |
| 私有 | `build_frame_request()` | 默认 FramePlanRequest |
| wasm | `searchFacadePage(request)` | 页内查找（桩：恒返回空结果） |
| wasm | `searchFacadeDocument(request)` | 全文查找（桩：恒返回空结果） |
| wasm | `searchFacadeReplace(request)` | 单区域替换（真实现，走 replace_pipeline） |
| wasm | `searchFacadeBatchReplace(request)` | 批量替换（桩：恒返回 0，TS 仍在调用） |
| wasm | `searchFacadeSetSession(session)` | 设置查找会话（页/全文作用域） |
| wasm | `searchFacadeClearSession()` | 清除查找会话 |
| wasm | `searchFacadeMoveMatch(step)` | 按 step 跳转活动匹配 |
| wasm | `searchFacadeGetSession()` | 读当前查找会话 |

## 疑点
- **双会话状态机并存**：`editor_store.rs`（SessionState：Viewing/Editing/EditingBlock/Saving，含回调与 guard_state! 宏）与 `session/session.rs`（EDITOR_MODE_STATE：编辑开关 + live_state）是两套并行状态容器，`editor_api::EditorSession` 需同时驱动两者；editor-api-architecture-proposal 已建议合并。
- **host_ 前缀未按计划改名**：`host_mode.rs` / `host_runtime.rs` / `host_snapshot.rs` 仍用 host_ 前缀；proposal 建议并入 session.rs，framework-refactor-completion-plan 写明 `host_runtime.rs → platform_bridge.rs`（即 platform_* 方向），均未执行。
- **透传/别名层冗余**：`mode.rs` 8 个函数全部 1:1 转调 session/，其中 `read_active_edit_paragraph`、`read_active_editor_target`、`set_active_edit_paragraph`、`reset_editor_mode` 无外部调用者；`engine_state.rs`、`debug_trace.rs`、`replacement_region.rs`、`replacement_snapshot.rs` 为 1–2 行 core re-export shim（预期过渡，保旧路径）。
- **死代码**：`workflow::build_active_editor_patch`、`orchestrator/render_transaction.rs` 的 `apply_host_input_tx` 与 `commit_editor_silent_tx`（旧 facade.rs 已删，无任何调用者）、`session::clear_active_editor_selection` 均无调用者；`search_facade.rs` 的 `SearchBox as SearchBoxRect` 再导出未被使用，且 `searchFacadePage/Document/SetSession/ClearSession/MoveMatch/GetSession` 在 TS（src/）未发现调用者，仅 Replace/BatchReplace 被 `find_facade.ts` 调用。
- **桩实现**：`searchFacadePage`、`searchFacadeDocument`、`searchFacadeBatchReplace` 恒返回空/0 结果但 TS 仍调用 BatchReplace；`editor_api/session.rs` 末尾 14 个方法（cut/copy/paste/clientToPage/exportPatch 等）为 NotImplemented 桩。
- **重复实现**：`resolve_editor_projection_zoom` / `sanitize_projection_zoom` 在 `host_snapshot.rs` 与 `overlay/visual.rs` 各有一份相同实现；`resolve_target_at_page_point` 在 `activation.rs`（私有）与 `editor_api/helpers.rs`（pub(crate)）重复；`build_frame_request` 在 `search_facade.rs` 与 `editor_api/helpers.rs` 重复（后者硬编码 viewport=0、timestamp=0 等默认值）。
- **与 core/edit 的边界**：领域逻辑已大部迁 core，本目录残留的 `workflow.rs`、`format/text_geometry.rs` 纯计算部分（caret 行构造）与 `orchestrator/mod.rs` 自述的"跨域编排"定位一致；`overlay/paragraph_overlay.rs` 内嵌 4 处 `apply_patch_with_history` 为文档化例外。
- **参数疑点**：`workflow::open_paragraph_editor` 的 `_visual_zoom` 形参被忽略；`editor_api::open_region` 返回的 `OpenBlockResult.block_id` 恒为字面量 `"region"` 而非真实 region_id。
