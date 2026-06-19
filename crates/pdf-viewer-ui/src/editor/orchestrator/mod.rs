//! editor/orchestrator — cross-domain workflow coordination for the editor domain.
//!
//! 诚实声明：这个子模块下的代码**不是纯编辑逻辑**，而是跨域编排：
//! 它们会主动调用 `document` / `state_manager` / `present` 域触发副作用。
//!
//! 来源 (structure-flow audit Batch 2 §2)：editor/ 耦合了 document / page /
//! present / render / state_manager / viewer / zoom 共 7 个域，152 处跨域引用
//! 中 editor 独占 ~22%。原本这些编排函数散落在 editor/ 的 3 个顶层文件里，
//! 与纯编辑基础设施（mode / activation / overlay 几何等）混居，违反"editor 是
//! 编辑域"的单一职责原则。
//!
//! 本模块是审计后的应用层：
//!
//! | 文件 | 触发的跨域副作用 |
//! |------|------------------|
//! | `commit.rs` | `document::patch_persistence::apply_document_patch_direct`；`state_manager::remember_paragraph_replacement_target` |
//! | `render_transaction.rs` | `present::present_store::schedule_render_frame_request`（9 个 *_tx 函数都会 schedule 渲染帧）|
//! | `replace_pipeline.rs` | `state_manager::record_patch`；`document::mutation_pipeline::request_document_refresh` |
//!
//! 已知例外（未完全迁入 orchestrator）：
//!
//! * `editor/overlay/paragraph_overlay.rs` 仍有 4 处 `record_patch`
//!   调用嵌在 overlay 构造代码里。这些调用与绘制几何耦合紧密，拆出来会破坏
//!   几何/副作用的局部性。保留在原地，但文档化。
//!
//! 新增跨域编排函数请放在本模块内，不要加到 editor/ 顶层。

pub mod commit;
pub mod render_transaction;
pub mod replace_pipeline;
