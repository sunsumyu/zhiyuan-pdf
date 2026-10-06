# HANDOFF — 会话交接与运维手册

> 2026-10-03 起本文件收敛为「最新会话 + 索引」。历史会话全文（723 行版）
> 存于 git 历史（b6a4920 / 8f119fc 之前的版本）；细节以 docs/adr/ 与
> docs/bug-postmortems/ 为准。仓库宪法见根目录 **AGENTS.md**。

## 会话 2026-10-06（第三批）— RENDER_STATE 类型化：消灭热路径 JSON 中转

- **动机**：立项报告§二——`HostRenderState<serde_json::Value>` 强迫帧计划在
  Rust 内部以 JSON Value 流转：schedule 时 `to_value`、share 检查
  `from_value(clone)` ×2、settle 再 `from_value`、陈旧 zoom 帧清扫用
  `.get("render_zoom")` 字符串取值（stringly-typed 调度判定住在呈现层）。
- **实现**：`render_store.rs` 的 store 与三函数（schedule/settle/drop_queued）
  收为具体类型 `FramePlanResult`（core plan_builder），直接 `clone()` 进出；
  to_value/from_value 闭包参数整体删除；`workflow.rs` settle 调用去闭包；
  `present_store.rs` 陈旧帧清扫改字段访问（`map_or(false,…)` 原样保留「无
  plan 不判 stale」语义，clippy 要求 `is_some_and`）；RENDER_STATE 全仓唯一
  实例化点，泛型只服务于旧测试——两份测试改用真实类型（render_store_tests
  以 `render_zoom` 为区分字段；workflow_tests 的 JSON 夹具经
  `from_value` 一次解析为类型化 plan，夹具本身保留可读性）。
- **边界不变**：token 状态机逐行保持（含 queued 提升不更新 active 的注释）；
  wasm 边界对 TS 的序列化照旧发生在 free_api 导出层——内部存储类型对 TS
  不可见，E2E 几何契约是行为见证。
- 门禁：core 274/274；clippy 双目标 `-D warnings` 0；触及文件 fmt 干净
  （既有漂移文件未追）；wasm **24/24**；wasm+e2e 重建后 zoom 套件
  23/24 + 单跑定性——失败 spec `zoom_gesture_clarity_contract`（套件轮
  max 15.5%>15%；单跑 3 轮 2 绿 1 红，红轮 p50 6.05%>5%，两轮命中**不同**
  断言、超出均 <20%）。定性 = 既有采样方差类别（HANDOFF 2026-10-05 已记
  「同配置 run 间方差大」且 p50 因同类伪影重钉过；改类型化**前**的批次二
  套件同样 1 失败），非回归——语义逐路径等价、热路径只减负、reknock
  cadence 与 direct≥3 断言全绿。证据留存本条；p50=5% 界在本机偏紧，
  是否放宽属 UX 决策未动。净变化 +64/−94。

## 会话 2026-10-06（第二批）— 呈现公式收拢 present_math（ADR-0009/0010/0024 承诺兑现）

- **动机**：立项报告§二最重要的重复代码发现——三表面统一公式以 4 份内联
  拷贝存活（2026-09-30/10-03 双重曝光 bug 正是拷贝间分歧）。本批收拢为唯一实现。
- **实现**：`src/bridge/render/present_math.ts`——`presentScale`（zoom 对验证
  + s 计算）、`visualOffset`/`visualTransform`（字符串化，identity 恒 scale(1)）、
  `createMemoizedStyle`（两 Owner 重复的 lastWritten 样板收拢；**直写属性赋值**
  而非 setProperty——vitest 是 node 环境，Owner 契约测试的 Proxy 桩只实现属性
  set 陷阱，走 setProperty 会当场打碎既有契约）。消费方改写：canvas_transform_owner
  （transform 单 memo）、detail_overlay_owner（left/top/transform 三 memo；
  DetailOverlayRect 改为 BaseRect 别名）、tile_layer drawTile +
  refreshTileTransforms（无 memo，保持原样）。**text_layer 不收拢**：它是构建
  时直接乘 displayZoom 的不同公式形状，强行统一属牵强抽象（初判「5 份」更正
  为 4 份，立项报告已更正）。
- **行为边界**：唯一语义差异 = tile_layer 两处原无验证（visualZoom 非法时会
  写 NaN 样式），现 s=null 跳过写——该输入被 zoom authority 消毒保证不可达；
  Owner/tile 契约测试零改动全过。
- **契约**：`present_math.test.ts` 8 例（s 公式、非法输入拒绝、q → q ×
  visualZoom 不变量、identity 表示、memo 跳写/失效/按属性独立）。全仓 grep
  确认代码路径已无内联公式残留（余下命中均为注释）。
