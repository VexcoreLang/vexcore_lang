const vscode = require('vscode');

const KEYWORDS = [
  'let', 'if', 'elf', 'els', 'while', 'for', 'in',
  'run', 'log', 'fn', 'return', 'use', 'true', 'false', 'null'
];

const MODULE_MEMBERS = {
  net: ['ping', 'port_open', 'resolve', 'test']
};

function activate(context) {
  const provider = vscode.languages.registerCompletionItemProvider(
    { language: 'penlang', scheme: 'file' },
    {
      provideCompletionItems(document, position) {
        const linePrefix = document
          .lineAt(position)
          .text
          .slice(0, position.character);

        const items = [];

        for (const kw of KEYWORDS) {
          const item = new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword);
          items.push(item);
        }

        for (const moduleName of Object.keys(MODULE_MEMBERS)) {
          const moduleItem = new vscode.CompletionItem(moduleName, vscode.CompletionItemKind.Module);
          moduleItem.detail = 'PenLang stdlib module';
          items.push(moduleItem);
        }

        if (linePrefix.endsWith('net.')) {
          for (const fn of MODULE_MEMBERS.net) {
            const item = new vscode.CompletionItem(fn, vscode.CompletionItemKind.Function);
            item.insertText = fn;
            item.detail = 'net module function';
            items.push(item);
          }
        }

        if (/\bnet\.[A-Za-z_]*$/.test(linePrefix)) {
          for (const fn of MODULE_MEMBERS.net) {
            const item = new vscode.CompletionItem(fn, vscode.CompletionItemKind.Function);
            item.insertText = fn;
            item.detail = 'net module function';
            items.push(item);
          }
        }

        return items;
      }
    },
    '.',
    ...'abcdefghijklmnopqrstuvwxyz_'
  );

  context.subscriptions.push(provider);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate
};
