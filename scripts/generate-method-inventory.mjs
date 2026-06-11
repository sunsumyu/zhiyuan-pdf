import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const includeExt = new Set(['.rs', '.ts', '.tsx', '.js', '.mjs']);
const excludeParts = new Set(['.git', 'node_modules', 'target', 'dist', 'pkg']);
const excludedDocDirs = new Set(['archive', 'origin', 'images']);

function toPosix(p) {
  return p.split(path.sep).join('/');
}

function rel(p) {
  return toPosix(path.relative(root, p));
}

function shouldSkipDir(abs) {
  const parts = rel(abs).split('/');
  if (parts.some((part) => excludeParts.has(part))) return true;
  if (parts[0] === 'docs' && parts[1] && excludedDocDirs.has(parts[1])) return true;
  return false;
}

function collectFiles(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!shouldSkipDir(abs)) collectFiles(abs, out);
      continue;
    }
    const ext = path.extname(entry.name);
    if (!includeExt.has(ext)) continue;
    if (entry.name.endsWith('.d.ts')) continue;
    out.push(abs);
  }
  return out;
}

function countChar(text, ch) {
  let count = 0;
  for (const c of text) if (c === ch) count++;
  return count;
}

function isSnake(name) {
  return /^[a-z_][a-z0-9_]*$/.test(name);
}

function isCamel(name) {
  return /^[a-z_$][A-Za-z0-9_$]*$/.test(name) && !name.includes('_');
}

function isPascal(name) {
  return /^[A-Z][A-Za-z0-9]*$/.test(name);
}

function isCamelOrPascal(name) {
  return isCamel(name) || isPascal(name);
}

function snakeParts(name) {
  return String(name).split('_').filter(Boolean);
}

function nameParts(name) {
  const value = String(name);
  if (value.includes('_')) return snakeParts(value);
  return value
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2')
    .split(/[_$]+/)
    .filter(Boolean);
}

function isTestPath(file) {
  return /(^|\/)(tests?|__tests__)(\/|$)|\.(spec|test)\.[cm]?[jt]sx?$/.test(file);
}

function nameComplexity(item) {
  const parts = nameParts(item.name);
  return {
    length: item.name.length,
    parts: parts.length,
    sentenceLike: /(when|without|with|preserves|accounts|clamps|renders|sanitizes|keeps|changed|unchanged|missing|synthetic)/i.test(item.name),
  };
}

function isLongOrSentenceLike(item) {
  const complexity = nameComplexity(item);
  return complexity.length > 48 || complexity.parts > 7 || (complexity.sentenceLike && complexity.parts > 5);
}

function readAttributeBlockEndingAt(lines, index) {
  const block = [];
  for (let i = index; i >= 0; i--) {
    block.unshift(lines[i]);
    if (lines[i].trim().startsWith('#[')) return { block, start: i };
  }
  return null;
}

function leadingRustAttributes(lines, index) {
  const blocks = [];
  let i = index - 1;

  while (i >= 0) {
    while (i >= 0 && lines[i].trim() === '') i--;
    if (i < 0) break;

    const trimmed = lines[i].trim();
    if (trimmed.startsWith('#[')) {
      blocks.unshift(lines[i]);
      i--;
      continue;
    }

    if (trimmed === ']' || trimmed === ')]' || trimmed.endsWith(')]')) {
      const attr = readAttributeBlockEndingAt(lines, i);
      if (!attr) break;
      blocks.unshift(...attr.block);
      i = attr.start - 1;
      continue;
    }

    break;
  }

  return blocks.join('\n');
}