- 门禁：vitest **173/173**（165+8）；tsc 0 + vite build 绿；无 Rust 改动
  （wasm 不重建）；e2e:build 重建后 zoom 套件 **24/24**——首轮 1 失败
  （输出被 tail 截断未留 spec 名，重跑 24/24 绿，判定 4-worker 并发假失败，
  宪法未决 #5 已知类别；教训：E2E 后台跑必须重定向完整日志而非 tail 管道）。
- **未动**（第二批余项）：renderVectorPageWithPlan（508 行）拆解、
  RENDER_STATE<serde_json::Value> 类型化、backCanvas left/top 双写者合一
  （present_math 已铺路——writer 合一属 Move Method 到 DetailOverlayOwner，
  涉及时序契约，单独循环）。

## 会话 2026-10-06 — 死门禁违例清除 + 死代码清退（debt-cleanup 第一批）

- **立项**：Fowler《重构》坏味道目录三路审查（Rust/TS/测试基建）→
  `docs/refactoring-review-2026-10-06.md`（含批次规划与审查误报更正）。
  分支 `refactor/debt-cleanup`。
- **热路径违例清除（ADR-0013/0022）**：`canvas/page.rs` render_page 尾部
  `[CANVAS-DBG]` 无条件日志（连带只服务它的三个 draw_*_count 计数器与
  VectorRenderObject 孤儿 import）；`vector_host.ts` stale 预中止、
  `vector_page_bundle.ts` bundle stale 中止 → 转
  `emitPdfDiagnostic(level:'DEBUG')`（免 IPC）。**红灯契约先行**：
  hot_path_logging 扩至 src/bridge 渲染路径四文件（禁
  console.log/info/debug，warn/error 保留给真故障）+ crates page.rs 扫描，
  +2 契约（修复前红 2 → 绿 5/5）。
- **死代码清退**（逐项全仓 grep 零调用后删）：core `render/renderer.rs`
  全文件（PdfRenderer trait + DrawCommand——trait impl 不触发 dead_code
  lint，故 0-warning 下存活）；`canvas/renderer.rs` 的孤儿 trait impl +
  3 私有 helper（保留 canvas_overlay 在用的 `draw_text_run`）；TS 侧
  `renderVectorPage`（私有零调用）、`resetMainCanvasTransformOwner`、
  `hasVectorPageBundle`/`isPageBundleCached`、`tileFacade.isReady`（仅测试
  引用，连同断言删）、`renderApi.renderPage`/`stepProgressiveRender`
  （onscreen 遗留通道；**Offscreen 版 vector_worker 在用，保留**）；
  `main.ts` 的 verify_editor_bugs 改 `import.meta.env.DEV` 条件动态导入
  （新增 `src/vite-env.d.ts`；生产 bundle 实测不含、wasm 实测无
  CANVAS-DBG 字符串）。
- **审查误报更正（复核价值）**：tile_cache_legacy **非死代码**——活实现经
  tile_cache.rs shim 被 frame_cache/plan_builder/present_store/workflow 等
  7 处消费，坏味道是「名字撒谎 + shim」，处置归 tile_v2/legacy 改名批次；
  `stageViewportCanvasFromSource` 非死代码（vector_host:634/719 在用，
  同签名纯转发别名，改名合并候选）。
- **卫生**：根目录 29 份陈旧 log + 2 张 tmp_repro_*.png 按宪法§四「用完
  删除」清理（全部未跟踪产物）。AGENTS.md 核心单测数字 271→274 并加
  「以 HANDOFF 最近门禁记录为准」防漂移。
- 门禁：core 274/274；clippy native+wasm32 `-D warnings` 0；触及文件 fmt
  干净（plan_builder.rs 存在**既有** fmt 漂移，非本会话引入，未追）；wasm
  24/24；vitest 165/165（163+2 新契约）；tsc 0 + vite build 绿；wasm+
  e2e:build 产物重建后 **zoom 套件 24/24**（6m30s）。
- **未决（后续批次，见立项报告§六）**：第二批 = renderVectorPageWithPlan
  （508 行）拆解 + 呈现公式 5 份拷贝收拢（含 backCanvas 双写者合一）+
  RENDER_STATE<serde_json::Value> 类型化；第三批 =
  ViewportGeometry.readPageViewportOffset()（消 4 处热路径 gBCR 旁路）+
  ZOOM_STATE 写入口 pub(crate) 化 + 12 shim/zoom 三别名/9 处
  `#[allow(deprecated)]` 清退（先补哨兵）+ window 全局收拢 + tile_v2/legacy
  改名。治理项：14 个零断言探针 spec 移出 zoom_* 门禁 glob、CI 补
  wasm-pack job、msedgedriver 出库。

## 会话 2026-10-05（晚）— 未决 #2 证伪关闭（ADR-0008 Negative 过时）

