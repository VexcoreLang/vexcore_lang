const fs = require('fs');
const path = require('path');
const vscode = require('vscode');

const BUILTIN_SYMBOLS = {
  log: {
    label: 'log(value: any): null',
    detail: 'Built-in function',
    documentation: 'Prints a value to stdout with a newline.'
  },
  out: {
    label: 'out(value: any): null',
    detail: 'Built-in function',
    documentation: 'Prints a value to stdout without a newline.'
  },
  outln: {
    label: 'outln(value: any): null',
    detail: 'Built-in function',
    documentation: 'Prints a value to stdout with a newline. Alias for log().'
  },
  run: {
    label: 'run(cmd: str): str',
    detail: 'Built-in function',
    documentation: 'Executes a shell command and returns trimmed stdout as a string.'
  }
};

const BUILTIN_MODULES = {
  net: {
    ping: {
      label: 'net.ping(host: str): bool',
      documentation: 'Returns whether the host responds to a ping.'
    },
    port_open: {
      label: 'net.port_open(host: str, port: int): bool',
      documentation: 'Returns whether the TCP port is reachable.'
    },
    resolve: {
      label: 'net.resolve(hostname: str): str | null',
      documentation: 'Resolves the first IP address for a hostname.'
    }
  },
  udp: {
    test: {
      label: 'udp.test(): bool',
      documentation: 'Test UDP module functionality. Returns true if successful.'
    }
  }
};

const KEYWORDS = [
  'let',
  'if',
  'elf',
  'els',
  'while',
  'for',
  'in',
  'fn',
  'return',
  'use',
  'true',
  'false',
  'null'
];

function activate(context) {
  context.subscriptions.push(
    vscode.languages.registerCompletionItemProvider(
      { language: 'penlang', scheme: 'file' },
      {
        provideCompletionItems(document, position) {
          const linePrefix = document.lineAt(position).text.slice(0, position.character);
          const index = buildModuleIndex(document);
          const moduleContext = getModuleCompletionContext(linePrefix);

          if (moduleContext) {
            return buildModuleMemberCompletions(moduleContext, index);
          }

          const items = [];
          items.push(...buildKeywordCompletions());
          items.push(...buildBuiltinCompletions());
          items.push(...buildDocumentSymbolCompletions(document, index));
          items.push(...buildModuleNameCompletions(index));
          return dedupeCompletions(items);
        }
      },
      '.',
      ...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_'
    )
  );

  context.subscriptions.push(
    vscode.languages.registerHoverProvider({ language: 'penlang', scheme: 'file' }, {
      provideHover(document, position) {
        const identifier = getIdentifierAt(document, position);
        if (!identifier) {
          return;
        }

        const index = buildModuleIndex(document);
        const meta = resolveSignatureMetadata(identifier, index);
        if (!meta) {
          return;
        }

        return new vscode.Hover(buildHoverMarkdown(meta));
      }
    })
  );

  context.subscriptions.push(
    vscode.languages.registerSignatureHelpProvider(
      { language: 'penlang', scheme: 'file' },
      {
        provideSignatureHelp(document, position) {
          const contextInfo = getCallContext(document, position);
          if (!contextInfo) {
            return;
          }

          const index = buildModuleIndex(document);
          const meta = resolveSignatureMetadata(contextInfo.name, index);
          if (!meta) {
            return;
          }

          const help = new vscode.SignatureHelp();
          const signature = new vscode.SignatureInformation(
            meta.label,
            new vscode.MarkdownString(meta.documentation)
          );

          signature.parameters = meta.params.map(
            (param) => new vscode.ParameterInformation(param)
          );
          help.signatures = [signature];
          help.activeSignature = 0;
          help.activeParameter = contextInfo.activeParameter;
          return help;
        }
      },
      '(',
      ','
    )
  );
}

function deactivate() { }