function extractRust(abs) {
  const text = fs.readFileSync(abs, 'utf8');
  const lines = text.split(/\r?\n/);
  const items = [];
  let braceDepth = 0;
  const implStack = [];
  const moduleStack = [];

  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const lineAttrText = leadingRustAttributes(lines, idx);
    const implMatch = line.match(/^\s*(?:unsafe\s+)?impl(?:\s*<[^>{}]*>)?\s+([^{]+?)\s*\{/);
    if (implMatch) {
      implStack.push({
        context: implMatch[1].trim().replace(/\s+/g, ' '),
        endDepth: braceDepth + countChar(line, '{') - countChar(line, '}'),
      });
    }

    const modMatch = line.match(/^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{/);
    if (modMatch) {
      const parentIsTest = moduleStack.some((module) => module.test);
      const isTestModule = parentIsTest || modMatch[1] === 'tests' || /#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]/.test(lineAttrText);
      moduleStack.push({
        test: isTestModule,
        endDepth: braceDepth + countChar(line, '{') - countChar(line, '}'),
      });
    }

    const fnRe = /\b((?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]+"\s+)?)fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<|\()/g;
    let match;
    while ((match = fnRe.exec(line)) !== null) {
      const name = match[2];
      const attrText = leadingRustAttributes(lines, idx);
      const wasmAttr = /#\s*\[\s*wasm_bindgen\b/.test(attrText);
      const rustTest = /#\s*\[\s*(?:tokio::)?test\b/.test(attrText);
      const wasm = attrText.match(/wasm_bindgen\s*\([^)]*js_name\s*=\s*"?([A-Za-z0-9_]+)"?/s);
      const tauriCommand = /#\s*\[\s*(?:tauri::)?command\s*\]/.test(attrText);
      const context = implStack.length ? implStack[implStack.length - 1].context : '';
      items.push({
        language: 'Rust',
        kind: context ? 'rust_method' : 'rust_fn',
        file: rel(abs),
        line: lineNo,
        name,
        context,
        exported: /\bpub\b/.test(match[1]) || wasmAttr || tauriCommand,
        command: tauriCommand ? name : '',
        wasmJsName: wasm?.[1] ?? '',
        wasmExported: wasmAttr,
        test: rustTest || moduleStack.some((module) => module.test),
        raw: line.trim(),
      });
    }

    braceDepth += countChar(line, '{') - countChar(line, '}');
    while (implStack.length && braceDepth < implStack[implStack.length - 1].endDepth) {
      implStack.pop();
    }
    while (moduleStack.length && braceDepth < moduleStack[moduleStack.length - 1].endDepth) {
      moduleStack.pop();
    }
  });

  return items;
}

const tsMethodKeywords = new Set([
  'if', 'for', 'while', 'switch', 'catch', 'function', 'return', 'new',
  'typeof', 'await', 'else', 'do', 'class', 'interface', 'type',
]);

function extractTs(abs) {
  const text = fs.readFileSync(abs, 'utf8');
  const lines = text.split(/\r?\n/);
  const items = [];
  let braceDepth = 0;
  const classStack = [];

  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const classMatch = line.match(/^\s*(?:export\s+)?(?:default\s+)?class\s+([A-Za-z_$][A-Za-z0-9_$]*)/);
    if (classMatch) {
      classStack.push({
        context: classMatch[1],
        endDepth: braceDepth + countChar(line, '{') - countChar(line, '}'),
      });
    }

    const context = classStack.length ? classStack[classStack.length - 1].context : '';

    const functionMatch = line.match(/^\s*(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*(?:<[^>]+>)?\s*\(/);
    if (functionMatch) {
      items.push({
        language: 'TS/JS',
        kind: 'function',
        file: rel(abs),
        line: lineNo,
        name: functionMatch[1],
        context: '',
        exported: /^\s*export\b/.test(line),
        command: '',
        wasmJsName: '',
        test: isTestPath(rel(abs)),
        raw: line.trim(),
      });
    }

    const arrowMatch = line.match(/^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*(?::[^=]+)?=\s*(?:async\s*)?(?:\([^)]*\)|[A-Za-z_$][A-Za-z0-9_$]*)\s*=>/);
    const fnExprMatch = line.match(/^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*(?::[^=]+)?=\s*(?:async\s+)?function\b/);
    const variableFn = arrowMatch?.[1] ?? fnExprMatch?.[1];
    if (variableFn) {
      items.push({
        language: 'TS/JS',
        kind: arrowMatch ? 'arrow_fn' : 'function_expr',
        file: rel(abs),
        line: lineNo,
        name: variableFn,
        context: '',
        exported: /^\s*export\b/.test(line),
        command: '',
        wasmJsName: '',
        test: isTestPath(rel(abs)),
        raw: line.trim(),
      });
    }

    if (context) {
      const methodMatch = line.match(/^\s*(?:(?:public|private|protected|static|async|get|set|override|readonly)\s+)*([A-Za-z_$][A-Za-z0-9_$]*)\s*(?:<[^>]+>)?\s*\([^)]*\)\s*(?::[^={]+)?\s*\{/);
      if (methodMatch && !tsMethodKeywords.has(methodMatch[1])) {
        items.push({
          language: 'TS/JS',
          kind: 'class_method',
          file: rel(abs),
          line: lineNo,
          name: methodMatch[1],
          context,
          exported: false,
          command: '',
          wasmJsName: '',
          test: isTestPath(rel(abs)),
          raw: line.trim(),
        });
      }
    }

    const objectMethod = line.match(/^\s*([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*(?:async\s*)?(?:\([^)]*\)|[A-Za-z_$][A-Za-z0-9_$]*)\s*=>/);
    if (objectMethod && !tsMethodKeywords.has(objectMethod[1])) {
      items.push({
        language: 'TS/JS',
        kind: 'object_arrow_method',
        file: rel(abs),
        line: lineNo,
        name: objectMethod[1],
        context: '',
        exported: false,
        command: '',
        wasmJsName: '',
        test: isTestPath(rel(abs)),
        raw: line.trim(),
      });
    }

    braceDepth += countChar(line, '{') - countChar(line, '}');
    while (classStack.length && braceDepth < classStack[classStack.length - 1].endDepth) {
      classStack.pop();
    }
  });

  return items;
}

function extractItems(abs) {
  const ext = path.extname(abs);
  if (ext === '.rs') return extractRust(abs);
  return extractTs(abs);
}

function extractRustTypes(abs) {
  const text = fs.readFileSync(abs, 'utf8');
  const lines = text.split(/\r?\n/);
  const types = [];

  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const typeMatch = line.match(/^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(struct|enum|trait|type)\s+([A-Za-z_][A-Za-z0-9_]*)\b/);
    if (!typeMatch) return;
    types.push({
      language: 'Rust',
      kind: `rust_${typeMatch[1]}`,
      file: rel(abs),
      line: lineNo,
      name: typeMatch[2],
      exported: /^\s*pub\b/.test(line),
      test: isTestPath(rel(abs)),
      raw: line.trim(),
    });
  });

  return types;
}

function extractTsTypes(abs) {
  const text = fs.readFileSync(abs, 'utf8');
  const lines = text.split(/\r?\n/);
  const types = [];

  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const typeMatch = line.match(/^\s*(?:export\s+)?(?:default\s+)?(class|interface|type|enum)\s+([A-Za-z_$][A-Za-z0-9_$]*)\b/);
    if (!typeMatch) return;
    types.push({
      language: 'TS/JS',
      kind: `ts_${typeMatch[1]}`,
      file: rel(abs),
      line: lineNo,
      name: typeMatch[2],
      exported: /^\s*export\b/.test(line),
      test: isTestPath(rel(abs)),
      raw: line.trim(),
    });
  });

  return types;
}

