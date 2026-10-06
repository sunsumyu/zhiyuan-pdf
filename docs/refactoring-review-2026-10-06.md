# 重构审查报告（Fowler 坏味道目录视角）— 2026-10-06

> 本报告是 `refactor/debt-cleanup` 分支的立项依据。方法：以《重构：改善既有
> 代码的设计》（Fowler, 2nd ed）第 3 章坏味道目录为显式框架，三路并行审查
> （Rust crates / TS bridge / 测试基建与文档治理），关键指控逐条人工复核。
> 已按宪法流程落地或更正的项，在【处置】中标注。

## 总评

工程纪律显著高于平均水平：契约先行、变异校验、ADR/postmortem 与源码同批
入库。坏味道不在「烂设计」，而在两类系统性问题：① 迁移完成后脚手架未清退
（兼容 shim、双 API 面、死亡抽象）；② 一批宪法违例在热路径上存活，因守护
契约存在扫描盲区。

## 一、死门禁违例（第一批已处置）

| # | 位置 | 违例 | 处置 |
|---|---|---|---|
| 1 | `crates/pdf-viewer-ui/src/render/canvas/page.rs:269` | paint 路径无条件 `[CANVAS-DBG]` 日志（含 format! 分配） | ✅ 已删（连带三计数器），契约补扫 |
| 2 | `src/bridge/render/vector_host.ts:411` | stale 预中止无条件 console.log | ✅ 转 emitPdfDiagnostic DEBUG（ADR-0022） |
| 3 | `src/bridge/render/vector_page_bundle.ts:323` | bundle 加载 stale 中止无条件 console.log | ✅ 同上 |
| 4 | `src/main.ts:8` | dev 探针 verify_editor_bugs 无条件打进生产 bundle | ✅ 改 import.meta.env.DEV 条件动态导入，实测生产 bundle 不含 |
| 5 | `tile_layer.ts:302/708`、`zoom_controller.ts:110` | getBoundingClientRect 旁路 ViewportGeometry（热路径 4 处） | ⏳ 根因是 Owner 缺 `readPageViewportOffset()` 接口——补全抽象后替换（第三批） |
| 6 | `vector_canvas_host.ts:118-119` | backCanvas left/top 与 DetailOverlayOwner 双写者（ADR-0024 批准的时序契约） | ⏳ 呈现公式收拢时合一（第二批） |

## 二、主要坏味道（按书中目录）

### 重复代码 —— 最重要发现（✅ 2026-10-06 第二批已收拢）
ADR-0009/0010/0024 声称三表面「共享同一呈现公式」，实测**共享的是注释不是
代码**：`s = visual/zoom; left = rect.left × s; scale(s)` 有 **4 份内联拷贝**
（canvas_transform_owner.ts、detail_overlay_owner.ts、tile_layer.ts ×2），
两个 Owner 的 lastWritten memo 样板亦各一份。初判的 text_layer 第 5 份
**系误报**：它是构建时直接乘 displayZoom 的不同公式形状（无基矩形重缩放），
不强行收拢。**已处置**：`src/bridge/render/present_math.ts` 收拢为唯一实现
（presentScale / visualOffset / visualTransform / createMemoizedStyle），4 处
全部改走共享模块，契约 `present_math.test.ts` 钉住不变量；Owner 契约测试
零改动全过 = 行为逐字节保持。
其他：render_flow.ts stale-guard 三行块 ×5；Rust resolveFramePlan/
takeFramePlan 函数体逐字相同。

### 过长函数 / 过大模块
TS：renderVectorPageWithPlan 508 行（vector_host.ts:338-845，含 233 行内嵌
闭包）、createTileLayer 644、createPdfViewerRuntime 594；
renderViewportProgressiveIfNeeded 18 参（15 个可选位置参数）。Rust 仅 3 个
超 300 行：parse_content_stream 603、build_vector_page_model_from_display_list
379、build_frame_plan_result 312（混 render_reason/gesture/DPR 预算/缓存键
四职责）。**处置：第二批 Extract Function + Introduce Parameter Object。**
过大模块仅 vector_host.ts 1142 行（三职责）。

### 神秘命名
zoom 子系统 4 名并存（zoom_host/zoom_interaction/zoom_state 别名已成事实主
名）；decision.rs 与 zoom_decide.rs 一字之差；tile_v2/tile_cache_legacy 用
版本号当语义；4 个同名 plan_builder.rs；getLastDisplayZoom 实为 231 行状态
机。**处置：第三批。**

