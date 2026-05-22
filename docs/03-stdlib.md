# Standard Library

## Встроенные функции

### `log(value)`

Печатает значение в stdout.

```vcl
log("hello");
log(123);
```

### `run(cmd)`

Выполняет shell-команду, возвращает stdout как строку.

```vcl
let out = run("echo test");
log(out);
```

## Модуль `net`

### `net.ping(host) -> bool`

Проверяет доступность хоста через системный `ping`.

```vcl
let ok = net.ping("8.8.8.8");
log(ok);
```

### `net.port_open(host, port) -> bool`

Проверяет, открыт ли TCP-порт.

```vcl
let ssh = net.port_open("127.0.0.1", 22);
log(ssh);
```

### `net.resolve(hostname) -> str | null`

Пробует резолвить DNS-имя, возвращает IP или `null`.

```vcl
let ip = net.resolve("example.com");
log(ip);
```

## Расширение stdlib

Архитектура сделана так, чтобы добавлять функциональность точечно.

Новый built-in:

- добавить функцию в `crates/stdlib/src/lib.rs`
- зарегистрировать её в `register_builtins()` (`// EXTENSION POINT`)

Новый модуль (`http`, `fs`, ...):

- создать модуль в `crates/stdlib/src/lib.rs` или вынести в отдельный файл
- зарегистрировать его в `register_modules()` (`// EXTENSION POINT`)

Интерпретатор и парсер при этом менять не нужно.
