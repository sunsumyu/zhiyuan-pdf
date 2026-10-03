# HANDOFF — 会话交接与运维手册

> 2026-10-03 起本文件收敛为「最新会话 + 索引」。历史会话全文（723 行版）
> 存于 git 历史（b6a4920 / 8f119fc 之前的版本）；细节以 docs/adr/ 与
> docs/bug-postmortems/ 为准。仓库宪法见根目录 **AGENTS.md**。

## 最新会话 2026-10-03（晚）— 知识链入库 + 记忆层收拢

- 用户录屏（13:20，**已含 ADR-0023 修复的最新构建**）逐帧取证：缩放双重
  曝光/静止态重影的根因 = **backCanvas 陈旧视口补丁**（detail overlay 无
  几何补偿、手势结束无人隐藏、settle 复用跳渲染时存在永不消失变体）。
  全文 → `docs/bug-postmortems/2026-10-03-backcanvas-stale-patch.md`
  （P1~P4 + 修复方向，**尚未修复**）。
- 三笔提交：`b6a4920`（backfill ADR-0009~0022 + 09-27 postmortem）、
  `8f119fc`（ADR 时期源码 + 3 个 Rust 契约测试 + 8 个 vitest 契约 +
  11 个 E2E spec + CDP 工具 + HANDOFF，59 文件）、`5e282d9`（新 postmortem）。
- 提交前复跑：core 269/269 ✓；vitest 138/139（唯一失败 = wasm mtime>1h
  新鲜度哨兵，预期）。
- 本提交：AGENTS.md 宪法层落地；.cursorrules/.windsurfrules 收敛为指针；
  HANDOFF 修剪为索引。

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

1. **P1~P4 backCanvas 陈旧补丁**（本分支下一个修复目标）：含"settle 合法
   复用跳渲染时补丁永不消失"的永久变体。修复方向与契约设计见 postmortem；
   按红灯契约流程在新窗口进行。
2. resize 期间锚点重置 — `syncHostLayout` 仍写居中 offset（ADR-0008 Negative）。
3. 瓦片并行渲染 — `pumpRequest` 单飞行，一屏瓦片串行填充；若滚动仍慢可
   允许 2-3 张并行（HANDOFF 历史未决 #3）。
4. vello-wasm + WebGPU prototype（GPU 矢量逐帧金标准，ADR-0009 引用段）。
5. "手势中滚动失灵" — 假设已被探针否证（09-28 会话）；如再现需重新取证。
6. E2E 并发 4 worker 偶发假失败 — 判定失败前先单独重跑该 spec。
7. E2E 帧级/静止态缺陷盲区 — 现有契约抓不到 backCanvas 类缺陷，postmortem
   P4 提出了需新增的契约形态（快速缩小后静止逐帧采样）。

## 运维：E2E 必须对打包产物跑（否则 boot 闪烁）

`tests/e2e/wdio.conf.ts` 默认用 `target/debug/pdf-viewer-standalone.exe`。
该路径可能存放两种二进制：**`tauri dev` 模式**（内嵌 devUrl=localhost:5001，
依赖 dev server）或 **`npm run e2e:build` 的打包模式**（内嵌 dist/，自包含）。
dev 模式下 E2E 会报 "app HTML never loaded" / "wheel did not change target
zoom" 等闪烁——**是二进制/环境问题，不是产品缺陷**。跑 E2E 前务必先
`npm run e2e:build`（顺带解决 `target/debug/*.exe` 被残留进程占用导致的
"failed to remove file ... os error 5"：先
`taskkill //F //IM pdf-viewer-standalone.exe` 再 build）。打包产物 E2E
**12/12 spec 全绿**（2026-10-02/03 实测），比 dev-server 模式稳。

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

更早（ADR 编号前）：2026-09-27 瓦片遮蔽根因（旧 zoom 瓦片盖住 canvas）→
`docs/bug-postmortems/zoom-frame-analysis-2026-09-27.md`；2026-09-28 双重
曝光收口 + 手势期主线程饥饿治理、2026-09-29~30 canvas present 帧跳变与
平移项修复 → 各 ADR 与 git 历史（5266a48 / 5055589 / ac88fd2 / f4f66cc）。
