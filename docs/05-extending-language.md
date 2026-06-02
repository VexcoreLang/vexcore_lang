# Расширение языка

Этот файл нужен как практический чеклист, если ты хочешь добавить новую возможность в Vexcore.

## Общая схема

Почти любая новая фича проходит через одни и те же слои:

1. Лексер превращает символы в токены.
2. Парсер собирает токены в AST.
3. AST получает новые узлы или поля.
4. Интерпретатор выполняет новую конструкцию.
5. Документация и примеры обновляются вместе с кодом.

## Как добавить новый keyword

Например, ты хочешь добавить `repeat`.

1. Добавь новый вариант в `TokenKind` в [crates/lexer/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/lexer/src/lib.rs).
2. Распознай слово в `lex_ident_or_kw`.
3. Если keyword вводит новую инструкцию, добавь ветку в `parse_stmt` в [crates/parser/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/parser/src/lib.rs).
4. Добавь новый вариант `Stmt` в [crates/ast/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/ast/src/lib.rs), если это отдельная конструкция.
5. Реализуй выполнение в `eval_stmt` в [crates/interpreter/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/interpreter/src/lib.rs).
6. Обнови примеры и этот документ.

Минимальный шаблон выглядит так:

```rust
// lexer
TokenKind::Repeat,

// parser dispatch
TokenKind::Repeat => self.parse_repeat_stmt(),

// AST
Stmt::Repeat { count: Expr, body: Vec<Stmt> },

// interpreter dispatch
Stmt::Repeat { count, body } => self.eval_repeat_stmt(count, body),
```

## Как добавить новый тип данных

Если новый тип должен быть доступен в языке как `mytype`, обычно нужно обновить несколько мест:

1. Добавь новый вариант в `TypeAnnotation` в [crates/ast/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/ast/src/lib.rs).
2. Добавь новый вариант в `Value`, если нужен runtime-объект.
3. Обнови `Value::type_name`, `Value::is_truthy`, `Value::to_pretty_string` и `Value::coerce_to`.
4. Обнови `parse_type_annotation` в [crates/parser/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/parser/src/lib.rs).
5. Если нужен literal-синтаксис, добавь его в `parse_primary`.
6. Если новый тип участвует в операциях, обнови `apply_unary`, `apply_binop` или другие runtime-хелперы.
7. Добавь примеры в [docs/02-language-reference.md](./02-language-reference.md).

Пример для нового типа `bytes`:

```rust
pub enum TypeAnnotation {
    // ...
    Bytes,
}

pub enum Value {
    // ...
    Bytes(Vec<u8>),
}
```

После этого нужно решить, как он печатается, как приводится и какие операции над ним разрешены.

## Как добавить что-то новое в синтаксис

Если это новая конструкция, например `match`, лучше идти так:

1. Сначала опиши поведение на бумаге: как выглядит синтаксис, какие блоки нужны, какие ошибки допустимы.
2. Добавь токены, если они нужны.
3. Добавь AST-узел.
4. Добавь парсинг.
5. Добавь интерпретацию.
6. Добавь тестовый `.vcl` файл и пример в документацию.

Если это только новый оператор, тогда обычно достаточно:

1. Добавить токен.
2. Добавить ветку в приоритеты парсера выражений.
3. Добавить runtime-реализацию.

## Где обычно править код

- [crates/lexer/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/lexer/src/lib.rs) - токены и ключевые слова
- [crates/parser/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/parser/src/lib.rs) - синтаксис и AST-строение
- [crates/ast/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/ast/src/lib.rs) - узлы AST и runtime-значения
- [crates/interpreter/src/lib.rs](/home/master/Desktop/vexcore_lang/crates/interpreter/src/lib.rs) - выполнение и семантика
- [docs/02-language-reference.md](./02-language-reference.md) - пользовательская документация

## Практический совет

Лучше добавлять новую фичу маленькими шагами:

1. токен или AST;
2. парсер;
3. интерпретатор;
4. документация;
5. проверка на реальном `.vcl` примере.

Так проще найти, на каком слое всё сломалось.
