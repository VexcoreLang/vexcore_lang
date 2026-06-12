# Language Reference

## Структура файла

Файл `.vcl` теперь состоит из обычных инструкций без секций и обёрток.

```vcl
let name = "vexcore";
outln(f"hello {name}");
```

Поддерживаются:

- комментарии через `#`
- объявления переменных
- функции
- `if / elf / els`
- `while`
- `for`
- `use "file.vcl";`

## Типы

- `int`
- `float`
- `str`
- `bool`
- `null`
- `list`
- `json`

Примеры:

```vcl
let a = 1;
let b = 3.14;
let s = "hello";
let flag = true;
let n = null;
let arr = [1, 2, 3];
let obj = {"key": 1};
```

## Аннотации и мягкое приведение

```vcl
let port: int = "8080";
let mark: str = 404;
```

Если приведение невозможно, будет runtime-ошибка.

## Операторы

Арифметика:

```text
+ - * / // % **
```

Сравнение:

```text
== != < > <= >=
```

Логика:

```text
&& || !
```

Диапазон:

```text
1..10
```

Присваивание:

```text
= += -= *= /=
```

## Управляющие конструкции

```vcl
if (x > 0) {
  outln("positive");
} elf (x == 0) {
  outln("zero");
} els {
  outln("negative");
}

while (x < 3) {
  x += 1;
}

for (i in 1..4) {
  outln(i);
}

for (item in arr) {
  outln(item);
}
```

## Функции

```vcl
fn sum(a: int, b: int): int {
  return a + b;
}

let result = sum(5, 7);
outln(result);
```

`return;` без значения возвращает `null`.

## Коллекции и индексация

```vcl
let data = {"key": [1, 2, 3]};
outln(data["key"][0]);
```

## Интерполяция строк

```vcl
let host = "8.8.8.8";
let alive = net.ping(host);
outln(f"host {host} alive = {alive}");
```

## Импорты

```vcl
use "libtest.vcl";
```

## Расширение языка

Если ты хочешь добавить новый keyword, тип данных или новую конструкцию, см. отдельный гайд:

- [05-extending-language.md](./05-extending-language.md)
