# VexCore Language - Новые возможности

## ✨ Реализованные функции

### 1. **Неймспейс для пользовательских модулей**

Теперь вы можете импортировать модули с собственным неймспейсом через `as`:

```vcl
use "my_lib.vcl" as mylib;
mylib.my_function();
```

**Пример:**
```vcl
use "helper.vcl" as h;
outln(h.add(5, 3));        // 8
outln(h.multiply(4, 7));   // 28
```

**Плюсы:**
- Избегаете конфликтов имён между модулями
- Код читается чётче с явным указанием источника функции
- Можно импортировать один файл несколько раз с разными именами

### 2. **Префикс `std` для стандартной библиотеки**

Теперь для вызова функций стандартной библиотеки используется префикс `std`:

```vcl
// Новый синтаксис (рекомендуется)
std.net.ping("127.0.0.1");
std.net.port_open("localhost", 8080);
std.net.resolve("example.com");

// Старый синтаксис (всё ещё работает)
net.ping("127.0.0.1");  // Тоже валидно
```

**Пример:**
```vcl
outln("=== Test: std prefix ===");

if (std.net.ping("127.0.0.1")) {
    outln("Localhost is reachable");
}

let ip = std.net.resolve("localhost");
outln("Resolved IP: " + ip);
```

**Плюсы:**
- Ясно видно, что функция из стандартной библиотеки
- Нельзя случайно переопределить встроенные функции
- Возможность в будущем добавить конкуренцию между `std` и пользовательскими модулями

## 📚 Документация

### Как расширять стандартную библиотеку

Полная документация находится в `docs/STDLIB_EXTENSION.md`. Там описано:

1. Как создавать новые модули стандартной библиотеки
2. Структура проекта
3. Примеры реализации функций
4. Лучшие практики

**Быстрый пример добавления модуля `str_utils`:**

Создайте файл `crates/stdlib/src/str_utils.rs`:

```rust
pub mod str_utils {
    use super::{ModuleFn, StdlibError};
    use ast::Value;
    use std::collections::HashMap;
    use std::sync::Arc;

    pub fn register() -> HashMap<String, ModuleFn> {
        let mut map: HashMap<String, ModuleFn> = HashMap::new();
        map.insert("upper".to_string(), upper as ModuleFn);
        map.insert("lower".to_string(), lower as ModuleFn);
        map
    }

    fn upper(args: Vec<Value>) -> Result<Value, StdlibError> {
        match args.first() {
            Some(Value::Str(s)) => Ok(Value::Str(Arc::from(s.to_uppercase()))),
            _ => Err(StdlibError::Message("expects string".to_string())),
        }
    }

    fn lower(args: Vec<Value>) -> Result<Value, StdlibError> {
        match args.first() {
            Some(Value::Str(s)) => Ok(Value::Str(Arc::from(s.to_lowercase()))),
            _ => Err(StdlibError::Message("expects string".to_string())),
        }
    }
}
```

Затем в `lib.rs` добавьте регистрацию:

```rust
pub mod str_utils;

// В функции register_modules():
modules.insert("str_utils".to_string(), str_utils::register());
```

Используйте в VexCore:

```vcl
outln(std.str_utils.upper("hello"));    // "HELLO"
outln(std.str_utils.lower("WORLD"));    // "world"
```

## 🧪 Тестовые файлы

Созданы примеры использования новых функций:

- **test_ns.vcl** - Тестирование пользовательских модулей с неймспейсом
- **test_std.vcl** - Тестирование std префикса для stdlib
- **test_old_syntax.vcl** - Проверка обратной совместимости
- **helper.vcl** - Вспомогательные функции для тестов

**Запуск тестов:**

```bash
cargo run --bin vexcore test_ns.vcl
cargo run --bin vexcore test_std.vcl
cargo run --bin vexcore test_old_syntax.vcl
```

## 🔧 Технические изменения

### AST (crates/ast/src/lib.rs)
- Изменён `Stmt::Use` с `Use(String)` на `Use { path: String, namespace: Option<String> }`
- Добавлен новый вариант `Expr::StdModuleCall { module, function, args }`

### Лексер (crates/lexer/src/lib.rs)
- Добавлен токен `TokenKind::As`

### Парсер (crates/parser/src/lib.rs)
- Обновлён `parse_use_stmt()` для обработки синтаксиса `as namespace`
- Добавлена новая функция `split_std_module_call()` для распознавания `std.module.function()`
- Обновлён `parse_postfix()` для создания `StdModuleCall` вместо `ModuleCall` когда есть `std` префикс

### Интерпретатор (crates/interpreter/src/lib.rs)
- Обновлены типы `Stmt::Use` и `Instr::Use` для поддержки namespace
- Обновлена функция `run_use()` для использования namespace
- Добавлен новый `Instr::CallStdModule` для вызова std модулей
- Добавлена функция `call_std_module()` которая всегда обращается к stdlib, не к user modules

## ✅ Статус

- ✅ Поддержка `use "file.vcl" as namespace;`
- ✅ Поддержка `std.module.function()` синтаксиса
- ✅ Обратная совместимость (старый синтаксис всё ещё работает)
- ✅ Документация для расширения stdlib
- ✅ Примеры и тесты

## 🚀 Следующие шаги

Возможные улучшения:

1. Добавить встроенные модули для работы с файлами (`std.file.read()`, etc)
2. Добавить модуль для работы со строками (`std.str.upper()`, etc)
3. Добавить модуль для математики (`std.math.sqrt()`, etc)
4. Добавить модуль для JSON обработки (`std.json.parse()`, etc)
5. Улучшить обработку ошибок при импорте модулей