- **取证**：HANDOFF #2 记录的「resize 期间 syncHostLayout 写居中 offset
  重置锚点」在当前架构不可达——`syncLayoutBox` 唯一调用方是调试探针
  `__pdfViewerGeometryProbe`；`visual_layout` 生产写者只剩
  `on_wheel_event`（cursor-anchor）与 `apply_committed_frame`（仅 settle）；
  生产 resize 监听只有 tile_layer 的 ViewportGeometry 失效（ADR-0014）。
- **实证**（`zoom_resize_anchor_contract.spec.ts`）：缩放 2.3× 后
  shrink+grow 循环，插桩确认 `syncHostLayout` 0 次调用、wrapper 0 次
  style 变更（其宽度是 CSS 流式布局自然等于 vp−滚动条，无 inline width）、
  scroll 精确保持 (112,510)、zoom 不变；2 轮逐位一致，变异校验红
  （注入布局改写+滚动重置 → toBe 红）。
- **处置**：#2 关闭（同 #3/#6 模式：证伪不再作为待办）；探针升级为守护
  契约钉住"resize 不改写布局/滚动/缩放"，防止未来把 syncLayoutBox 重新
  接回 resize 时无声回归。"resize 保持视口中心点居中"记为增强（未立项）。
- 取证过程中的两个误读（供后续避免）：wrapper 宽度随视口变化曾被当成
  "有人改写几何"——实为 CSS 流式布局；zoom 2.257→2.297 曾被当成
  "resize 触发重缩放"——实为收敛尾未完（SETTLE_MS 1800 不够，契约用
  2500 并断言 A/C 双端 settled）。

## 最新会话 2026-10-05 — longtask 列车证伪 + console sink 限流（ADR-0015 前置条件落地）

- **证伪**：ADR-0026/HANDOFF #0 记录的「手势期 ~100ms/周期管线 longtask
  列车」不存在于产品——verbose OFF（生产默认）同手势 **0 长任务**（5/5
  运行，rAF 175-224 帧 vs verbose 下 70-99）；列车是 `helpers/app.js`
  强制 verbose 追踪的诊断洪泛伪影（verbose ON：4-19 LT/手势，run 间方差
  4→19，唯一稳定对照是 B≡0）。全文 → postmortem
  `2026-10-05-gesture-longtask-train-artifact.md`；归因工具
  `zoom_longtask_attribution_probe.spec.ts`（隔相 reload A/C/B）。
- **随修**：`emitPdfDiagnostic` console sink 生产者侧限流（32ms/8 条、
  history 全量、ERROR/WARN 豁免、抑制摘要）+ terminalMessage 惰性构建；
  红灯契约 `diagnostics_console_rate_limit.test.ts`（4，先红后绿）——
  ADR-0015「生产者侧限流」前置条件首次落地（console sink；IPC 分支不动）。
- **连带重标**：clarity 契约 p50 1%→5%——原值是 verbose 饿死 rAF 下的
  稀疏采样伪影（限流解饿后采样密度 ×3、斜坡滞后进中位、最差 0.0395）；
  结构性质（零设计拉伸）不变，max≤15% 不受影响。变异校验红复验 1 轮。
- **陷阱**：verbose ON 下 E2E 计时全部被伪影润滑（性能结论必查归因探针）；
  同配置 run 间方差大（<~500ms 差异不可解读）；对已提交文件做变异校验用
  sed 注入/sed 删除回退，勿 `git checkout --`（会连带回滚未提交重标定）。
- 门禁：core 271/271（无 Rust 改动）、vitest 163/163、tsc 0、wasm+e2e
  产物重建、clarity 5 轮绿、zoom 全套件 **19/19**（首跑 2 失败均已定性：
  tilegrid=ECONNREFUSED 并发假失败、attribution=60s 超时→局部 300s）。

## 会话 2026-10-04 — P2 内容档位实测：分布双峰，杠杆更正，决策不动

- 用 `zoom_gesture_tilegrid_probe.spec.ts`（**零插桩**：配对诊断历史
  `ts.layer.rendered(useViewportTile)` ↔ 其前的 `ts.layer-plan`）测得
  **屏幕上实际呈现**的补丁档位——此前用 `ts.layer-plan.displayZoom` 测的是
  请求目标 zoom，构造上滞后恒 0，不代表屏幕内容（两个测量陷阱都已记入探针
  注释）。
- **4 轮实测**（16 步 ctrl-wheel 1→5.28×）：滞后分布**双峰**——约半数帧 0；
  瞬态两段（快速爬升段 `render_in_flight` 抑制 reknock + **收敛尾段 ~800ms**
  RAF `stop_zoom_raf_loop()` 后补丁停在最后成功档），上四分位 11–21 档、
  峰值 15–27 档（高 zoom 区 **CSS 拉伸 ~13–14%**，可见模糊）。
