# PDF 编辑策略：内存重建与坐标同步规范

> [!NOTE]
> 本文档定义了“Nushell Enhanced”项目中 PDF 编辑的核心架构原则。具体的 FFI 技术细节和 0.8.x 升级坑点，请参阅[项目专家知识库 (Skill)](file:///e:/chain/nushell-enhanced/.agents/skills/pdf-expertise/SKILL.md)。

## 1. 核心架构逻辑 (Architecture)

### 内存修改 -> 全量重建 (Memory-First)
不再采用极易导致损坏的“字节流打补丁”方案。所有编辑操作均通过 PDFium 的内存对象模型完成。

1.  **加载**：使用 `load_pdf_from_file` 将文档载入内存。
2.  **查找**：定位到目标 `PdfPageTextObject`（利用 Backend 提供的 Object ID）。
3.  **修改**：通过 `set_text` 精准更新内容，或通过 `translate` 修改位置。
4.  **导出**：调用 `doc.save_to_file`。此时 PDFium 会自动重新计算内部偏移，生成合法的新文件。

## 2. 坐标原点同步规范 (Origin Synchronization)

为了解决“点不准”的问题，必须遵循统一的 **3000px 虚拟分辨率** 标准：

- **Backend (Render)**: 渲染宽度强制设为 `3000px`。
- **Backend (Model)**: 提取坐标时转换到 `3000px` 空间，并减去 `PdfPageBoundaries` 的 `crop.left()` 偏移。
- **Frontend (UI)**: 依据 `containerWidth / 3000` 计算显示比率。

## 3. 文字提取与编辑 (Text Processing)

- **WYSIWYG 模式**：通过 HTML `contentEditable` 实现直观编辑。
- **高精度判定**：后端必须使用 `loose_bounds()` 提供字符级包围盒，确保前端高亮框与视觉文字完美契合。
- **字体管理**：所有动态添加的文字默认使用 `Helvetica` 以确保广泛的跨平台兼容性。

---

> [!IMPORTANT]
> **后续开发要求**：即使引入 OCR 或 图像编辑功能，也必须保持“解析 -> 修改对象树 -> 重新导出”的逻辑链路，保障 PDF 文件的二进制合法性。