function extractTypeItems(abs) {
  const ext = path.extname(abs);
  if (ext === '.rs') return extractRustTypes(abs);
  return extractTsTypes(abs);
}

function mdEscape(value) {
  return String(value ?? '').replace(/\|/g, '\\|').replace(/\r?\n/g, ' ');
}

function groupBy(items, keyFn) {
  const map = new Map();
  for (const item of items) {
    const key = keyFn(item);
    if (!map.has(key)) map.set(key, []);
    map.get(key).push(item);
  }
  return map;
}

const files = collectFiles(root).sort((a, b) => rel(a).localeCompare(rel(b)));
const items = files.flatMap(extractItems).sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line || a.name.localeCompare(b.name));
const typeItems = files.flatMap(extractTypeItems).sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line || a.name.localeCompare(b.name));

const byLanguage = groupBy(items, (item) => item.language);
const byKind = groupBy(items, (item) => item.kind);
const typesByKind = groupBy(typeItems, (item) => item.kind);
const rustNamingViolations = items.filter((item) => item.language === 'Rust' && !isSnake(item.name));
const tauriCommands = items.filter((item) => item.command);
const tauriViolations = tauriCommands.filter((item) => !isSnake(item.command));
const wasmExports = items.filter((item) => item.wasmJsName);
const bareWasmExports = items.filter((item) => item.wasmExported && !item.wasmJsName);
const wasmNameViolations = wasmExports.filter((item) => !isCamelOrPascal(item.wasmJsName));
const bareWasmNameViolations = bareWasmExports.filter((item) => !isCamelOrPascal(item.name));
const tsNamingViolations = items.filter((item) => item.language === 'TS/JS' && !isCamelOrPascal(item.name));
const longOrSentenceNames = items.filter(isLongOrSentenceLike);
const productionLongOrSentenceNames = longOrSentenceNames.filter((item) => !item.test && !isTestPath(item.file));
const testLongOrSentenceNames = longOrSentenceNames.filter((item) => item.test || isTestPath(item.file));
const typeNamingViolations = typeItems.filter((item) => !isPascal(item.name));
const longOrSentenceTypeNames = typeItems.filter(isLongOrSentenceLike);
const historyNames = items.filter((item) => /(v\d+|sovereign|audit)/i.test(item.name) || /(v\d+|sovereign|audit)/i.test(item.raw));
const helperNames = items.filter((item) => /(helper|manager|misc|utils?)/i.test(item.name) || /(helper|manager|misc|utils?)/i.test(item.file));