- **两个更正**：(1) P2 是**补丁**的内容新鲜度问题，不能映射到瓦片网格
  （`pdf-tile-layer` DOM 手势中按设计冻结——持开页 settle 瓦片被拉伸，
  settle 才重排）；(2) 尾段滞后与 reknock 节流（`PREVIEW_REKNOCK_*`）
  **无关**——RAF 已停，reknock 不参与；真正窗口是「RAF 停止 → settle
  渲染落地」。
- **决策：P2 不动**。再优化应从 settle 渲染启动时机入手；结合 ADR-0025
  （加重渲染负载 → 更多长任务）风险大于收益。postmortem P2 段已写实测
  数据与结论。
- 另修：diag 诊断截图 untrack + gitignore（`d289810`，避免每次跑 spec
  污染工作区）；ADR-0024 契约去 flaky 并在并发全套件验证（`72c961d`/
  `2beb986`）；全 E2E 22 spec 绿（`458b092`）。

## 会话 2026-10-04 — 瓦片泵并发：实现后否决回滚（ADR-0025）

- 推进 HANDOFF 未决 #3（瓦片并行）。取证：worker 单瓦片 p50=1ms/p90=4.4ms
  （CPU 空闲）、手势队列峰值 52、TS 泵单飞行。**据此假设泵是瓶颈**，
  实现有界并发泵（上限 4 + 重复键守卫），红灯契约 6/6 绿，全量门禁绿。
- **决定性 A/B（同代码只改 `MAX_INFLIGHT_TILES`）证伪假设**：cap=4 的
  排空窗口（3630–4393ms）比 cap=1（3315ms）**更慢**、长任务**更多**、
  rAF 帧数**更低**，而 `cache.ready` **两者都是 36** —— 零吞吐收益 + 增加 jank。
- **根因修正**：队列深度 52 是手势期持续流式入队的正常 churn（Rust 只在
  出队时清理陈旧/重复项，`queue_size` 是上界非待办数）；真实工作 ≈36 张，
  两配置都按时完成，**没有可加速的积压**。60ms reknock 节流是**内容新鲜度**
  问题（P2 残余），非吞吐问题。
- **决定：回滚**（`tile_layer.ts` 还原到 ADR-0024 已提交态；删除新增契约）。
  全文与复盘 → `docs/adr/0025-tile-pump-concurrency-rejected.md`
  （含"未来重试前置条件"：须先证明真实积压、且 A/B 不劣化）。
- 无源码净变更；仅 ADR-0025 + 本 HANDOFF 入库。

## 会话 2026-10-04 — P1 修复：DetailOverlayOwner（ADR-0024）

- backCanvas 陈旧视口补丁（前会话录屏定位的 P1/P3）已按红灯契约流程修复：
  - 红灯先行：`src/__tests__/detail_overlay_owner.test.ts` 7 例（模块不存在
    → 红），实现 `src/bridge/render/detail_overlay_owner.ts` 后 7/7 绿。
  - 接线三处：present 在 `commitVectorRenderResult`（detail present 落地时
    记 rect/displayZoom + 同 turn 写视觉映射）；sync 在 tile_layer 的
    `syncVisualTransforms`（新增 `getDetailCanvas` dep，pdf_runtime 注入）；
    reset 在 `hideDetailCanvas` / `presentViewportCanvas` hide 分支 /
    `clearVectorCanvasHost`。
  - 公式与 ADR-0009 瓦片、ADR-0010 主 canvas 完全一致：
    `left = rect.left × (visual/displayZoom)`，`transform = scale(s)`，
    width/height 留 base 空间 → 三个 zoom 驱动表面同一页面坐标不变量
    `q → q × visualZoom`。静止态"白色假页面"与手势中几何错位同时消解
    （P1 永久变体：settle skip-render 后补丁仍被逐帧对齐）。
- **P4 E2E 帧级契约已补齐**（postmortem P4 闭环，P1~P4 全关）：
  `tests/e2e/specs/zoom_detail_overlay_geometry.spec.ts`——像素无关谓词
  "补丁可见帧必含于主 canvas 视觉矩形（8px 容差）"，快速缩小 ≥3 档 +
  静止 2s 逐帧采样，含检测器非空转（人为位移必须被标记）与路径非空转
  （补丁可见帧 ≥1，防 fixture 不触发瓦片路径时契约空转）自检。
  **变异校验**：s=1 冻结公式 → 红（"2/43 帧补丁超出页面"，首帧 ms=179
  rect 记录右/下缘超出 7-9px——录屏缺陷的几何特征）；还原 → 2/2 绿。
  zoom 全套件 15 spec：并发轮 1 个假失败（4-worker 已知类别），15/15 单跑
  全绿（blank/p2/writer/frame/sudden_jump 逐个单跑验证）。
