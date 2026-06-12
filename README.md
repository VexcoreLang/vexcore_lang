# Vexcore

Vexcore — минималистичный скриптовый язык для пентест-задач и лабораторных сценариев.

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

Кэш автоматически инвалидируется при изменении `.vcl` файла.


## Минимальный `.vcl` файл

```vcl
outln("hello");
```

## Краткое описание синтаксиса

- типы: `int`, `float`, `str`, `bool`, `null`, `list`, `json`
- переменные и мягкая типизация через аннотации
- `if / elf / els`, `while`, `for (in range)`, `for (in list)`
- функции пользователя (`fn`, `return`)
- импорты через `use "file.vcl";`
- встроенные функции: `outln`, `run`
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

- [docs/README.md](docs/README.md)
