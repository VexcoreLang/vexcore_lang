use super::{ModuleFn, StdlibError};
use ast::Value;
use std::collections::HashMap;

pub fn register() -> HashMap<String, ModuleFn> {
    let mut map: HashMap<String, ModuleFn> = HashMap::new();
    map.insert("PI".to_string(), PI as ModuleFn);
    map.insert("E".to_string(), E as ModuleFn);
    map.insert("TAU".to_string(), TAU as ModuleFn);
    map.insert("abs".to_string(), ABS as ModuleFn);
    map.insert("min".to_string(), MIN as ModuleFn);
    map.insert("max".to_string(), MAX as ModuleFn);
    map.insert("clamp".to_string(), CLAMP as ModuleFn);
    map.insert("pow".to_string(), POW as ModuleFn);
    map.insert("sqrt".to_string(), SQRT as ModuleFn);
    map.insert("cbrt".to_string(), CBRT as ModuleFn);
    map.insert("floor".to_string(), FLOOR as ModuleFn);
    map.insert("ceil".to_string(), CEIL as ModuleFn);
    map.insert("round".to_string(), ROUND as ModuleFn);
    map.insert("trunc".to_string(), TRUNC as ModuleFn);
    map.insert("sin".to_string(), SIN as ModuleFn);
    map.insert("cos".to_string(), COS as ModuleFn);
    map.insert("tan".to_string(), TAN as ModuleFn);

    map.insert("atan2".to_string(), ATAN2 as ModuleFn);

    map.insert("exp".to_string(), EXP as ModuleFn);
    map.insert("ln".to_string(), LN as ModuleFn);
    map.insert("log".to_string(), LOG as ModuleFn);
    map
}

fn as_f64(v: &Value) -> Result<f64, StdlibError> {
    match v {
        Value::Float(x) => Ok(*x),
        Value::Int(x) => Ok(*x as f64),
        _ => Err(StdlibError::Message("expected number".to_string())),
    }
}

fn PI(args: Vec<Value>) -> Result<Value, StdlibError> {
    Ok(Value::Float(3.14159))
}

fn E(args: Vec<Value>) -> Result<Value, StdlibError> {
    Ok(Value::Float(2.71828))
}

fn TAU(args: Vec<Value>) -> Result<Value, StdlibError> {
    Ok(Value::Float(2.0 * 3.14159))
}

fn ABS(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.abs(x) expects 1 argument".to_string(),
        ));
    }

    match &args[0] {
        Value::Float(x) => Ok(Value::Float(x.abs())),
        Value::Int(x) => Ok(Value::Int(x.abs())),
        _ => Err(StdlibError::Message(
            "math.abs(x) expects number (int or float)".to_string(),
        )),
    }
}

fn MIN(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 2 {
        return Err(StdlibError::Message(
            "math.min(a, b) expects 2 arguments".to_string(),
        ));
    }

    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b as f64))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
        _ => Err(StdlibError::Message("math.min expects numbers".to_string())),
    }
}

fn MAX(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 2 {
        return Err(StdlibError::Message(
            "math.max(a, b) expects 2 arguments".to_string(),
        ));
    }

    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b as f64))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
        _ => Err(StdlibError::Message("math.max expects numbers".to_string())),
    }
}

fn CLAMP(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 3 {
        return Err(StdlibError::Message(
            "math.clamp(x, min, max) expects 3 arguments".to_string(),
        ));
    }
    
    let (x, min, max) = (&args[0], &args[1], &args[2]);

    match (x, min, max) {
        (Value::Int(x), Value::Int(min), Value::Int(max)) => {
            Ok(Value::Int(*x.max(min).min(max)))
        }
        _ => {
            let x = as_f64(x)?;
            let min = as_f64(min)?;
            let max = as_f64(max)?;
            Ok(Value::Float(x.max(min).min(max)))
        }
    }
}

fn POW(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 2 {
        return Err(StdlibError::Message(
            "math.pow(x, y) expects 2 arguments".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;
    let y = as_f64(&args[1])?;

    Ok(Value::Float(x.powf(y)))
}

fn SQRT(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.sqrt(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;

    Ok(Value::Float(x.sqrt()))
}

fn CBRT(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.cbrt(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;

    Ok(Value::Float(x.cbrt()))
}

fn FLOOR(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.floor(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;

    Ok(Value::Float(x.floor()))
}

fn CEIL(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.ceil(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;

    Ok(Value::Float(x.ceil()))
}

fn ROUND(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.round(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;

    Ok(Value::Float(x.round()))
}

fn TRUNC(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.trunc(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;

    Ok(Value::Float(x.trunc()))
}

fn SIN(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.sin(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;
    Ok(Value::Float(x.sin()))
}

fn COS(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.cos(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;
    Ok(Value::Float(x.cos()))
}

fn TAN(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.tan(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;
    Ok(Value::Float(x.tan()))
}

fn ATAN2(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 2 {
        return Err(StdlibError::Message(
            "math.atan2(y, x) expects 2 arguments".to_string(),
        ));
    }

    let y = as_f64(&args[0])?;
    let x = as_f64(&args[1])?;

    Ok(Value::Float(y.atan2(x)))
}

fn EXP(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.exp(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;
    Ok(Value::Float(x.exp()))
}

fn LN(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 1 {
        return Err(StdlibError::Message(
            "math.ln(x) expects 1 argument".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;

    if x <= 0.0 {
        return Err(StdlibError::Message(
            "math.ln(x) domain error: x must be > 0".to_string(),
        ));
    }

    Ok(Value::Float(x.ln()))
}

fn LOG(args: Vec<Value>) -> Result<Value, StdlibError> {
    if args.len() != 2 {
        return Err(StdlibError::Message(
            "math.log(x, base) expects 2 arguments".to_string(),
        ));
    }

    let x = as_f64(&args[0])?;
    let base = as_f64(&args[1])?;

    if x <= 0.0 || base <= 0.0 {
        return Err(StdlibError::Message(
            "math.log domain error: x and base must be > 0".to_string(),
        ));
    }

    Ok(Value::Float(x.log(base)))
}