- 门禁：tsc 0 err；core 269/269；E2E 产物已重建（wasm:pdf-viewer-ui +
  e2e:build）。vitest 145/146——唯一失败 = wasm mtime>1h 哨兵（wasm 重建后
  1h 内曾全绿 146/146，时钟推移后老化转红，属预期非回归）。
- 全文 → `docs/adr/0024-detail-overlay-unified-geometry.md`；
  AGENTS.md 死门禁 2 单写者表已补 backCanvas → DetailOverlayOwner 行。

## 会话 2026-10-03（晚）— 知识链入库 + 记忆层收拢

- 用户录屏（13:20，**已含 ADR-0023 修复的最新构建**）逐帧取证：缩放双重
  曝光/静止态重影的根因 = **backCanvas 陈旧视口补丁**（detail overlay 无
  几何补偿、手势结束无人隐藏、settle 复用跳渲染时存在永不消失变体）。
  全文 → `docs/bug-postmortems/2026-10-03-backcanvas-stale-patch.md`。
- 三笔提交：`b6a4920`（backfill ADR-0009~0022 + 09-27 postmortem）、
  `8f119fc`（ADR 时期源码 + 3 个 Rust 契约测试 + 8 个 vitest 契约 +
  11 个 E2E spec + CDP 工具 + HANDOFF，59 文件）、`5e282d9`（新 postmortem）。
- 提交前复跑：core 269/269 ✓；vitest 138/139（唯一失败 = wasm mtime>1h
  新鲜度哨兵，预期）。
- 另一笔：AGENTS.md 宪法层落地；.cursorrules/.windsurfrules 收敛为指针；
  HANDOFF 修剪为索引（cbeb419）。

## 会话 2026-10-03（日）— 缩放后持续模糊（ADR-0023）

用户报 119% 文字"不是矢量化"。取证：文档是矢量、当前构建 settle 后原生
清晰（R=1.001）、但用户截图边缘过渡 ~2×（被放大的位图）。复现 + 根因：
mid-gesture 帧把 `quantize(visual)` 记成 base 缓存条目（位图从未渲染的
**幻影条目**），settle 命中幻影 → reuse=true → settle 渲染被完全跳过，
`lastRendered` 前进而像素从未重画。修复：(a) remember 仅在
`render_base_layer`（真渲染）时发生；(b) base 复用校验 key 在 TS frame
cache 真实存在（自愈）。详见 ADR-0023。验证：3 契约红→绿（wasm 24/24）、
复现脚本 R 0.812→1.0、E2E 12/12、vitest 139/139、core 269/269、clippy 干净。

## 未决事项 / 后续建议

