/**
 * 点击"分布式"那一行，编辑器打开后，在 DevTools Console 运行这段代码。
 * 这会读取编辑器的内部状态并输出 marker/body/caret 的精确值。
 */

(async function diagnoseDistributed() {
    // 1. 读取编辑器完整快照
    const snap = window.__readEditorSnapshot?.();
    if (!snap?.activeTarget) {
        console.log("编辑器未打开！请先点击'分布式'那一行。");
        return;
    }
    
    const t = snap.activeTarget;
    console.log("=== 编辑器状态 ===");
    console.log("段落ID:", t.paragraphId);
    console.log("区域ID:", t.regionId);
    console.log("完整text:", t.text);
    console.log("markerText:", t.markerText ?? "(null)");
    console.log("markerKind:", t.markerKind ?? "(null)");
    console.log("markerAdvance:", t.markerAdvance ?? "(null)");
    console.log("draftText:", snap.draftText ?? "(null)");
    console.log("caretIndex:", snap.caretIndex);
    console.log("initialCaretIndex:", t.initialCaretIndex);
    console.log("liveCaretIndex:", t.liveCaretIndex ?? "(null)");
    
    // 2. 读取 textarea 的当前值
    const ta = document.getElementById('pdf-editor-textarea-vector');
    if (ta) {
        console.log("\n=== Textarea 实际值 ===");
        console.log("textarea.value:", ta.value);
        console.log("textarea.selectionStart:", ta.selectionStart);
        console.log("textarea.selectionEnd:", ta.selectionEnd);
        console.log("value.charCodeAt(0):", ta.value.charCodeAt(0), "=", ta.value[0]);
        console.log("value.charCodeAt(end-1):", ta.value.charCodeAt(ta.value.length-1), "=", ta.value[ta.value.length-1]);
    }
    
    // 3. 检查 diagnostics 中的 runs 信息
    if (snap.diagnostics?.runs) {
        console.log("\n=== Runs 详情 ===");
        snap.diagnostics.runs.forEach((run, i) => {
            console.log(`Run[${i}]: text="${run.text}" font="${run.fontFamily}"`);
        });
    }
    
    // 4. 检查 slots
    if (snap.diagnostics?.slots) {
        console.log("\n=== Slots 详情 (前20个) ===");
        snap.diagnostics.slots.slice(0, 20).forEach((slot, i) => {
            console.log(`Slot[${i}]: char="${slot.char}" runIndex=${slot.runIndex} x=${slot.x?.toFixed(1) ?? 'N/A'} y=${slot.y?.toFixed(1) ?? 'N/A'}`);
        });
    }
    
    // 5. 检查 sourceBodyCharCount vs draftCharCount
    if (snap.diagnostics) {
        console.log("\n=== 字符计数 ===");
        console.log("sourceBodyCharCount:", snap.diagnostics.sourceBodyCharCount);
        console.log("draftCharCount:", snap.diagnostics.draftCharCount);
        console.log("textPlanCharCount:", snap.diagnostics.textPlanCharCount);
        console.log("slotCount:", snap.diagnostics.slotCount);
        console.log("markerRunCount:", snap.diagnostics.markerRunCount ?? "N/A");
    }
    
    // 6. 获取所有同页 targets，找到附近是否有单独的 ● 段落
    if (snap.targets) {
        console.log("\n=== 同页段落 (y 在 490~510 之间) ===");
        const nearby = snap.targets.filter(tg => tg.top >= 490 && tg.top <= 510);
        nearby.forEach(tg => {
            console.log(`  ${tg.paragraphId}: "${tg.text.substring(0, 40)}" left=${tg.left} top=${tg.top} font=${tg.fontFamily}`);
        });
    }
    
    console.log("\n=== 诊断完成 ===");
})();
