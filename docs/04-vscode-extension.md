# VS Code Extension

В репозитории есть папка `vscode-penlang/` с заготовкой расширения для `.vcl`.

## Что уже есть

- регистрация языка
- TextMate-подсветка
- snippets
- базовые подсказки

## Локальный запуск

```bash
cd vscode-penlang
npm install
```

Открыть папку расширения в VS Code и нажать `F5` (Run Extension).

## Замечание

Расширение осталось совместимым с текущим синтаксисом Vexcore, но это отдельный tooling-пакет. Рантайм языка находится в Rust workspace и не зависит от VS Code-части.
