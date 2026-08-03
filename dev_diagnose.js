/**
 * 在 tauri:dev 的 DevTools Console 中运行这段代码，诊断编辑器状态。
 * 复制完整输出给我。
 */

(function diagnose() {
    const el = (s) => document.querySelector(s);
    const els = (s) => document.querySelectorAll(s);

    console.log("=== 1. 编辑模式状态 ===");
    const snap = window.__readEditorSnapshot?.();
    console.log("enabled:", snap?.enabled);
    console.log("activeTarget:", snap?.activeTarget);
    console.log("target count:", snap?.targets?.length);

    console.log("\n=== 2. DOM 元素检查 ===");
    const ta = document.getElementById('pdf-editor-textarea-vector');
    console.log("textarea exists:", !!ta);
    if (ta) {
        const shell = ta.closest('[data-pdf-editor-shell]');
        console.log("shell exists:", !!shell);
        if (shell) {
            const style = window.getComputedStyle(shell);
            console.log("shell display:", style.display);
            console.log("shell visibility:", style.visibility);
        }
    }
    const targets = els('[data-paragraph-id]');
    console.log("interaction targets:", targets.length);
    if (targets.length > 0) {
        targets.forEach((t, i) => {
            const rect = t.getBoundingClientRect();
            console.log(`  target[${i}] id=${t.dataset.paragraphId} left=${rect.left.toFixed(0)} top=${rect.top.toFixed(0)} w=${rect.width.toFixed(0)} h=${rect.height.toFixed(0)}`);
        });
    }

    console.log("\n=== 3. 点击坐标建议 ===");
    if (targets.length > 0) {
        const first = targets[0];
        const rect = first.getBoundingClientRect();
        console.log(`建议点击中心: x=${(rect.left + rect.width/2).toFixed(0)}, y=${(rect.top + rect.height/2).toFixed(0)}`);
    }

    console.log("\n=== 4. 诊断完成 ===");
})();