0. **手势过程中也要清晰——A′ 已实现（2026-10-05，ADR-0026）**：core
   reknock 间隔 60→16ms（契约 271/271）+ `gesture_direct_render.ts`
   （判据/预算守卫/PROF 采样，13 契约）+ vector_host 直渲分支（缓存读
   绕过写保留，直渲 wasm 实测 0.9–2.5ms）+ 回滚 flag
   `__pdfGestureDirectRenderDisabled`。E2E `zoom_gesture_clarity_contract`
   （p50≤5%、max≤15% 1-tick 界、direct≥3；变异 flag 注入红 3 轮）；
   zoom 全套件 19/19。
   **原「下一循环的靶子：~100ms 管线 longtask 列车」已证伪（2026-10-05，
   postmortem `docs/bug-postmortems/2026-10-05-gesture-longtask-train-
   artifact.md`）**：verbose OFF（生产默认）同手势 0 长任务（5/5 运行），
   列车 = E2E 强制 verbose 追踪的诊断洪泛伪影。随修：console sink 限流
   （`diagnostics_console_rate_limit.test.ts` 4 契约，ADR-0015 前置条件
   落地）+ clarity 契约 p50 1%→5% 重钉（原值是 rAF 饥饿采样伪影）。
   归因工具留存：`zoom_longtask_attribution_probe.spec.ts`（隔相 reload
   A/C/B 设计，verbose 计时结论必查它）。
   **未决残段**：verbose ON 下列车剩余构成（分配/GC + layout 快照 + IPC）
   未归因——仅在 verbose 工效重要时才值得追。
   **渲染周期分解已测（2026-10-05，`zoom_cycle_decomposition_probe.spec.ts`，
   生产配置）**：管线**不是**瓶颈——strategy p50 12.4ms / p90 21.3 / max
   39.3（直渲 wasm 5.2ms 采样、plan-build 3.5–11ms、knock→complete p50
   36ms）。节奏真受限的是**策略层**三因：(a) **2% blur 门锯齿**——渲染后
   lastR=visual → blur≈0 → reknock 抑制，visual 需 ~100–170ms 背离 2% 才
   再敲（40% 手势帧处于抑制态，呈"突发 1–2 渲 + ~130ms 空窗"）；
   (b) wheel tick 量子（target 逐 tick 跳 10.8%，visual 指数追赶恒落后）；
   (c) **收敛尾**（9/s 档）——松手后 ~0.86s 内容才定格（缓动 + RAF 停止
   后 30ms drawing delay + 最终渲）。
   **用户视频反馈（2026-10-05）**：手势中仍不清晰（较 ADR-0026 前好转）
   + 缩放手感慢。定性 = (a)(b)(c)，非管线慢。**→ 同日 (a)(c) 修复落地
   （ADR-0026 修订注记）**：(a) `PREVIEW_REKNOCK_BLUR_THRESHOLD` 0.02 →
   **0.002（节奏地板）**——2% 门锯齿（渲染后 blur≈0 需 ~130ms 重建）证伪
   "可容忍拉伸"假设，改为任何真实移动按 16ms 节奏门持续敲击；(c) 尾段
   缓动档 **9/s → 14/s**——settle 延迟 ~860ms → **~580ms**（契约
   `zoom_reknock_cadence`：完成数 ≥36（原 32–34）、settleLatency ≤650，
   红 782 → 绿 570–585 ×3）。良性残留：最后量化档位的缓存复用拒绝
   （`requires_render=false`）造成单次 ~400-550ms 渲染静默，内容拉伸
   ≤~3%，由 zoom-state commit 收敛——**非缺陷不作指标**。(b) tick 量子
   为物理界。门禁：core 274/274（+3 契约）、clippy 双目标 0、vitest
   163/163、zoom 全套件 24/24（含新 cadence 契约）。手感的残项 = 缓动
   本体的设计（18/15/12/14 每秒四档指数），如仍觉慢属 UX 决策。
   **用户截图（2026-10-05，154% 静止态）→ 已实锤并出 postmortem
   `docs/bug-postmortems/2026-10-05-rest-blur-stale-tiles.md`，同日修复
   落地**：静止态部分模糊 + 接缝 = **旧档瓦片滞留网格**（复现：12 瓦片
   横跨 5 档；清场规则只看 presentedZoom 对 per-tile 陈旧度失明；瓦片层
   z=3 在补丁 z=2 之上压住原生层）。**修复**：tile_layer settle 态
   per-tile 陈旧扫除（`gridHasStaleTiles` → `clearPage`+`clear()` 重排；
   只清 DOM 会撞 Rust ready 死锁——契约 populated 守卫抓住）。契约
   `zoom_tile_band_uniformity`：红（worstDev 0.68，含 1.0 档页开瓦片）→
   绿（0.0000 单一档位，3 轮稳定）；zoom 全套件 23/23。
   **#3 关闭；#1（视频手势中双影/灰带）待用户在新构建上复验**——同族
   根因，静止态部分已消；手势中形态需重新取证。
   探针裁决数据保留供实现期 A/B 对照——原记录：
   **可行性探针已跑**（`zoom_reknock_cost_probe.spec.ts`，零插桩，3 轮复现），
   裁决数据：
   - **视口 reknock 渲染周期 p50=70.9–90.6ms**，其中 **worker 渲染仅
     3.1–3.5ms（~4%）**，wasm 段 1ms、blit 0.2ms 可忽略——**~95% 耗在
     worker 往返排队**（单 worker 共享 + 主线程消息延迟）。持续完成率
     8.2–9.5/s。全页渲（对照）roundMs 140–213ms。
   - **推论 1**：呈现新鲜度下限 = 渲染周期（~75ms），与 60ms 节流无关
     （节流 < 周期，`render_in_flight` 才是实际约束——解释了 P2 实测的
     补丁更新节奏）。
   - **推论 2**：候选 A 原样（worker 逐帧重渲）**不可行**——周期需 4.4–5.6×
     提速才能进 16ms 帧预算。
   - **新候选 A'（探针照出，最优先评估）主线程直渲视口 patch**：wasm 视口
     渲染本体 ~3.5ms + blit 0.2ms ≈ **周期 ~5ms < 16ms ✓**；架构先例已存在
     （overlay 渲染即主线程直渲，`vector_host.ts:888` renderPageOffscreen）。
     待验证：更高 zoom/更大视口下的真实成本（ADR-0016 像素预算封顶）、
     每帧 3.5ms 主线程占用对 rAF 的影响、单渲染链/单写者红线的落点。
   - 候选 B：GPU 矢量逐帧（vello/WebGPU，未决 #4）——绕过 worker 往返的
     根本解；候选 C：收紧档位——ADR-0025 已证加重负载增加 jank，否决。
   - 候选 A 变体"专属 detail worker"：可消 worker 侧排队，但主线程消息
     延迟是共享的，周期下限不确定，次于 A'。
   - 红线：单一渲染链、单写者（ADR-0010/0011/0014/0019/0024）、ADR-0016
     像素预算、热路径洁净。探针基建现成（tilegrid + reknock-cost 探针
     可复用作 A/B 验证）。
