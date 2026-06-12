# Расширение стандартной библиотеки VexCore

Этот документ описывает как расширять стандартную библиотеку VexCore, добавляя новые модули с собственными функциями.

## Структура стандартной библиотеки

Стандартная библиотека находится в `crates/stdlib/src/lib.rs`. Она состоит из:
- **Встроенные функции** (builtins): `out`, `outln`, `log`, `run`
- **Модули**: `net`, `udp` (и вы можете добавлять свои)

Каждый модуль содержит набор функций, которые доступны через синтаксис `module.function()`.

## Два способа структурировать код

### Способ 1: Все в одном файле (как `net`)

Для простых модулей вы можете определить их прямо в `lib.rs`:

```rust
mod my_module {
    use super::{ModuleFn, StdlibError};
    use ast::Value;
    use std::collections::HashMap;

    pub fn register() -> HashMap<String, ModuleFn> {
        let mut map: HashMap<String, ModuleFn> = HashMap::new();
        map.insert("my_func".to_string(), my_func as ModuleFn);
        map
    }

    fn my_func(args: Vec<Value>) -> Result<Value, StdlibError> {
        // Ваша логика здесь
        Ok(Value::Null)
    }
}
```

### Способ 2: В отдельном файле (как `udp.rs`)

Для больших модулей лучше использовать отдельный файл:

#### 1. Создайте новый файл `crates/stdlib/src/my_module.rs`:

```rust
use super::{ModuleFn, StdlibError};
use ast::Value;
use std::collections::HashMap;

pub fn register() -> HashMap<String, ModuleFn> {
    let mut map: HashMap<String, ModuleFn> = HashMap::new();
    map.insert("my_func".to_string(), my_func as ModuleFn);
    map.insert("another_func".to_string(), another_func as ModuleFn);
    map
}

fn my_func(args: Vec<Value>) -> Result<Value, StdlibError> {
    // Ваша логика
    Ok(Value::Null)
}

fn another_func(args: Vec<Value>) -> Result<Value, StdlibError> {
    // Еще логика
    Ok(Value::Null)
}
```

#### 2. Добавьте модуль в `crates/stdlib/src/lib.rs`:

```rust
// В начало файла добавьте:
pub mod my_module;

// В функцию register_modules() добавьте:
modules.insert("my_module".to_string(), my_module::register());
```

## Пример: Создание модуля для работы со строками

Создайте файл `crates/stdlib/src/str_utils.rs`:

```rust
use super::{ModuleFn, StdlibError};
use ast::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub fn register() -> HashMap<String, ModuleFn> {
    let mut map: HashMap<String, ModuleFn> = HashMap::new();
    map.insert("upper".to_string(), upper as ModuleFn);
    map.insert("lower".to_string(), lower as ModuleFn);
    map.insert("reverse".to_string(), reverse as ModuleFn);
    map.insert("length".to_string(), length as ModuleFn);
    map
}

fn upper(args: Vec<Value>) -> Result<Value, StdlibError> {
    match args.first() {
        Some(Value::Str(s)) => Ok(Value::Str(Arc::from(s.to_uppercase()))),
        _ => Err(StdlibError::Message(
            "str_utils.upper(s) expects a string argument".to_string(),
        )),
    }
}

fn lower(args: Vec<Value>) -> Result<Value, StdlibError> {
    match args.first() {
        Some(Value::Str(s)) => Ok(Value::Str(Arc::from(s.to_lowercase()))),
        _ => Err(StdlibError::Message(
            "str_utils.lower(s) expects a string argument".to_string(),
        )),
    }
}

fn reverse(args: Vec<Value>) -> Result<Value, StdlibError> {
    match args.first() {
        Some(Value::Str(s)) => Ok(Value::Str(Arc::from(s.chars().rev().collect::<String>()))),
        _ => Err(StdlibError::Message(
            "str_utils.reverse(s) expects a string argument".to_string(),
        )),
    }
}

fn length(args: Vec<Value>) -> Result<Value, StdlibError> {
    match args.first() {
        Some(Value::Str(s)) => Ok(Value::Int(s.len() as i64)),
        _ => Err(StdlibError::Message(
            "str_utils.length(s) expects a string argument".to_string(),
        )),
    }
}
```

Затем в `lib.rs`:

```rust
pub mod str_utils;

// И в register_modules():
modules.insert("str_utils".to_string(), str_utils::register());
```

Теперь вы можете использовать:

```
std.str_utils.upper("hello");    // "HELLO"
std.str_utils.lower("HELLO");    // "hello"
std.str_utils.reverse("hello");  // "olleh"
std.str_utils.length("hello");   // 5
```

## Руководство по написанию функций модуля

### Тип сигнатуры

Все функции модуля должны иметь тип `ModuleFn`:

```rust
type ModuleFn = fn(Vec<Value>) -> Result<Value, StdlibError>;
```

### Проверка аргументов

Всегда проверяйте количество и тип аргументов:

```rust
fn my_function(args: Vec<Value>) -> Result<Value, StdlibError> {
    // Проверка количества
    if args.len() != 2 {
        return Err(StdlibError::Message(
            "my_function(a, b) expects 2 arguments".to_string(),
        ));
    }

    // Проверка типа
    let arg1 = match &args[0] {
        Value::Int(n) => *n,
        _ => {
            return Err(StdlibError::Message(
                "first argument must be an integer".to_string(),
            ))
        }
    };

    let arg2 = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(StdlibError::Message(
                "second argument must be a string".to_string(),
            ))
        }
    };

    // Ваша логика
    Ok(Value::Null)
}
```

