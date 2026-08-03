/**
 * 诊断为什么没有交互目标。复制整个输出给我。
 */

(function diagnoseTargets() {
    // 查找所有可能的文本层容器
    const containers = [
        '[id*="text-layer"]',
        '[id*="text"]',
        '[id*="vector"]',
        '[id*="pdf-render-target"]',
        '#pdf-text-layer',
        '#pdf-vector-container',
        '#pdf-text-container',
    ];

    console.log("=== 1. 可能的文本层容器 ===");
    containers.forEach(selector => {
        const el = document.querySelector(selector);
        if (el) {
            console.log(`Found: ${selector}`, el);
            // 检查它的子元素
            console.log(`  children:`, el.children.length);
            if (el.children.length > 0 && el.children.length <= 20) {
                Array.from(el.children).forEach((child, i) => {
                    console.log(`  [${i}]`, child.tagName, child.id || '', child.className || '', 
                        child.dataset?.paragraphId ? `pid=${child.dataset.paragraphId}` : '');
                });
            }
        }
    });

    // 查找任何有 data-paragraph-id 的元素
    console.log("\n=== 2. 所有带 paragraphId 的元素 ===");
    const pidElements = document.querySelectorAll('[data-paragraph-id]');
    console.log(`Count: ${pidElements.length}`);
    
    // 查找任何文本 span 或 div
    console.log("\n=== 3. 文本层中的所有 div/span ===");
    const textLayer = document.getElementById('pdf-text-layer');
    if (textLayer) {
        const allTexts = textLayer.querySelectorAll('div, span');
        console.log(`Total elements: ${allTexts.length}`);
        if (allTexts.length > 0 && allTexts.length <= 50) {
            Array.from(allTexts).slice(0, 30).forEach((el, i) => {
                console.log(`  [${i}]`, el.tagName, 
                    el.className ? `class="${el.className}"` : '',
                    el.id ? `id="${el.id}"` : '',
                    el.dataset?.paragraphId ? `pid=${el.dataset.paragraphId}` : '',
                    `style="${el.style.cssText?.substring(0, 80)}"`,
                    `"${el.textContent?.substring(0, 40)}"`);
            });
        }
    }

    console.log("\n=== 4. 全局搜索包含"分布式"的元素 ===");
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, null, false);
    let element;
    let found = [];
    while (element = walker.nextNode()) {
        if (element.textContent?.includes('分布式') || element.textContent?.includes('专业技能')) {
            found.push({
                tag: element.tagName,
                id: element.id,
                class: element.className,
                pid: element.dataset?.paragraphId,
                style: element.style?.cssText?.substring(0, 80),
                rect: element.getBoundingClientRect ? {
                    left: element.getBoundingClientRect().left.toFixed(0),
                    top: element.getBoundingClientRect().top.toFixed(0),
                    width: element.getBoundingClientRect().width.toFixed(0),
                    height: element.getBoundingClientRect().height.toFixed(0),
                } : null,
                text: element.textContent?.substring(0, 60),
            });
        }
    }
    console.log(`Found: ${found.length}`);
    console.log(JSON.stringify(found, null, 2));

    console.log("\n=== 5. 检查 pdf-render-target 结构 ===");
    const pdfRenderTarget = document.getElementById('pdf-render-target');
    if (pdfRenderTarget) {
        console.log("pdf-render-target found, children:", pdfRenderTarget.children.length);
        function dumpTree(el, prefix = '') {
            if (el.id || el.dataset?.paragraphId || el.dataset?.pageId || el.className) {
                console.log(`${prefix}${el.tagName}#${el.id || ''} [${el.dataset?.paragraphId || 'no-pid'}] [${el.className || ''}]`);
            }
            for (const child of el.children) {
                dumpTree(child, prefix + '  ');
            }
        }
        dumpTree(pdfRenderTarget);
    }

    console.log("\n=== DONE ===");
})();