const commandStrings = [];
const invokeRe = /\b(?:targetInvokeV3|target_invoke|smart_invoke|invoke)\s*\(\s*['"`]([A-Za-z0-9_:-]+)['"`]/g;
for (const abs of files) {
  const text = fs.readFileSync(abs, 'utf8');
  let m;
  while ((m = invokeRe.exec(text)) !== null) {
    const prefix = text.slice(0, m.index);
    const line = prefix.split(/\r?\n/).length;
    commandStrings.push({ file: rel(abs), line, command: m[1] });
  }
}

const commandStringViolations = commandStrings.filter((item) => !/^[a-z][a-z0-9_:-]*$/.test(item.command));

const inventory = [];
inventory.push('# 方法与类型清单');
inventory.push('');
inventory.push('> 由 `node scripts/generate-method-inventory.mjs` 生成。');
inventory.push('> 范围：`.rs`、`.ts`、`.tsx`、`.js`、`.mjs`；排除 `node_modules/`、`target/`、`dist/`、生成的 `pkg/`、归档/origin 文档。');
inventory.push('> 这是静态提取，宏生成方法和运行时动态创建函数不包含在内。');
inventory.push('');
inventory.push(`- 扫描源码文件：${files.length}`);
inventory.push(`- 方法/函数数量：${items.length}`);
inventory.push(`- 类型/类数量：${typeItems.length}`);
for (const [language, list] of byLanguage) inventory.push(`- ${language}: ${list.length}`);
inventory.push('');
inventory.push('## 方法类型统计');
inventory.push('');
inventory.push('| 类型 | 数量 |');
inventory.push('|---|---:|');
for (const [kind, list] of [...byKind.entries()].sort((a, b) => b[1].length - a[1].length)) {
  inventory.push(`| ${kind} | ${list.length} |`);
}
inventory.push('');
inventory.push('## 类型/类统计');
inventory.push('');
inventory.push('| 类型 | 数量 |');
inventory.push('|---|---:|');
for (const [kind, list] of [...typesByKind.entries()].sort((a, b) => b[1].length - a[1].length)) {
  inventory.push(`| ${kind} | ${list.length} |`);
}
inventory.push('');
inventory.push('## 全部方法');
inventory.push('');

for (const [file, fileItems] of groupBy(items, (item) => item.file)) {
  inventory.push(`### \`${file}\``);
  inventory.push('');
  inventory.push('| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |');
  inventory.push('|---:|---|---|---|---|---|');
  for (const item of fileItems) {
    const command = item.command || item.wasmJsName || (item.wasmExported ? item.name : '');
    inventory.push(`| ${item.line} | ${item.kind} | ${mdEscape(item.context)} | \`${mdEscape(item.name)}\` | ${item.exported ? 'yes' : ''} | ${mdEscape(command)} |`);
  }
  inventory.push('');
}

inventory.push('## 类型和类');
inventory.push('');
for (const [file, fileTypes] of groupBy(typeItems, (item) => item.file)) {
  inventory.push(`### \`${file}\``);
  inventory.push('');
  inventory.push('| 行 | 类型 | 名称 | 是否导出 |');
  inventory.push('|---:|---|---|---|');
  for (const item of fileTypes) {
    inventory.push(`| ${item.line} | ${item.kind} | \`${mdEscape(item.name)}\` | ${item.exported ? 'yes' : ''} |`);
  }
  inventory.push('');
}

const audit = [];
audit.push('# 方法命名约束审查');
audit.push('');
audit.push('> 由 `node scripts/generate-method-inventory.mjs` 与 `docs/method-inventory.md` 一起生成。');
audit.push('');
audit.push('## 范围和规则来源');
audit.push('');
audit.push('- `docs/architecture-principles.md`：单一渲染链、单一 owner、TS 作为宿主适配层、命名禁忌。');
audit.push('- `docs/architecture-overview.md`：Rust core / Rust WASM / Tauri / TS 分层边界、命令命名。');
audit.push('- `docs/development-guide.md`：Rust `fn` 使用 snake_case，Tauri command 使用 snake_case，WASM `js_name` 使用 camelCase，TS facade 命名。');
audit.push('- 本生成文档遵循的 Codex 工作约束：优先项目现有模式，生成物放在 `docs/`，避免无关重写。');
audit.push('');
audit.push('## 摘要');
audit.push('');
audit.push(`- 提取方法/函数总数：${items.length}`);
audit.push(`- 提取类型/类总数：${typeItems.length}`);
audit.push(`- Tauri commands：${tauriCommands.length}`);
audit.push(`- 显式 WASM js_name 导出：${wasmExports.length}`);
audit.push(`- WASM 推断 JS 名导出：${bareWasmExports.length}`);
audit.push(`- raw invoke/targetInvoke 命令字符串：${commandStrings.length}`);
audit.push(`- Rust 命名违规：${rustNamingViolations.length}`);
audit.push(`- Tauri command 命名违规：${tauriViolations.length}`);
audit.push(`- WASM js_name 命名违规：${wasmNameViolations.length}`);
audit.push(`- WASM 推断名违规：${bareWasmNameViolations.length}`);
audit.push(`- TS/JS 命名异常：${tsNamingViolations.length}`);
audit.push(`- 长/句子式方法名：${longOrSentenceNames.length}`);
audit.push(`- 生产代码长/句子式方法名：${productionLongOrSentenceNames.length}`);
audit.push(`- 测试代码长/句子式方法名：${testLongOrSentenceNames.length}`);
audit.push(`- 类型/类名违规：${typeNamingViolations.length}`);
audit.push(`- 长/句子式类型/类名：${longOrSentenceTypeNames.length}`);
audit.push(`- 历史标签/版本标签命中：${historyNames.length}`);
audit.push(`- helper/manager/utils 命中：${helperNames.length}`);
audit.push('');

audit.push('## 命令约束检查');
audit.push('');
audit.push('### Tauri Commands');
audit.push('');
audit.push(tauriViolations.length === 0
  ? '- 通过：提取到的 Tauri command 函数名全部是 snake_case。'
  : '- 失败：部分 Tauri command 函数名不是 snake_case。');
audit.push('');
audit.push('| 文件 | 行 | Command |');
audit.push('|---|---:|---|');
for (const item of tauriCommands) {
  audit.push(`| \`${item.file}\` | ${item.line} | \`${item.command}\` |`);
}
audit.push('');

audit.push('### Raw Invoke 命令字符串');
audit.push('');
audit.push(commandStringViolations.length === 0
  ? '- 通过：提取到的 raw invoke 命令字符串全部符合小写 snake/kebab 兼容模式。'
  : '- 警告：部分 raw invoke 命令字符串不符合预期小写命令模式。');
audit.push('');
audit.push('| 文件 | 行 | Command | 状态 |');
audit.push('|---|---:|---|---|');
for (const item of commandStrings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line)) {
  const ok = /^[a-z][a-z0-9_:-]*$/.test(item.command);
  audit.push(`| \`${item.file}\` | ${item.line} | \`${item.command}\` | ${ok ? 'ok' : 'check'} |`);
}
audit.push('');

audit.push('### WASM js_name 导出');
audit.push('');
audit.push(wasmNameViolations.length === 0
  ? '- 通过：显式 WASM `js_name` 导出全部符合 camelCase/PascalCase。'
  : '- 警告：部分显式 WASM `js_name` 导出不符合 camelCase/PascalCase。');
audit.push('');
audit.push('| 文件 | 行 | Rust fn | js_name | 状态 |');
audit.push('|---|---:|---|---|---|');
for (const item of wasmExports) {
  const ok = isCamelOrPascal(item.wasmJsName);
  audit.push(`| \`${item.file}\` | ${item.line} | \`${item.name}\` | \`${item.wasmJsName}\` | ${ok ? 'ok' : 'check'} |`);
}
audit.push('');

audit.push('### WASM 推断名导出');
audit.push('');
audit.push(bareWasmNameViolations.length === 0
  ? '- 通过：裸 `#[wasm_bindgen]` 函数推断出的 JS 名符合 camelCase/PascalCase。'
  : '- 警告：裸 `#[wasm_bindgen]` 函数会把 Rust 名推断到 JS；snake_case 项应加显式 camelCase `js_name`，或作为 legacy 导出退役。');
audit.push('');
audit.push('| 文件 | 行 | Rust fn | 推断 JS 名 | 状态 |');
audit.push('|---|---:|---|---|---|');
for (const item of bareWasmExports) {
  const ok = isCamelOrPascal(item.name);
  audit.push(`| \`${item.file}\` | ${item.line} | \`${item.name}\` | \`${item.name}\` | ${ok ? 'ok' : 'check'} |`);
}
audit.push('');

audit.push('## 类型和类名');
audit.push('');
audit.push(typeNamingViolations.length === 0
  ? '- 通过：提取到的 Rust/TS 类型和类名全部使用 PascalCase。'
  : '- 警告：部分 Rust/TS 类型或类名不是 PascalCase。');
audit.push('');
audit.push('| 文件 | 行 | 类型 | 长度 | 分段数 | 名称 | 状态 |');
audit.push('|---|---:|---|---:|---:|---|---|');
for (const item of typeItems) {
  const complexity = nameComplexity(item);
  const namingOk = isPascal(item.name);
  const long = isLongOrSentenceLike(item);
  const status = namingOk && !long ? 'ok' : [namingOk ? '' : 'case', long ? 'long/semantic' : ''].filter(Boolean).join(', ');
  audit.push(`| \`${item.file}\` | ${item.line} | ${item.kind} | ${complexity.length} | ${complexity.parts} | \`${mdEscape(item.name)}\` | ${status} |`);
}
audit.push('');

function emitFindingTable(title, intro, list, limit = 80) {
  audit.push(`## ${title}`);
  audit.push('');
  audit.push(intro);
  audit.push('');
  audit.push('| 文件 | 行 | 类型 | 上下文 | 名称 |');
  audit.push('|---|---:|---|---|---|');
  for (const item of list.slice(0, limit)) {
    audit.push(`| \`${item.file}\` | ${item.line} | ${item.kind} | ${mdEscape(item.context)} | \`${mdEscape(item.name)}\` |`);
  }
  if (list.length > limit) audit.push(`| ... | ... | ... | ... | 还有 ${list.length - limit} 项省略；见方法清单 |`);
  audit.push('');
}

emitFindingTable(
  'Rust 命名异常',
  rustNamingViolations.length
    ? '以下 Rust 函数名不符合 snake_case，应对照 `docs/architecture-principles.md` 命名规则审查。'
    : '未发现 Rust snake_case 异常。',
  rustNamingViolations,
);

emitFindingTable(
  'TS/JS 命名异常',
  tsNamingViolations.length
    ? '以下 TS/JS 名称不符合 camelCase/PascalCase。部分可能是测试回调或框架要求名称，但仍应检查。'
    : '未发现 TS/JS 命名异常。',
  tsNamingViolations,
);

audit.push('## 长/句子式方法名');
audit.push('');
audit.push(longOrSentenceNames.length
  ? '以下名称过长，或像行为场景句。文档反对用方法名描述整条 workflow/process；测试名可以描述场景，但过长时应迁到聚焦测试模块，并使用更短 case 名。'
  : '未发现长/句子式方法名。');
audit.push('');
audit.push('| 文件 | 行 | 类型 | 测试 | 长度 | 分段数 | 名称 |');
audit.push('|---|---:|---|---|---:|---:|---|');
for (const item of longOrSentenceNames.slice(0, 120)) {
  const complexity = nameComplexity(item);
  const isTest = item.test || isTestPath(item.file);
  audit.push(`| \`${item.file}\` | ${item.line} | ${item.kind} | ${isTest ? '是' : ''} | ${complexity.length} | ${complexity.parts} | \`${mdEscape(item.name)}\` |`);
}
if (longOrSentenceNames.length > 120) {
  audit.push(`| ... | ... | ... | ... | ... | ... | 还有 ${longOrSentenceNames.length - 120} 项省略；见方法清单 |`);
}
audit.push('');

emitFindingTable(
  '历史标签或版本标签命中',
  '文档反对 `v3`、`audit`、`sovereign` 等历史标签，临时日志 tag 除外。以下命中应清理或给出理由。',
  historyNames,
);

emitFindingTable(
  'Helper/Manager/Utils 命名命中',
  '文档反对模糊的 helper/manager/utils 命名，明确临时用途除外。以下位置需要审查；并非每一项都一定错误。',
  helperNames,
);

audit.push('## 架构边界观察');
audit.push('');
audit.push('- TS 拥有大量 DOM/canvas 宿主函数，这是预期的。但任何 TS 方法如果负责页面准入、渲染队列、字体、glyph、PDF 语义决策，都应迁回或镜像到 Rust/WASM。');
audit.push('- `targetInvokeV3`/raw invoke 字符串仍是跨边界命令面。命令名本身大多合规，但类型安全弱于文档要求的 facade/session 方向。');
audit.push('- 清单故意包含测试和脚本，便于区分测试专用方法与生产方法。');
audit.push('');
audit.push('## 建议后续动作');
audit.push('');
audit.push('1. 将本脚本接入 CI，对高置信约束失败：长/句子式方法名、Tauri command snake_case、显式 WASM `js_name` camelCase、新增裸 WASM snake_case 导出。');
audit.push('2. 先缩短或迁移长测试名，尤其是 `draft_layout.rs` 和生产模块内联 `#[cfg(test)] mod tests`。');
audit.push('3. 审查历史/版本标签命中，把活跃运行时命名改为中性名称；只保留兼容 alias 或日志 tag。');
audit.push('4. 用 typed bridge 方法或现有领域 facade 包装 raw invoke 字符串，逐步减少裸调用。');
audit.push('5. 单独审查 TS render/presentation 方法，确认每个方法只是宿主适配，还是持有了应迁到 WASM 的决策。');

fs.mkdirSync(path.join(root, 'docs'), { recursive: true });
fs.writeFileSync(path.join(root, 'docs', 'method-inventory.md'), inventory.join('\n'), 'utf8');
fs.writeFileSync(path.join(root, 'docs', 'method-constraint-audit.md'), audit.join('\n'), 'utf8');

console.log(JSON.stringify({
  files: files.length,
  methods: items.length,
  types: typeItems.length,
  tauriCommands: tauriCommands.length,
  wasmExports: wasmExports.length,
  bareWasmExports: bareWasmExports.length,
  commandStrings: commandStrings.length,
  rustNamingViolations: rustNamingViolations.length,
  tauriViolations: tauriViolations.length,
  wasmNameViolations: wasmNameViolations.length,
  bareWasmNameViolations: bareWasmNameViolations.length,
  tsNamingViolations: tsNamingViolations.length,
  longOrSentenceNames: longOrSentenceNames.length,
  productionLongOrSentenceNames: productionLongOrSentenceNames.length,
  testLongOrSentenceNames: testLongOrSentenceNames.length,
  typeNamingViolations: typeNamingViolations.length,
  longOrSentenceTypeNames: longOrSentenceTypeNames.length,
  historyNames: historyNames.length,
  helperNames: helperNames.length,
}, null, 2));