### Типы значений

VexCore поддерживает следующие типы значений:

- `Value::Int(i64)` - целое число
- `Value::Float(f64)` - число с плавающей точкой
- `Value::Str(Arc<str>)` - строка
- `Value::Bool(bool)` - булево значение
- `Value::Null` - null
- `Value::List(Arc<[Value]>)` - список
- `Value::Json(Arc<BTreeMap<String, Value>>)` - JSON объект

### Работа со строками

Используйте `Arc::from()` при создании строк:

```rust
Ok(Value::Str(Arc::from("my string")))
```

## Использование модулей в VexCore коде

### Стандартные модули (с префиксом `std`)

```
std.net.ping("127.0.0.1");
std.net.port_open("localhost", 8080);
std.str_utils.upper("hello");
```

### Пользовательские модули (импортированные через `use`)

```
use "my_module.vcl" as custom;
custom.my_func();

// Или без alias (используется имя файла):
use "my_module.vcl";
my_module.my_func();
```

## Структура проекта после расширения

```
crates/stdlib/src/
├── lib.rs              # Главный файл, регистрация всех модулей
├── net.rs             # Модуль для сетевых функций
├── udp.rs             # Модуль для UDP
├── str_utils.rs       # Ваш новый модуль
└── file_io.rs         # Еще один модуль
```

## Сборка и тестирование

1. Добавьте свой модуль как описано выше
2. Скомпилируйте проект:
   ```bash
   cargo build
   ```
3. Протестируйте в REPL или скриптах:
   ```bash
   cargo run
   ```

## Советы и лучшие практики

1. **Четкие имена ошибок**: Укажите имя функции в сообщении об ошибке
2. **Проверка входных данных**: Всегда проверяйте типы и количество аргументов
3. **Документирование**: Документируйте свои функции комментариями
4. **Группировка**: Разберите функции по логическим модулям
5. **Тестирование**: Создавайте тесты для ваших функций в `crates/stdlib/tests/`

## Пример полного модуля Math

```rust
pub mod math {
    use super::{ModuleFn, StdlibError};
    use ast::Value;
    use std::collections::HashMap;

    pub fn register() -> HashMap<String, ModuleFn> {
        let mut map: HashMap<String, ModuleFn> = HashMap::new();
        map.insert("abs".to_string(), abs as ModuleFn);
        map.insert("sqrt".to_string(), sqrt as ModuleFn);
        map.insert("pow".to_string(), pow as ModuleFn);
        map.insert("max".to_string(), max as ModuleFn);
        map.insert("min".to_string(), min as ModuleFn);
        map
    }

    fn abs(args: Vec<Value>) -> Result<Value, StdlibError> {
        match args.first() {
            Some(Value::Int(n)) => Ok(Value::Int(n.abs())),
            Some(Value::Float(f)) => Ok(Value::Float(f.abs())),
            _ => Err(StdlibError::Message("math.abs expects a number".to_string())),
        }
    }

    fn sqrt(args: Vec<Value>) -> Result<Value, StdlibError> {
        match args.first() {
            Some(Value::Float(f)) => Ok(Value::Float(f.sqrt())),
            Some(Value::Int(n)) => Ok(Value::Float((*n as f64).sqrt())),
            _ => Err(StdlibError::Message("math.sqrt expects a number".to_string())),
        }
    }

    fn pow(args: Vec<Value>) -> Result<Value, StdlibError> {
        if args.len() != 2 {
            return Err(StdlibError::Message("math.pow expects 2 arguments".to_string()));
        }

        let (base, exp) = match (&args[0], &args[1]) {
            (Value::Float(b), Value::Float(e)) => (*b, *e),
            (Value::Int(b), Value::Int(e)) => (*b as f64, *e as f64),
            _ => return Err(StdlibError::Message("math.pow expects numbers".to_string())),
        };

        Ok(Value::Float(base.powf(exp)))
    }

    fn max(args: Vec<Value>) -> Result<Value, StdlibError> {
        if args.is_empty() {
            return Err(StdlibError::Message("math.max expects at least 1 argument".to_string()));
        }

        let mut result = match &args[0] {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            _ => return Err(StdlibError::Message("math.max expects numbers".to_string())),
        };

        for arg in &args[1..] {
            let val = match arg {
                Value::Int(n) => *n as f64,
                Value::Float(f) => *f,
                _ => return Err(StdlibError::Message("math.max expects numbers".to_string())),
            };
            if val > result {
                result = val;
            }
        }

        Ok(Value::Float(result))
    }

    fn min(args: Vec<Value>) -> Result<Value, StdlibError> {
        if args.is_empty() {
            return Err(StdlibError::Message("math.min expects at least 1 argument".to_string()));
        }

        let mut result = match &args[0] {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            _ => return Err(StdlibError::Message("math.min expects numbers".to_string())),
        };

        for arg in &args[1..] {
            let val = match arg {
                Value::Int(n) => *n as f64,
                Value::Float(f) => *f,
                _ => return Err(StdlibError::Message("math.min expects numbers".to_string())),
            };
            if val < result {
                result = val;
            }
        }

        Ok(Value::Float(result))
    }
}
```

Используется как:

```
std.math.abs(-42);
std.math.sqrt(16.0);
std.math.pow(2.0, 3.0);
std.math.max(1, 5, 3);
std.math.min(1, 5, 3);
```
