use super::{ModuleFn, StdlibError};
use ast::Value;
use std::collections::HashMap;

pub fn register() -> HashMap<String, ModuleFn> {
    let mut map: HashMap<String, ModuleFn> = HashMap::new();
    map.insert("test_math_func".to_string(), test_math_func as ModuleFn);
    map
}

fn test_math_func(args: Vec<Value>) -> Result<Value, StdlibError> {
    println!("Hello Math");
    Ok(Value::Null)
}
