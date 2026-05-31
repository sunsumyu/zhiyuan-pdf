# PDF Engine Naming Guide

This guide defines naming rules for the PDF viewer/editor engine. The goal is to make a file name and function name enough to understand where a behavior belongs.

## Module Names

- Use snake_case for Rust and TypeScript module files.
- Prefer domain nouns over implementation labels.
- Avoid `utils`, `helper`, `manager`, and `misc` unless the file is intentionally temporary.
- Do not repeat the crate or plugin name inside every file.
- A file name owns context, so function names inside that file should not repeat the same context.

Examples:

- `editor_runtime_workflow.rs` may expose `sync_editor_input_v3`, not `sync_active_editor_input_runtime_v3`.
- `editor_host_workflow.rs` may expose `open_editor_at_client_point_v3`, not `open_paragraph_editor_at_client_point_v3`.
- `pdf_window_api.ts` owns window API registration, so exported functions should be about registration, not individual DOM behavior.

## Function Names

- Use `build_*` when creating a value without side effects.
- Use `find_*` when returning `Option<T>` / nullable values.
- Use `resolve_*` when deriving a decision from input and fallback rules.
- Use `sync_*` when copying state between boundaries.
- Use `set_*` when mutating a single state field.
- Use `open_*`, `close_*`, `save_*`, `commit_*`, `undo_*`, `redo_*` for use-case actions.
- Keep external wasm function names stable until the TS call sites are migrated together.

## Avoid These Patterns

- Do not stack boundary words: `runtime_workflow_action_host`.
- Do not encode implementation history in names: `v19`, `audit`, `sovereign`, unless it is a temporary log tag.
- Do not use names that describe how the code was refactored instead of what it does.
- Do not add one-off wrappers whose names differ only by `runtime`, `workflow`, or `host` unless they are real architectural boundaries.

## Boundary Rules

- Rust owns PDF render plans, editor state, text geometry, save/writeback, history, and font/layout semantics.
- TypeScript owns host DOM lookup, global window binding, keyboard event bridging, and invoking Rust wasm APIs.
- AI and manual editing must call the same document edit API and must not create separate writeback paths.
- Stable external wasm names may be longer for compatibility, but internal Rust names should stay concise.
