# Standard Library

Сейчас в проекте есть модуль `net`.

## net.ping(host)

Проверяет доступность хоста через системный `ping`.

Пример:

```pen
let ok = net.ping("8.8.8.8");
log(ok);
```

Возвращает `true` или `false`.

## net.port_open(host, port)

Проверяет открытие TCP-порта.

```pen
let ssh = net.port_open("127.0.0.1", 22);
log(ssh);
```

## net.resolve(hostname)

Возвращает IPv4-адрес по доменному имени либо `null`, если не удалось резолвить.

```pen
let ip = net.resolve("example.com");
log(ip);
```

## net.test()

Тестовая функция из `stdlib/net.py`.

## Как добавить новый модуль

1. Создать `stdlib/<module>.py`.
2. Описать функции.
3. Экспортировать их через словарь `EXPORTS`.

Пример:

```python
# stdlib/mathx.py

def _sum(args, env):
    return args[0] + args[1]

EXPORTS = {
    "sum": _sum,
}
```

После этого вызов в PenLang:

```pen
let x = mathx.sum(2, 3);
log(x);
```
