//! 编辑链路日志切面（lightweight AOP-style trace）
//!
//! 在编辑→commit→持久化→渲染整条链路的关键节点各打 **一行** 结构化日志，
//! 全部用 `[CHAIN]` 前缀。用户在控制台过滤 `[CHAIN]` 即可看到完整调用链，
//! 不被其它 `[AREN_*]`、`[OVERLAY-COLLECT]` 等噪音淹没。
//!
//! 使用：在每个链路节点调用 `trace_step("step.name", &[("k", "v"), ...])`
//!
//! 节点命名约定（按调用顺序）：
//!   open                   ─ 编辑器打开
//!   input                  ─ 文本输入到 live state
//!   exit                   ─ 任何"退出编辑"入口（close / toggle / setMode）
//!   commit.start           ─ commit_active_editor_text 进入
//!   commit.build           ─ build_active_editor_patch 结果
//!   commit.persist         ─ apply_patch_with_history 完成
//!   render.collect         ─ collect_paragraph_render_overlays 完成
//!   render.suppress        ─ source-text suppress 决策
//!   render.draw            ─ overlay 绘制完成
//!
//! 一条日志格式示例：
//!   [CHAIN] commit.persist regionId=p-1 totalPatches=1 newLen=12

use std::cell::Cell;

thread_local! {
    static CHAIN_ENABLED: Cell<bool> = Cell::new(true);
}

/// 运行时开关。默认 ON。生产环境可调用关闭。
pub fn set_chain_trace_enabled(enabled: bool) {
    CHAIN_ENABLED.with(|c| c.set(enabled));
}

pub fn is_chain_trace_enabled() -> bool {
    CHAIN_ENABLED.with(|c| c.get())
}

/// 链路节点打点。**每个节点只调一次**，避免循环内 spam。
pub fn trace_step(step: &str, fields: &[(&str, &dyn std::fmt::Display)]) {
    if !is_chain_trace_enabled() {
        return;
    }
    let mut line = String::with_capacity(64 + 16 * fields.len());
    line.push_str("[CHAIN] ");
    line.push_str(step);
    for (k, v) in fields {
        line.push(' ');
        line.push_str(k);
        line.push('=');
        // 短截：value 超过 40 字符截断，避免一行刷屏
        let s = format!("{}", v);
        if s.chars().count() > 40 {
            let truncated: String = s.chars().take(37).collect();
            line.push('"');
            line.push_str(&truncated);
            line.push_str("...");
            line.push('"');
        } else if s.contains(' ') || s.is_empty() {
            line.push('"');
            line.push_str(&s);
            line.push('"');
        } else {
            line.push_str(&s);
        }
    }
    web_sys::console::log_1(&line.into());
}

/// 便捷宏：trace_step!("commit.persist", "regionId" => &id, "totalPatches" => count);
#[macro_export]
macro_rules! chain_trace {
    ($step:expr $(, $key:literal => $val:expr )* $(,)?) => {
        $crate::editor::edit_chain_trace::trace_step(
            $step,
            &[ $(($key, &$val as &dyn std::fmt::Display)),* ],
        )
    };
}
