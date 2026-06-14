# 矢量 PDF 文字渲染空白及首屏图像拉大闪烁问题 (Postmortem)

## 背景描述
在项目中引入了“分步加载优化”（Split Loading Optimization）机制（意图为先加载图片和路径进行快速首屏渲染，随后异步获取文本对象）。但此优化引入后导致：
1. 大量矢量 PDF 文件在前端渲染时文字完全不显示（空白）。
2. 部分带背景色的 PDF 即使渲染出文字，也会被深色背景块完全遮挡。
3. 当用户首次打开一个新 PDF 文件时，页面会出现短暂的“失量图被拉伸/放大闪烁”现象。

## 根本原因分析

### 1. 文本空白与遮挡的成因
- **Worker 缓存导致的脏读 (Stale Cache)**：前端在 `vector_page_bundle.ts` 中试图通过异步请求加载完整的文字并强行塞入已被缓存的 `model.objects`，然后派发事件通知重绘。但底层的 `vector_worker.ts` 中有严格的 `isSamePage` 性能保护校验，只要路径和 `revision` 没变，Worker 就会忽略掉被 Mutation 的前端对象引用，一直使用其自身内存中那个只包含了图片的残缺 Model 状态渲染，导致文字死活画不出来。
- **渲染引擎打乱了 Z-Index 层级**：后端的 Rust 模块 (`vector_engine.rs`) 为了极致的批处理渲染性能，引入了样式并行排序算法 (`par_sort_by`)，只按颜色等样式归类，而彻底丢弃了原有的 `z_index`（物理堆叠顺序）。这直接导致深色路径矩形背景块被置于顶层，盖住了文字。

### 2. 初始加载拉大闪现的成因 (Canvas Aspect-Ratio Stretching)
- **双缓冲的 CSS 尺寸更新遗漏**：在前端渲染宿主 `vector_host.ts` 中，为了避免画面中间态闪烁，强行开启了 `deferVisiblePresent = true` 实施双缓冲。
- 这意味着 `applyViewportCanvasFrame` 虽然初始化了后台离屏画布，但**推迟了前台可见的 `refs.container` 外层容器的 CSS 宽高更新**。
- 当底层 Worker 渲染完毕并将像素提交给前台 `commitVectorRenderResult` 时，代码中只更新了 `mainCanvas` 自己内部元素的宽高，却依然**忘记更新**外侧包裹它的 Container 容器的宽高！结果外层容器维持了上个文件的旧尺寸或默认小尺寸，导致刚画好的大尺寸 Canvas 像素被浏览器 CSS 强行挤压、拉伸，产生了“失量图被拉大闪现”的视觉 Bug。

## 3. 首屏拉大闪现的成因（深度分析）

### 表现

打开新 PDF 文件时，旧文档（如秒表）的矢量画布内容会短暂闪现，且被拉伸放大到新文档的尺寸，随即消失，新文档才正确出现。

### 根本原因：竞态条件（Race Condition）

`openTextPdfFlow` 函数中的打开流程原来是：

```ts
// ❌ 错误顺序（会产生竞态）
await session.open({ path, ... });  // 异步 IPC 调用，耗时数十～数百ms
deps.clearVectorHost();             // 太晚了！
```

在 `session.open()` 异步等待期间，JS 事件循环会继续处理其他 Promise。**旧文档的渲染 Worker 可能恰好在此期间完成渲染**，然后正常执行 `commitVectorRenderResult` → `presentViewportCanvas`，把旧文档内容设为 `display: block` 可见，而此时 `clearVectorHost()` 还没有被调用！

```
时间轴:
  t0: 用户打开新文档，session.open() 开始（等待 Rust IPC）
  t1: ← 旧文档 Worker 完成渲染（刚好在这个时间窗口）
  t2: ← 旧文档 commitVectorRenderResult 执行 → 旧内容 display:block
  t3: session.open() 返回
  t4: clearVectorHost() 被调用 → 容器隐藏（太晚了！t2~t4 之间用户已经看到闪烁）
```

### 为什么旧内容是拉伸状态

`syncLayoutBox`（在 `commitVectorRenderResult` → `prepareVisibleFrame` → `commitRenderedFrame` 链路里调用）会用**当前（已经是新文档的）zoom 和 pageWidth/pageHeight** 来计算容器 CSS 尺寸。而旧文档的画布像素（如秒表）大小和新文档（简历）的页面尺寸不同，导致像素被拉伸填充到新的容器里，产生"放大失量图"的视觉效果。

### 修复方案

**将 `clearVectorHost()` 移到 `session.open()` 之前调用**：

```ts
// ✅ 正确顺序（防止竞态）
deps.clearVectorHost();             // 立即调用：取消 Rust 侧的 in-flight 渲染
                                    // cancelProgressiveRender + resetFrameCache
                                    // 旧帧 token 失效，stale-frame 检查会拦截
await session.open({ path, ... });  // 安全地等待 IPC
```

这样在整个 `session.open()` 等待期间，旧渲染的帧 token 已失效：
- 旧 Worker 完成后，`isRenderFrameCurrent()` 返回 false → 渲染中止
- `commitVectorRenderResult` 不会被调用 → 旧内容不会闪现

1. **全面回退分步加载 (Rollback Split Loading)**:
   - 彻底移除 `vector_page_bundle.ts` 中的 `triggerAsyncFullBundleLoad` 和针对 `textOnly` / `imageOnly` 的特殊处理逻辑。
   - 回归单一职责和不可变数据流（Single Source of Truth），强制要求在 Rust 端一次性吐出包含了图、文、路径的完整页面描述包供渲染。同时清理掉由于分步加载引入的复杂版本号判定逻辑。

2. **修复后端 Z-Index 联合排序**:
   - 在 Rust 的 `vector_engine.rs` 及 `prepared_scene.rs` 中修改排序闭包，引入 `(style, z_index)` 的联合比较。在保证 Z-Index 物理层叠绝对正确的前提下，再在同层级内部进行样式分组渲染。

3. **补齐双缓冲生命周期里的容器尺寸设置**:
   - 在 `vector_host.ts` 中，向渲染出口函数 `commitVectorRenderResult` 补传了正确的 `displayWidth` 和 `displayHeight` 参数。
   - 在 Present 正式将后台画布切换至前台显示之前，增加代码 `refs.container.style.width = ...` 强制同步容器宽高。这样外框与内画板同时抵达正确的宽高比，根除了由于 CSS 容器错位引起的画面拉伸拉大闪现。
