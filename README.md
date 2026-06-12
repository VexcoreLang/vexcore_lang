# Vexcore

Vexcore — минималистичный скриптовый язык для пентест-задач и лабораторных сценариев.

Проект полностью переписан на Rust и собран как Cargo workspace.

## Быстрый старт

```bash
# запуск скрипта
cargo run -- test.vcl

# запуск REPL
cargo run -- --repl

# запуск с отладочной информацией
cargo run -- test.vcl --debug

# запуск без использования кэша (всегда перекомпилирует)
cargo run -- test.vcl --no-cache
```

## Кэширование парсинга

VexCore автоматически кэширует скомпилированный bytecode в файлы `.vcbc`:

```
script.vcl → (парсинг+компиляция) → script.vcbc (сохраняется)
                               ↓
           второй запуск загружает готовый script.vcbc
```

**Результаты:**
- compute.vcl: **2.3x ускорение** (6.64ms → 2.85ms)
- imports.vcl: **5.3x ускорение** (1.83ms → 0.34ms)

Кэш автоматически инвалидируется при изменении `.vcl` файла.


## Минимальный `.vcl` файл

```vcl
log("hello");
```

## Что уже поддерживается

- типы: `int`, `float`, `str`, `bool`, `null`, `list`, `json`
- переменные и мягкая типизация через аннотации
- `if / elf / els`, `while`, `for (in range)`, `for (in list)`
- функции пользователя (`fn`, `return`)
- импорты через `use "file.vcl";`
- встроенные функции: `log`, `run`
- модуль `net`: `ping`, `port_open`, `resolve`

## Пример

```vcl
use "libtest.vcl";

let host = "8.8.8.8";
let alive = net.ping(host);
log(f"host {host} alive = {alive}");

log(sum(1, 2));
```

## Документация

- [docs/01-getting-started.md](docs/01-getting-started.md) — установка и запуск
- [docs/02-language-reference.md](docs/02-language-reference.md) — синтаксис и семантика
- [docs/03-stdlib.md](docs/03-stdlib.md) — встроенные функции и `net`
- [docs/04-vscode-extension.md](docs/04-vscode-extension.md) — работа с VS Code
- [docs/05-extending-language.md](docs/05-extending-language.md) — как расширять язык

## Лицензия

См. файл `LICENSE`.
