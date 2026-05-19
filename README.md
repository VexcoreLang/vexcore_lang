# Vexcore 🖊️

Минималистичный скриптовый язык для пентест-лабораторий.

## Структура проекта

```
penlang/
├── run.py              # точка входа
├── core/
│   ├── lexer.py        # токенизатор
│   ├── ast_nodes.py    # узлы AST
│   ├── parser.py       # парсер → AST
│   └── interpreter.py  # исполнитель AST
├── stdlib/             # сюда добавляй модули (net, http, ...)
└── examples/
    └── test.vcl        # тестовый скрипт
```

## Запуск

```bash
# запустить файл
python run.py script.vcl

# REPL
python run.py --repl

# дебаг (токены + AST)
python run.py script.vcl --debug
```

## Синтаксис

### Структура файла
```
[setts]
cpu=1;
ram=1024;
mem=0;        # 0 = без ограничений

[scenary]
# код здесь
```

### Переменные и типы
```
let a = 1;          # int
let b = 3.14;       # float
let s = "hello";    # str
let flag = true;    # bool
let n = null;       # null
let arr = [1,2,3];  # list
let obj = {"k": 1}; # json
```

### Операторы
```
# арифметика
+ - * / // % **

# сравнение
== != < > <= >=

# логика
|| && !

# присваивание
= += -= *= /=
```

### Управление потоком
```
if (x > 0) {
    log("positive");
} elf (x == 0) {
    log("zero");
} els {
    log("negative");
}

while (a != 10) {
    a += 1;
}

for (i in 1..10) {
    log(f"i = {i}");
}

for (item in myList) {
    log(f"{item}");
}
```

### Встроенные функции
```
log("текст");           # вывод
log(f"val = {a}");      # f-string интерполяция
run("ls -la");          # выполнить shell команду
run(f"ping {host}");    # с интерполяцией
```

### JSON / доступ к элементам
```
let data = {"key": [1, 2, 3]};
log(data["key"]);       # [1, 2, 3]
log(data["key"][0]);    # 1
```

## Как расширять

### Добавить ключевое слово
1. `core/lexer.py` → добавить в `KEYWORDS`
2. `core/ast_nodes.py` → новый dataclass узла
3. `core/parser.py` → метод `_parse_XXX`, вызов в `_stmt()`
4. `core/interpreter.py` → метод `_exec_XXX` или `_eval_XXX`

### Добавить встроенную функцию (stdlib)
Пример модуля `stdlib/net.py`:
```python
def builtin_scan(args, env):
    host = args[0]
    # логика сканирования
    return results
```
Зарегистрируй в `interpreter.py` в `_builtins` словаре.

### Добавить тип данных
1. `ast_nodes.py` → новый `Literal` или обёртка
2. `interpreter.py` → обработка в `_to_str()`, `_truthy()`, `_apply_binop()`

## Документация

Подробная документация находится в папке `docs/`:

- `docs/README.md`
- `docs/01-getting-started.md`
- `docs/02-language-reference.md`
- `docs/03-stdlib.md`
- `docs/04-vscode-extension.md`

## VS Code Extension

В проект добавлено расширение VS Code в папке `vscode-penlang/` с поддержкой:

- подсветки синтаксиса `.vcl`
- snippets
- базового autocomplete (ключевые слова и `net.*`)

См. `docs/04-vscode-extension.md` для установки и запуска.
