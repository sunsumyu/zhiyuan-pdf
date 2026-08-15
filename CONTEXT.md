# 纸鸢 Zhiyuan — Domain Glossary

## PDF 算符与状态

- **TextState** — 共享文本状态字段（font_size, char_spacing, word_spacing, horizontal_scaling, render_mode, tl）+ 矩阵操作（op_cm, op_bt, op_tm, op_td, op_t_star），嵌入在读路径 GraphicsState 和写路径 PdfTextState 中。定义于 `text_state.rs`。
- **TextMatrixCore** — PDF 文本矩阵三元组（ctm/tm/tlm）及不变量操作（concat_ctm, begin_text, set_text_matrix, translate_text, advance_text, text_render_matrix）。读写路径共享的最底层抽象。
- **GraphicsState** — 读路径状态：TextState + 图形专属字段（line_width, line_cap, line_join, miter_limit, stroke/fill color, alpha, current_font）。
- **PdfTextState** — 写路径状态：TextState + 写入专属字段（font_alias 字节数组）。仅在 pdf_write.rs 内部可见。

## 路径

- **读路径** — 内容流解析（content_parser.rs）：遍历 PDF 算符，构建 RenderObject/StyledRun 向量。使用 GraphicsState 追踪状态。
- **写路径** — 内容流修补（pdf_write.rs）：遍历 PDF 算符，应用文本重排补丁（TextReflowPatch），发射修改后的算符。使用 PdfTextState 追踪状态。

## 字体

- **字体解析链** — pdf_write_font/ 子目录：finder（系统字体查找）→ face（TTF 解析 + 字形子集提取）→ embed（PDF 对象构造）。三模块间通过 SystemFont 数据契约连接。
- **ParsedFont** — 从系统字体或嵌入字体解析得到的字体数据结构，包含字形映射、度量信息、原始字节。
