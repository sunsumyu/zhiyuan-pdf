# PDF 工业级全景渲染引擎终极路线图 (V6 Architecture)

## 编者按与架构参考

在成功实现了核心的矢量坐标抽离（Native Path）和多维层叠的 Form XObject 递归引擎后，我们需要在保障稳定性的前提下，安全、增量地吃透剩余的 PDF 页面元素（不仅包括因位图而丢失的 Logo，还包括最为复杂的工业级文字渲染）。

**业界大厂架构参考**：
1. **PDF.js (Mozilla)**：采用 `CanvasGraphics` 设计模式，对于极度复杂的连篇文本布局、字体 CID/CMap 解码，均采用了在 JavaScript 引擎内手工维护字体矩阵，将解析的字形通过 `@font-face` 直接挂载给 Browser DOM 进行高速呈现，把图形相交（Clipping）运算交还给底层 HTML5 Canvas API。
2. **PDFium (Chrome)**：C++ 层面维持极其庞大的状态机。所有的 `CPDF_TextObject` 和 `CPDF_ImageObject` 均为独立对象。文字呈现硬绑定于 FreeType，因此内存开销通常较大。

**我们的纯 Rust (Tauri Backend) 路线选择 (先简后难，边界隔离)**：
借鉴 PDF.js “解析器提供结构化元数据，渲染器执行纯绘制”的理念，我们要在 `vector_engine.rs` 这个 Rust Parse 引擎里做到“只提取裸数据、绝不擅自做平台耦合的绘制”。

下面是我们用**“小步快跑、不要改太多”**战术实现“完美纯 Rust PDF 解析引擎”的实施阶段计划，确保每做一步，都能用 `[PDF-XXX-AUDIT]` 日志与之前 PDFium 的输出进行严格像素级对齐验证。

---

## 小步快跑实施阶段计划 (Incremental MVP Breakdown)

### Phase 1: Image XObject 底层提取 (位图 Logo 恢复｜高 ROI，低代码侵入)
由于之前测试的全页面均未触发 Form 递归，故那些丢失的印章、发票 Logo 极大的概率是位图。
*   **目标**：恢复所有的扫描版和插入型像素图片。
*   **步骤设计**：
    1.  继续拦截 `Do` 算子，如果查询到的 XObject 字典里的 `Subtype` 为 `Image`，获取原始数据流。
    2.  读取元数据：`Width`, `Height`, `ColorSpace`，并运用适当的机制将解压的原生像素转化为 Base64 URI（或保留在内存让前端调取）。
    3.  融合当前计算得到的 CTM `(a,b,c,d,e,f)`，封装至 `NativeImageModel`。
    4.  **验证标准**：启动 [PDF-IMAGE-AUDIT] 日志，核对图片的 (X,Y,W,H) 是否与此前 PDFium 提取的包围盒数字对齐。

### Phase 2: Dash Pattern 虚线支持 (细节完备｜极其简单)
*   **目标**：表格中的虚线和引导线准确度恢复，避免财务报表将虚线识别为粗实线。
*   **步骤设计**：
    1.  扩展 `GraphicsState`，增加 `dash_array: Vec<f32>` 和 `dash_phase: f32` 成员。
    2.  新增处理 `d` (Set line dash pattern) 算子。
    3.  提取进 `NativePathModel` 并持久化序列化给前端处理。
    4.  **验证标准**：通过日志输出 PDF 页面中明确的虚线条，对比参数长度。

### Phase 3: 图形剪裁范围 (Clipping Paths - `W / W*`｜中等复杂度)
*   **目标**：防止多边形覆盖越界，这在复杂的出版级 PDF 中很常见。
*   **步骤设计**：
    1.  不再粗暴清空缓存路径，而是提供 `clip_mode`。
    2.  将 Clipping Path 送入栈中。若底层运算过重，我们的退路是：只把当前算子所代表的包围盒作为 `clip_box` 带在 `RenderObject` 上，交由浏览器 `ctx.clip()` 免费完成。
    3.  **验证标准**：页面没有突兀的矢量溢出。

### Phase 4: 原生文本抽取 (工业级 V5 架构 - 步进 1｜硬核骨架)
*   **目标**：剥离文字内容及布局信息，脱离 PDFium 获取文字。
*   **步骤设计**：
    1.  构建完整的 Text State 状态机：拦截 `BT` (Begin Text), `ET` (End Text) 获取上下文边界。
    2.  识别排版控制：`Tf` (基于 FontDict 的字号), `Td/TD` (换行与平移), `Tm` (文字矩阵，极重要，关系到字怎么歪脖子排布)。
    3.  拦截显示打印：`Tj` (打印流) 与 `TJ` (含 Kerning 字距微调的打印数组)。
    4.  这步**只会输出裸二进制 C-String (如 `[0,12,0,45]`) 和原神坐标**，作为 `NativeTextModel` 的初期雏形。

### Phase 5: CMap / CID 字体破译 (工业级 V5 架构 - 步进 2｜极高复杂度)
*   **目标**：终结乱码噩梦，把 Phase 4 提取的字节阵列，映射为真实的 UTF-8 字符（如“开票日期”）。
*   **步骤设计**：
    1.  解析 Font Dictionary 中的 `ToUnicode` stream。
    2.  在 Rust 侧仿写并建立起一套 CMap 解析器，破译 `bfrange` (段落式映射) 和 `bfchar` (单点映射) 语法。
    3.  组合构建出字体引擎结构，专门处理 Type0 (CID) 加密的特种字符。
    4.  **验证标准**：用真实的 PDF 报表验证提取出的中文字符全部可识别不乱码，且能完美复制高亮。

---

## 本周执行建议 (Next Action)

遵循**“一步不要改太多”**的原则，我强烈建议我们现在**只动一处代码**：**实施 Phase 1 (Image XObject)**。

> 理由：它是我们修复“渲染图片缺失”最有力的武器，且在当前架构下只需要在 `parse_content_stream` 的 `Do` 分支增加三四十行代码，验证极其低风险、易对齐，完全符合小步快跑思想。