### 死代码 / 夸夸其谈通用性
**已删**（本分支）：core render/renderer.rs 的 PdfRenderer trait +
DrawCommand（trait impl 不触发 dead_code lint，故 0-warning 下存活）、
canvas/renderer.rs 的孤儿 trait impl 与 3 个私有 helper、TS 的
renderVectorPage / resetMainCanvasTransformOwner / hasVectorPageBundle /
isPageBundleCached / tileFacade.isReady / renderApi.renderPage /
stepProgressiveRender（onscreen）、verify_editor_bugs 生产导入。
**仍存活**：约 12 个纯 re-export shim、9 处 #[allow(deprecated)]（双 API 面
症状）、zoom 三别名、decision.rs hub——清退前需先补「旧路径无调用方」哨兵。

### 隐藏耦合
window 全局 30+ 个、三套命名并存（__pdf* / __PDF_* / 无前缀）；vector_host
绕过注入 deps 直读 window.__getCurrentPage 判 stale。Rust 约 15 处
thread_local 藏在非 *_store.rs 文件。**处置：第三批（Encapsulate Global
Reference + thread_local 归位）。**

## 三、测试基建

四层实测：core 274 / wasm-pack 24 / vitest 28 文件 165 用例 / E2E 31 spec
（zoom_* 24）。结构性问题：
1. **探针与契约 1:1 混跑**：14 个零断言测量探针被宪法门禁 zoom_*.spec.ts
   glob 一并当验收跑。建议搬 tests/e2e/probes/ 退出门禁。
2. **wasm 层最薄（24）且 CI 完全不跑**——五条宪法门禁 CI 只守 3 条。
3. **源码 grep 契约是重构阻力点**：readFileSync 断言实现形状的测试（如
   render_entry_single_request）使提取函数级别的重构也会打碎它们，应逐步
   换行为契约。
4. 盲区：tile_layer / vector_host / render_flow 三宿主零直测，全靠 E2E 兜底。

## 四、文档与仓库卫生

- docs/ 顶层 33 篇约 44% 是 5-6 月已完成时代的化石，docs/archive/ 未收编。
- 三处活矛盾：ZOOM_ARCHITECTURE_SPEC 仍以被 ADR-0008 取代的 ADR-0007 为前提；
  AGENTS.md 指向被 guide/README 宣布取代的 development-guide.md；根 CONTEXT.md
  与 docs/CONTEXT.md 双词汇表。
- 数字漂移：宪法 271 → 实际 274；HANDOFF「22 spec」→ 实际 31。
- 跟踪残留：tools/msedgedriver/ 80MB（含 .bak147）；icon.jpg / test.pdf /
  ocr*.ps1 零引用。
- 根目录 29 份陈旧 log + 2 张 tmp_repro_*.png 已按宪法§四清理（本分支）。

## 五、审查误报更正（复核的价值）

1. **tile_cache_legacy 不是死代码**：初判「469 行仅被 shim 引用」——实际它
   是活实现，HostPresentState / FrameCacheStoreResult / BaseLayerCacheEntry
   等经 tile_cache.rs shim 被 frame_cache / plan_builder / present_store /
   workflow（+2 测试）消费。坏味道是「名字撒谎 + shim」非死代码，处置改为
   tile_v2/legacy 命名整肃（第二批）。
2. **stageViewportCanvasFromSource 不是死代码**：vector_host.ts:634/719 在
   用；它是与 presentViewportCanvasFromSource 同签名的纯转发别名，属改名合并
   候选（第二批）。
3. **stepProgressiveRenderOffscreen 在 vector_worker.ts:111 在用**：只能删
   onscreen 版。

## 六、批次规划

- **第一批（本分支已落地）**：热路径违例清除 + 契约补盲 + 死代码清退 + 根
  目录卫生 + 宪法数字修正。
- **第二批（结构）**：✅ 呈现公式收拢 present_math（2026-10-06）；⏳ 拆
  renderVectorPageWithPlan；⏳ RENDER_STATE<serde_json::Value> 类型化；
  backCanvas left/top 双写者合一（present_math 已铺路，涉及时序契约单独循环）。
- **第三批（抽象补全）**：ViewportGeometry 补 readPageViewportOffset()；
  ZOOM_STATE 写入口 pub(crate) 化；shim/别名/deprecated 清退（先补哨兵）；
  window 全局收拢；tile_v2/legacy 改名。