function buildKeywordCompletions() {
  return KEYWORDS.map((kw) => new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
}

function buildBuiltinCompletions() {
  const items = [];

  for (const [name, meta] of Object.entries(BUILTIN_SYMBOLS)) {
    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
    item.detail = meta.label;
    item.documentation = new vscode.MarkdownString(meta.documentation);
    items.push(item);
  }

  for (const [moduleName, members] of Object.entries(BUILTIN_MODULES)) {
    const moduleItem = new vscode.CompletionItem(moduleName, vscode.CompletionItemKind.Module);
    moduleItem.detail = `${moduleName} stdlib module`;
    moduleItem.documentation = new vscode.MarkdownString(
      `Module with functions: ${Object.keys(members).join(', ')}`
    );
    items.push(moduleItem);
  }

  return items;
}

function buildDocumentSymbolCompletions(document, moduleIndex) {
  const items = [];

  for (const [moduleName, moduleMeta] of Object.entries(moduleIndex.modules)) {
    const item = new vscode.CompletionItem(moduleName, vscode.CompletionItemKind.Module);
    item.detail = moduleMeta.source;
    item.documentation = new vscode.MarkdownString(
      `Imported module from \`${moduleMeta.importPath}\``
    );
    items.push(item);
  }

  for (const [name, meta] of Object.entries(moduleIndex.symbols)) {
    if (name.includes('.')) {
      continue;
    }
    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
    item.detail = meta.label;
    item.documentation = new vscode.MarkdownString(meta.documentation);
    items.push(item);
  }

  return items;
}

function buildModuleNameCompletions(moduleIndex) {
  const items = [];

  for (const [moduleName, moduleMeta] of Object.entries(moduleIndex.modules)) {
    const item = new vscode.CompletionItem(moduleName, vscode.CompletionItemKind.Module);
    item.detail = moduleMeta.source;
    item.documentation = new vscode.MarkdownString(
      `Imported module from \`${moduleMeta.importPath}\``
    );
    items.push(item);
  }

  for (const moduleName of Object.keys(BUILTIN_MODULES)) {
    const item = new vscode.CompletionItem(moduleName, vscode.CompletionItemKind.Module);
    item.detail = 'Stdlib module';
    items.push(item);
  }

  return items;
}

function buildModuleMemberCompletions(moduleContext, moduleIndex) {
  const items = [];
  const { moduleName, memberPrefix } = moduleContext;
  const builtinMembers = BUILTIN_MODULES[moduleName];
  if (builtinMembers) {
    for (const [name, meta] of Object.entries(builtinMembers)) {
      if (memberPrefix && !name.startsWith(memberPrefix)) {
        continue;
      }
      const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
      item.insertText = name;
      item.detail = meta.label;
      item.documentation = new vscode.MarkdownString(meta.documentation);
      item.sortText = `0_${name}`;
      items.push(item);
    }
  }

  const userModule = moduleIndex.modules[moduleName];
  if (userModule) {
    for (const [name, meta] of Object.entries(userModule.members)) {
      if (memberPrefix && !name.startsWith(memberPrefix)) {
        continue;
      }
      const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
      item.insertText = name;
      item.detail = meta.label;
      item.documentation = new vscode.MarkdownString(meta.documentation);
      item.sortText = `0_${name}`;
      items.push(item);
    }
  }

  return dedupeCompletions(items);
}

function buildModuleIndex(document) {
  const visited = new Set();
  const baseDir = path.dirname(document.uri.fsPath);
  const modules = {};

  collectModuleFromDocument(document, baseDir, visited, modules);

  return {
    symbols: collectSymbols(document),
    modules
  };
}

function collectModuleFromDocument(document, baseDir, visited, modules) {
  const current = document.uri.fsPath;
  if (visited.has(current)) {
    return;
  }
  visited.add(current);

  const imports = collectImports(document.getText());
  for (const importPath of imports) {
    const resolved = path.resolve(baseDir, importPath);
    const moduleName = path.basename(importPath, path.extname(importPath));
    if (!fs.existsSync(resolved)) {
      continue;
    }

    const importedText = fs.readFileSync(resolved, 'utf8');
    const memberMap = collectFnSignatures(importedText);
    modules[moduleName] = {
      importPath,
      source: 'Imported module',
      members: memberMap
    };

    const nestedDoc = {
      uri: { fsPath: resolved },
      getText: () => importedText
    };
    collectModuleFromDocument(nestedDoc, path.dirname(resolved), visited, modules);
  }
}

function collectImports(text) {
  const imports = [];
  const re = /^\s*use\s+"([^"]+)"\s*;?/gm;
  let match;
  while ((match = re.exec(text)) !== null) {
    imports.push(match[1]);
  }
  return imports;
}

function collectSymbols(document) {
  const symbols = {};
  const text = document.getText();

  for (const [name, meta] of Object.entries(BUILTIN_SYMBOLS)) {
    symbols[name] = normalizeBuiltinMeta(meta);
  }

  for (const [moduleName, members] of Object.entries(BUILTIN_MODULES)) {
    for (const [name, meta] of Object.entries(members)) {
      symbols[`${moduleName}.${name}`] = normalizeBuiltinMeta({
        label: meta.label,
        documentation: meta.documentation
      });
    }
  }

  for (const [name, meta] of Object.entries(collectFnSignatures(text))) {
    symbols[name] = meta;
  }

  return symbols;
}

function collectFnSignatures(text) {
  const symbols = {};
  const fnRe = /^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*(?::\s*([A-Za-z_][A-Za-z0-9_]*))?/gm;
  let match;
  while ((match = fnRe.exec(text)) !== null) {
    const [, name, paramsText, returnType] = match;
    const params = parseParams(paramsText);
    const retType = returnType || 'null';
    symbols[name] = {
      label: formatSignature(name, params, retType),
      params,
      returnType: retType,
      documentation: buildFunctionDocs(name, params, retType)
    };
  }
  return symbols;
}

