# Getting Started

## Требования

- Rust toolchain (stable)
- Cargo
- ОС: Linux/macOS/Windows

## Проверка окружения

```bash
rustc --version
cargo --version
```

## Первый запуск

В корне проекта:

```bash
cargo check
cargo run -- test.vcl
```

## Запуск REPL

```bash
cargo run -- --repl
```

Выход: `:q` или `:quit`.

## Debug-режим

```bash
cargo run --bin vexcore -- test.vcl --debug
```

Показывает техническую информацию (например, количество токенов и выражений).

## Минимальный рабочий скрипт

```vcl
let name = "vexcore";
outln(f"hello {name}");
```

## Импорт локальной библиотеки

Файл `libtest.vcl`:

```vcl
fn sum(a: int, b: int): int {
  return a + b;
}
```

Файл `test.vcl`:

```vcl
use "libtest.vcl";
outln(sum(1, 2));
```

Запуск:

```bash
cargo run --bin vexcore -- test.vcl
```
