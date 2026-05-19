# Getting Started

## Требования

- Python 3.10+
- Linux (из-за `ping` параметров в stdlib/net.py)

## Структура `.vcl` файла

Минимальная структура:

```pen
[setts]
cpu=1;
ram=1024;
mem=0;

[scenary]
log("hello");
```

## Запуск

```bash
python3 run.py test.vcl
```

REPL:

```bash
python3 run.py --repl
```

Debug режим:

```bash
python3 run.py test.vcl --debug
```

## Первый практический пример

```pen
[scenary]
let host: str = "8.8.8.8";
let alive: bool = net.ping(host);
log(f"host {host} alive = {alive}");
```