function resolveSignatureMetadata(identifier, moduleIndex) {
  if (moduleIndex.symbols[identifier]) {
    return moduleIndex.symbols[identifier];
  }

  if (identifier.includes('.')) {
    const [moduleName, memberName] = identifier.split('.', 2);
    const builtinModule = BUILTIN_MODULES[moduleName];
    if (builtinModule && builtinModule[memberName]) {
      return normalizeBuiltinMeta({
        label: builtinModule[memberName].label,
        documentation: builtinModule[memberName].documentation
      });
    }
    const userModule = moduleIndex.modules[moduleName];
    if (userModule && userModule.members[memberName]) {
      return userModule.members[memberName];
    }
  }

  if (BUILTIN_SYMBOLS[identifier]) {
    return normalizeBuiltinMeta(BUILTIN_SYMBOLS[identifier]);
  }

  return null;
}

function normalizeBuiltinMeta(meta) {
  const params = extractParamsFromLabel(meta.label);
  return {
    label: meta.label,
    params,
    returnType: extractReturnTypeFromLabel(meta.label),
    documentation: meta.documentation
  };
}

function extractParamsFromLabel(label) {
  const open = label.indexOf('(');
  const close = label.indexOf(')', open + 1);
  if (open < 0 || close < 0) {
    return [];
  }
  const paramsText = label.slice(open + 1, close).trim();
  if (!paramsText) {
    return [];
  }
  return paramsText.split(',').map((part) => part.trim());
}

function extractReturnTypeFromLabel(label) {
  const idx = label.indexOf('):');
  if (idx < 0) {
    return 'null';
  }
  return label.slice(idx + 2).trim();
}

function parseParams(paramsText) {
  if (!paramsText.trim()) {
    return [];
  }
  return paramsText
    .split(',')
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const colonIndex = part.indexOf(':');
      if (colonIndex >= 0) {
        const name = part.slice(0, colonIndex).trim();
        const type = part.slice(colonIndex + 1).trim();
        return `${name}: ${type || 'any'}`;
      }
      return `${part}: any`;
    });
}

function formatSignature(name, params, returnType) {
  return `${name}(${params.join(', ')}): ${returnType}`;
}

function buildFunctionDocs(name, params, returnType) {
  return `\`${formatSignature(name, params, returnType)}\``;
}

function buildHoverMarkdown(meta) {
  const markdown = new vscode.MarkdownString();
  markdown.appendCodeblock(meta.label, 'vcl');
  markdown.appendMarkdown(`\n\n${meta.documentation}`);
  return markdown;
}

function getModuleCompletionContext(linePrefix) {
  const match = linePrefix.match(/([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z0-9_]*)$/);
  if (!match) {
    return null;
  }

  return {
    moduleName: match[1],
    memberPrefix: match[2] || ''
  };
}

function getIdentifierAt(document, position) {
  const line = document.lineAt(position.line).text;
  let start = position.character;
  let end = position.character;

  while (start > 0 && /[A-Za-z0-9_.]/.test(line.charAt(start - 1))) {
    start -= 1;
  }
  while (end < line.length && /[A-Za-z0-9_.]/.test(line.charAt(end))) {
    end += 1;
  }

  const value = line.slice(start, end).trim();
  return value || null;
}

function getCallContext(document, position) {
  const text = document.getText(
    new vscode.Range(new vscode.Position(0, 0), position)
  );

  let depth = 0;
  for (let i = text.length - 1; i >= 0; i -= 1) {
    const ch = text[i];
    if (ch === ')') {
      depth += 1;
      continue;
    }
    if (ch === '(') {
      if (depth > 0) {
        depth -= 1;
        continue;
      }

      const before = text.slice(0, i).trimEnd();
      const match = before.match(/([A-Za-z_][A-Za-z0-9_.]*)$/);
      if (!match) {
        return null;
      }

      const argsText = text.slice(i + 1);
      const activeParameter = countActiveParameters(argsText);
      return {
        name: match[1],
        activeParameter
      };
    }
  }

  return null;
}

function countActiveParameters(argsText) {
  let depth = 0;
  let active = 0;
  for (let i = 0; i < argsText.length; i += 1) {
    const ch = argsText[i];
    if (ch === '(' || ch === '[' || ch === '{') {
      depth += 1;
      continue;
    }
    if (ch === ')' || ch === ']' || ch === '}') {
      depth = Math.max(0, depth - 1);
      continue;
    }
    if (ch === ',' && depth === 0) {
      active += 1;
    }
  }
  return active;
}

function dedupeCompletions(items) {
  const seen = new Set();
  return items.filter((item) => {
    const key = `${item.label}::${item.kind}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

module.exports = {
  activate,
  deactivate
};