1. ~~P2 残余~~ — **定性更正（2026-10-04，用户确认）：设计行为，非缺陷**。
   设计契约两半：**手势中允许模糊**（内容按 3% 档渲染 + CSS 拉伸换 60fps
   跟手，reknock 把模糊压在预算内）+ **settle 必然清晰**（原生渲染 1:1）。
   P2 是前半句的可见签名。实测（探针
   `zoom_gesture_tilegrid_probe.spec.ts`，零插桩，配对诊断历史取屏幕实际
   呈现档位）：滞后约半数帧 0、瞬态峰值 15–27 档（高 zoom 区 CSS 拉伸
   ~13–14%）、**settle 后必然归零**——契约成立。已修的真 bug 均为违反
   后半（ADR-0023 幻影缓存 → settle 后仍模糊；P1 永久变体 → 静止态补丁
   滞留），分别由 ADR-0023 修复 + ADR-0024 契约 rest-window 断言守护。
   **勿再当缺陷处理**；若未来用户报"settle 后仍模糊"，那才是新缺陷
   （查 ADR-0023 类路径）。
   **2026-10-05 更新**：ADR-0026 决策后，A′ 落地将消解手势中的模糊签名
   （E2E 契约：p95 ≤3%）；届时手势中新鲜度由 ADR-0026 契约接管，本条
   "勿再当缺陷"指引在 A′ 落地前仍有效；"settle 必然清晰"后半不变。
2. ~~resize 期间锚点重置 — `syncHostLayout` 仍写居中 offset~~ —
   **2026-10-05 证伪关闭（ADR-0008 Negative 验证注记）**：该机制在当前
   架构不可达——`syncLayoutBox` 唯一调用方是调试探针
   `__pdfViewerGeometryProbe`，生产 resize 路径只有 tile_layer 的
   ViewportGeometry 失效。实证（`zoom_resize_anchor_contract.spec.ts`，
   插桩 0 次 syncHostLayout / 0 次 wrapper style 变更 / scroll 精确保持 /
   zoom 不变，2 轮逐位一致 + 变异校验红）。"resize 保持视口中心点居中"
   是增强非缺陷，未立项。**不再作为待办**。
3. ~~瓦片并行渲染~~ — **2026-10-04 实测否决**（ADR-0025）：单飞行非瓶颈
   （worker 1–4ms/张，CPU 空闲；队列深度是流式 churn 非积压）；并发泵
   A/B 显示零吞吐收益且增加 jank，已回滚。**不再作为待办**。
4. vello-wasm + WebGPU prototype（GPU 矢量逐帧金标准，ADR-0009 引用段）。
5. E2E 并发 4 worker 偶发假失败 — 判定失败前先单独重跑该 spec
   （2026-10-04 zoom 全套件 15/15 并发轮全绿，见 #6）。
6. ~~detail_overlay 契约并发 flaky~~ — **已修**（72c961d）：rAF 帧数门槛
   改语义门槛后，并发 4-worker 全套件 15/15 稳定；变异校验仍捕获（3/43
   越界）。**不再作为待办**。

## 运维：E2E 必须对打包产物跑（否则 boot 闪烁）

`tests/e2e/wdio.conf.ts` 默认用 `target/debug/pdf-viewer-standalone.exe`。
该路径可能存放两种二进制：**`tauri dev` 模式**（内嵌 devUrl=localhost:5001，
依赖 dev server）或 **`npm run e2e:build` 的打包模式**（内嵌 dist/，自包含）。
dev 模式下 E2E 会报 "app HTML never loaded" / "wheel did not change target
zoom" 等闪烁——**是二进制/环境问题，不是产品缺陷**。跑 E2E 前务必先
`npm run e2e:build`（顺带解决 `target/debug/*.exe` 被残留进程占用导致的
"failed to remove file ... os error 5"：先
`taskkill //F //IM pdf-viewer-standalone.exe` 再 build）。打包产物 E2E
**22 spec（21 跑 + 1 骨架）全绿**（2026-10-04 实测：zoom 15/15 并发轮
+ 非 zoom 6/6；`editor_bugs` 2 例为故意 `it.skip` 的编辑器骨架），比
dev-server 模式稳。

## 重要工程约束（踩过的坑）

- **改 Rust 后必须重建两个产物再跑 E2E**：`npm run wasm:pdf-viewer-ui`
  + `npm run e2e:build`。pkg 与 target/debug 都不在 git 内，否则 E2E 跑的
  是旧二进制（曾据此被误导过一次）。
- `zoom_wasm_binary.test.ts` 断言 wasm 文件 mtime < 1 小时，改 Rust 后
  不重建会失败——属预期，不是回归。
