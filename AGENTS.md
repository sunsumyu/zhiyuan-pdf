# AGENTS.md — 纸鸢（pdf-viewer-standalone）仓库宪法

Tauri 桌面 PDF 查看器/编辑器。Rust workspace（`pdf-viewer-core` 纯计算 /
`pdf-viewer-ui` wasm 渲染代理 / `pdf-viewer-standalone` 壳）+ TS 呈现桥
（`src/bridge`）。**最高风险区 = 缩放渲染管线**（docs/adr/0009~0023 的主
战场）：所有可见像素由 Rust canvas 绘制，TS 只做几何呈现与交互。

本文件是唯一宪法，冲突时以本文件为准。每条门禁必须可验证——配命令或
指向确定文件。

## 〇、死门禁（违反即缺陷，与测试是否绿无关）

1. 【单一渲染链】所有可见像素只走一条链：core 解析 → pdf-viewer-ui paint
   plan → Rust canvas 绘制 → DOM canvas。严禁浏览器字体承担显示职责、
   严禁 TS `fillText`/DOM 补绘、严禁白底遮罩"修复"未抑制对象
   （docs/architecture-principles.md §1）。
2. 【单写者铁律】每项几何/状态能力只有一个 writer，新增写路径前先查
   docs/adr/，不得旁路：
   - canvas transform → `CanvasTransformOwner`（ADR-0010）
   - backCanvas（视口补丁）left/top/transform 视觉映射 →
     `DetailOverlayOwner`（ADR-0024）
   - 页面表面 display/visibility → `PresentationSurfaceOwner`（ADR-0011）
   - viewport 几何读 → `ViewportGeometry`（ADR-0014）
   - 缩放权威 → `ZOOM_STATE` 单入口；渲染侧 actor/follow-up 只读，
     **严禁回写 target_zoom**（ADR-0019）；base 缓存条目 ⇔ 真实存在的
     位图（ADR-0023）
3. 【契约先行】行为修复必须先写红灯契约（修复前红）再实现。严禁实现与
   测试同发自证（AI 自测反模式：测试围绕错误实现断言）。
4. 【知识随码入库】ADR/postmortem/HANDOFF 与对应源码必须同批 commit。
   严禁让已验证工作滞留未提交工作区（2026-10-03 教训：15 篇 ADR 曾
   untracked 裸奔多日）。
5. 【热路径洁净】渲染/动画路径严禁无条件日志与 `JSON.stringify`；
   error 流只留真故障，abort/stale frame 归 DEBUG（ADR-0013/0022）。
6. 【E2E 前置】改 Rust 后必须先 `npm run wasm:pdf-viewer-ui && npm run
   e2e:build` 再跑 E2E，否则跑的是旧二进制。E2E 失败先单独重跑该 spec
   再定性（并发 4 worker 偶发假失败）。

## 一、验证命令（验收只认退出码，不认口头）

| 门禁 | 命令 |
|---|---|
| 核心单测 | `cargo test -p pdf-viewer-core`（274 @ 2026-10-06；数字以 HANDOFF 最近门禁记录为准，防漂移） |
| wasm 契约 | `npx wasm-pack test --node crates/pdf-viewer-ui` |
| TS 契约 | `npx vitest run src/__tests__/`（含 wasm mtime 哨兵：改 Rust 后不重建必红，属预期） |
| lint | `cargo clippy`（native + wasm32 双目标，0 warning） |
| E2E | `npm run e2e -- --spec "tests/e2e/specs/zoom_*.spec.ts"`（先 e2e:build） |
| TS 构建 | `tsc && vite build` |

## 二、三层边界与命名

- **pdf-viewer-core**：纯计算（lopdf 解析、字形布局、几何推断）。严禁读取
  "前端当前页/哪页过期"等交互状态。
- **pdf-viewer-ui**（wasm 边缘）：只做入参反序列化 + 委派（`*_api.rs`）；
  过期请求 Early-Abort 拦截，严禁渗透进计算层。
- **src/bridge（TS）**：用户交互、DOM 几何呈现。严禁二次绘制像素；
  `renderVectorPageWithPlan` 的 stale frame 中止必须优雅转 `{aborted:true}`，
  严禁触发无意义预览 fallback。
- 命名：`*_service.rs` 有状态业务；`*_store.rs` thread_local 全局态；
  `*_api.rs` wasm/JS 边缘。DOM 几何写入只能表达为 SurfaceOp
  （SetBox/SetTransform/SetDisplay，见 CONTEXT.md）。
- 交互即时性：翻页等动作先同步更新页码 DOM，不等重型渲染。

## 三、指针目录（按需加载，勿一次全读）

| 文档 | 何时读 |
|---|---|
| docs/architecture-principles.md | 动渲染/编辑架构前（铁律全文：单一渲染链、单 owner 表、三层边界） |
| docs/adr/0009~0026 | 改缩放渲染管线前（必读 0010/0011/0014/0019/0023/0026） |
| docs/bug-postmortems/ | 排渲染类缺陷前（帧级取证方法与案例） |
| docs/HANDOFF.md | 接手会话时（最新状态 + 运维坑 + 未决事项 + ADR 索引） |
| CONTEXT.md | 命名/词汇分歧时（zoom authority、SurfaceOp、FrameToken…） |
| docs/development-guide.md | 构建与开发流程 |

## 四、踩坑速查（历史命中率最高）

- `target/debug` 二进制被 `tauri dev` 覆盖成 dev 模式 → E2E 报
  "app HTML never loaded"：先 `npm run e2e:build`。
- TEMP 盘满（scoped_dir* 写不进）→ E2E 全灭 "session not created"：清临时目录。
- wdio 多个 `--spec` 参数互相覆盖（只跑最后一个）：用 glob，失败逐个单跑。
- `cargo fmt` 会顺带改 `layout_engine.rs` / `editor_api/mod.rs` 的 import
  排序：提交前 `git checkout --` 还原。
- E2E 探针输出写仓库内文件（`/tmp` 在 Git Bash 下不可靠），用完删除。

## 五、工作流

行为修复：取证（探针/录屏逐帧/CDP）→ postmortem 落 docs/bug-postmortems/
→ 红灯契约 → 最小实现 → 门禁全绿 → 与 ADR/HANDOFF 同批 commit。
单会话只吃一个修复循环；阶段产物外化到磁盘后再清空上下文。
