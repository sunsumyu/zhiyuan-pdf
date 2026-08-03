## 一步到位方案：Tauri 后端 vello GPU 渲染 + 前端双层 Canvas

### 核心发现

你的 **Tauri 后端已经有完整的 vello GPU 矢量渲染器**（`src-tauri/src/infrastructure/pdf/vello_renderer.rs`），支持：
- GPU 矢量路径渲染（vello + wgpu）
- 内嵌字体解析（swash + ttf-parser）
- CPU fallback（cosmic-text + swash 栅格化）
- 两阶段渲染管线

**当前问题是：渲染在前端 WASM/Canvas 完成，后端 vello 完全没有被使用。**

### 方案架构

```
┌─────────────────────────────────────────────┐
│  前端 DOM                                    │
│  ┌─────────────────────┐  ┌───────────────┐ │
│  │ <canvas> 页面位图     │  │ <canvas> 编辑 │ │
│  │ (vello GPU 渲染结果)  │  │ overlay      │ │
│  │ z-index: 1           │  │ z-index: 2   │ │
│  └─────────────────────┘  └───────────────┘ │
└─────────────────────────────────────────────┘
              ↑ Tauri command 返回 RGBA 位图
    ┌─────────────────────────────────────────┐
    │  Tauri 后端 (Rust)                       │
    │  PDF 解析 → vello GPU 渲染 → RGBA buffer │
    │  (已有完整实现)                           │
    └─────────────────────────────────────────┘
```

### 具体实施步骤

#### 阶段 1：Tauri 后端暴露渲染命令（1 周）

1. 新增 Tauri command：`render_page_to_image(document_id, page_index, zoom, width, height)`
2. 复用现有 `VelloRenderer`，传入页面数据
3. 渲染为 RGBA buffer，通过 Tauri 返回前端
4. 支持 PNG/JPEG 压缩减少传输量

#### 阶段 2：前端接收和显示（1 周）

1. 新增 `GpuPageRenderer` 类管理页面位图
2. 调用 Tauri command 获取页面图像
3. Canvas `drawImage()` / `putImageData()` 显示
4. zoom/pan 时重新请求（带缓存）

#### 阶段 3：编辑器 overlay 分离（1 周）

1. 保留现有 `GlyphPaintPlan` 在前端（用于编辑交互计算）
2. 编辑器 overlay 在独立 Canvas 层绘制
3. 编辑内容通过 Tauri command 发送到后端重新渲染
4. 后端渲染完成后前端更新页面位图

#### 阶段 4：性能优化（1 周）

1. 位图缓存（按 page × zoom 维度）
2. 预渲染相邻页面
3. 瓦片化渲染（大页面分块）
4. 编辑时前端即时反馈 + 后台异步完整渲染

### 为什么这是"一步到位"

| 维度 | 当前方案 | 本方案 |
|------|---------|--------|
| 字体渲染 | 浏览器 Canvas fillText（模糊） | vello GPU 矢量（锐利） |
| 字体来源 | CSS font-family 回退 | PDF 内嵌字体 + 系统字体 |
| 缩放质量 | 位图拉伸（锯齿） | 矢量无限缩放 |
| 抗锯齿 | 浏览器默认（一般） | MSAA16（优秀） |
| 架构改动 | 需重新发明轮子 | 复用已有 vello 渲染器 |

### 关键优势

1. **后端 vello 已完整可用** — 不需要重新实现
2. **内嵌字体可以被正确渲染** — 后端能读取 PDF 字体二进制
3. **GPU 矢量渲染** — 清晰度达到商业级
4. **前端编辑交互不受影响** — overlay 层保持现有逻辑

### 风险和对策

| 风险 | 对策 |
|------|------|
| zoom/pan 延迟 | 位图缓存 + 预渲染 + 瓦片化 |
| 内存占用 | LRU 缓存 + 及时释放不可见页面 |
| 编辑实时性 | 前端即时 Canvas 反馈 + 后台异步完整渲染 |
| 跨进程传输开销 | PNG/JPEG 压缩 + 只传输 viewport 区域 |

### 预期工期

**3-4 周**（比在前端重新实现 vello 的 2-3 个月要现实得多）

### 下一步

如果你确认这个方案，我将从阶段 1 开始实施：在 Tauri 后端暴露 `render_page_to_image` command。