- `cargo fmt` 会顺带改动两个无关文件（`layout_engine.rs`、
  `editor_api/mod.rs` 的 import 排序），提交前需 `git checkout --` 还原。
- **E2E 并发 4 worker，偶发假失败**：同一 spec 重跑即过。判定失败前先
  单独重跑该 spec。
- 写 E2E 探针时把输出落到仓库内的文件（如 `e2e_probe.log`），
  `/tmp` 在 Git Bash 下不可靠；用完记得删。
- wdio 的多个 `--spec` 参数互相覆盖（只跑最后一个）；跑多 spec 用 glob，
  失败时逐个单跑判定。

## Commands

```bash
cargo test -p pdf-viewer-core
npx wasm-pack test --node crates/pdf-viewer-ui
npx vitest run src/__tests__/
npm run wasm:pdf-viewer-ui && npm run e2e:build   # Rust 改动后必须
npm run e2e -- --spec "tests/e2e/specs/zoom_*.spec.ts"
```

## ADR / 历史会话索引

细节全文在 `docs/adr/`（每篇含根因/修复/红灯契约/验证）与
`docs/bug-postmortems/`；源码演变见 git 历史。

| ADR | 根因 → 修复 |
|---|---|
| 0008 | 缩放起点内容滑动 → 光标锚点缩放恢复（替代 0007，已 Superseded 标注） |
| 0009 | 手势期瓦片层整体隐藏、两表面坐标系不一致（"一闪一闪、像素错位"）→ 瓦片流式 + 统一呈现公式，删除 hide/clear 开关 |
| 0010 | canvas transform 三个写者（6-12% 漂移）→ CanvasTransformOwner 单写者 + boxZoom 事实源 |
| 0011 | 表面可见性 ≥6 写者（整页空白 ~2 帧）→ PresentationSurfaceOwner，show-before-hide 原子交换 |
| 0012 | 手势中途每次滚轮整页重渲（longtask 22→5）→ prefer_viewport_tile 视口瓦片刷新 |
| 0013 | 热路径无条件日志（`__wbg_log` 507→26ms，帧成本 33→22ms）→ 删除/门控 |
| 0014 | 每帧多次 gBCR 强制同步重排（100→19ms）→ ViewportGeometry 单 owner + 帧内缓存 |
| 0015 | terminal sink 批量写 → **否决回滚**（verbose 洪水下无界队列楔死主线程），含复盘 |
| 0016 | settle 整页渲染尖峰（190–264ms 冻结，19.2M px 位图）→ max_render_pixels 预算（2× 视口），settle 0 longtask |
| 0017 | follow-up 忙环不收敛（~200 帧/s 楔死主线程，进程 446s CPU）→ `request.display_zoom = decision.target_zoom` + runaway 上限 120 |
| 0018 | 变换恰好落后一帧（两个无因果序 rAF，手势反向 4.6–5.3% 漂移）→ `dispatch_animation_frame` 同 JS turn 敲门 |
| 0019 | 滚轮步长被截断（~1/3 闪烁）→ 删除 follow-up 的 `set_zoom` 回写（渲染 actor 只读权威） |
| 0020 | buildRequest 每次 scroll 读 ×2（416→190 次/手势）→ 单次读入局部变量 |
| 0021 | peek 与 scheduleRender 背靠背各 buildRequest → 单次构建共享 |
| 0022 | `stale frame` 假 ERROR 污染错误流 → abort 判定单一所有者，降级 DEBUG（免 IPC） |
| 0023 | 幻影 base 缓存条目 → settle 持续模糊（R=0.812 不恢复）→ 条目 ⇔ 真实位图，复用校验 TS frame cache |
| 0024 | backCanvas 视口补丁无几何补偿 → 静止态"白色假页面"+手势双重曝光 → DetailOverlayOwner（第三表面纳入 ADR-0009 统一公式） |
| 0025 | 瓦片泵有界并发（上限 4）→ **已否决回滚**：A/B 实测零吞吐收益（ready 均 36）且增加 jank；队列深度是流式 churn 非积压 |
| 0026 | 手势中模糊 = worker 往返传输延迟（周期 71–91ms，渲染本体仅 ~4%）→ 主线程直渲视口补丁（A′）：精确 visual 每帧 + core 节奏门（INTERVAL 16ms）+ 跳帧守卫 + 回滚开关；预缓存 Deferred |

更早（ADR 编号前）：2026-09-27 瓦片遮蔽根因（旧 zoom 瓦片盖住 canvas）→
`docs/bug-postmortems/zoom-frame-analysis-2026-09-27.md`；2026-09-28 双重
曝光收口 + 手势期主线程饥饿治理、2026-09-29~30 canvas present 帧跳变与
平移项修复 → 各 ADR 与 git 历史（5266a48 / 5055589 / ac88fd2 / f4f66cc）。
