# VS Code Extension

Папка расширения: `vscode-penlang/`.

## Что уже реализовано

- Регистрация языка `penlang` для файлов `.vcl` и `.vcl`
- Подсветка синтаксиса (TextMate grammar)
- Базовая language configuration (скобки, автозакрытие, комментарии)
- Snippets для частых конструкций
- Базовые автоподсказки:
  - ключевые слова (`fn`, `return`, `use` тоже)
  - модуль `net`
  - функции `net.*`

## Установка расширения локально

1. Открыть `vscode-penlang/` как отдельный проект в VS Code.
2. Запустить `npm install`.
3. Нажать `F5` (Run Extension).
4. В новом Extension Host открыть `.vcl` файл.
