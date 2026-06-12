use super::{ModuleFn, StdlibError};
use ast::Value;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

pub fn register() -> HashMap<String, ModuleFn> {
    let mut map: HashMap<String, ModuleFn> = HashMap::new();
    map.insert("wait".to_string(), WAIT as ModuleFn);
    map
}



fn WAIT(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "time.wait(seconds) expects 1 argument".to_string(),
        ));
    }

    let seconds = match &args[0] {
        Value::Float(x) => *x,
        Value::Int(x) => *x as f64,
        _ => {
            return Err(StdlibError::Message(
                "time.wait(seconds) expects number".to_string(),
            ))
        }
    };

    if seconds < 0.0 {
        return Err(StdlibError::Message(
            "time.wait(seconds) expects positive value".to_string(),
        ));
    }

    let duration = Duration::from_secs_f64(seconds);
    thread::sleep(duration);

    Ok(Value::Float(seconds))
}