use ast::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub mod math;
pub mod time;

pub type BuiltinFn = fn(Vec<Value>) -> Result<Value, StdlibError>;
pub type ModuleFn = fn(Vec<Value>) -> Result<Value, StdlibError>;

#[derive(Debug, thiserror::Error)]
pub enum StdlibError {
    #[error("{0}")]
    Message(String),
}

pub struct Stdlib {
    pub builtins: HashMap<String, BuiltinFn>,
    pub modules: HashMap<String, HashMap<String, ModuleFn>>,
}

impl Stdlib {
    pub fn new() -> Self {
        Self {
            builtins: register_builtins(),
            modules: register_modules(),
        }
    }

    pub fn call_builtin(&self, name: &str, args: Vec<Value>) -> Result<Value, StdlibError> {
        let func = self
            .builtins
            .get(name)
            .ok_or_else(|| StdlibError::Message(format!("Unknown builtin: {name}")))?;
        func(args)
    }

    pub fn call_module(
        &self,
        module: &str,
        func: &str,
        args: Vec<Value>,
    ) -> Result<Value, StdlibError> {
        let module_map = self
            .modules
            .get(module)
            .ok_or_else(|| StdlibError::Message(format!("Unknown module: {module}")))?;
        let f = module_map.get(func).ok_or_else(|| {
            StdlibError::Message(format!("Unknown module function: {module}.{func}"))
        })?;
        f(args)
    }
}

fn builtin_out(args: Vec<Value>) -> Result<Value, StdlibError> {
    if let Some(v) = args.first() {
        print!("{}", v.to_pretty_string());
    } else {
        print!("");
    }
    Ok(Value::Null)
}

fn builtin_outln(args: Vec<Value>) -> Result<Value, StdlibError> {
    if let Some(v) = args.first() {
        println!("{}", v.to_pretty_string());
    } else {
        println!();
    }
    Ok(Value::Null)
}

fn builtin_run(args: Vec<Value>) -> Result<Value, StdlibError> {
    let cmd = match args.first() {
        Some(Value::Str(s)) => s,
        _ => {
            return Err(StdlibError::Message(
                "run(cmd) expects a single string argument".to_string(),
            ))
        }
    };

    let output = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .arg("/C")
            .arg(cmd.as_ref())
            .output()
    } else {
        std::process::Command::new("sh").arg("-c").arg(cmd.as_ref()).output()
    }
    .map_err(|e| StdlibError::Message(format!("run failed: {e}")))?;

    Ok(Value::Str(Arc::from(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    )))
}


fn register_builtins() -> HashMap<String, BuiltinFn> {
    let mut map: HashMap<String, BuiltinFn> = HashMap::new();
    // EXTENSION POINT: add new built-in functions in this single registry.
    map.insert("out".to_string(), builtin_out as BuiltinFn);
    map.insert("outln".to_string(), builtin_outln as BuiltinFn);
    map.insert("log".to_string(), builtin_outln as BuiltinFn);
    map.insert("run".to_string(), builtin_run as BuiltinFn);
    map
}

fn register_modules() -> HashMap<String, HashMap<String, ModuleFn>> {
    let mut modules: HashMap<String, HashMap<String, ModuleFn>> = HashMap::new();
    // EXTENSION POINT: register new stdlib modules in this single registry.
    modules.insert("net".to_string(), net::register());
    modules.insert("udp".to_string(), udp::register());
    modules.insert("math".to_string(), math::register());
    modules.insert("time".to_string(), time::register());
    modules
}

mod net {
    use super::{ModuleFn, StdlibError};
    use ast::Value;
    use std::collections::HashMap;
    use std::net::{TcpStream, ToSocketAddrs};
    use std::sync::Arc;
    use std::time::Duration;

    pub fn register() -> HashMap<String, ModuleFn> {
        let mut map: HashMap<String, ModuleFn> = HashMap::new();
        map.insert("ping".to_string(), ping as ModuleFn);
        map.insert("port_open".to_string(), port_open as ModuleFn);
        map.insert("resolve".to_string(), resolve as ModuleFn);
        map
    }

    fn ping(args: Vec<Value>) -> Result<Value, StdlibError> {
        let host = match args.first() {
            Some(Value::Str(s)) => s,
            _ => {
                return Err(StdlibError::Message(
                    "net.ping(host) expects string host".to_string(),
                ))
            }
        };

        let output = if cfg!(target_os = "windows") {
            std::process::Command::new("ping")
                .arg("-n")
                .arg("1")
                .arg(host.as_ref())
                .output()
        } else {
            std::process::Command::new("ping")
                .arg("-c")
                .arg("1")
                .arg(host.as_ref())
                .output()
        }
        .map_err(|e| StdlibError::Message(format!("ping failed: {e}")))?;

        Ok(Value::Bool(output.status.success()))
    }

    fn port_open(args: Vec<Value>) -> Result<Value, StdlibError> {
        if args.len() != 2 {
            return Err(StdlibError::Message(
                "net.port_open(host, port) expects 2 arguments".to_string(),
            ));
        }
        let host = match &args[0] {
            Value::Str(s) => s.clone(),
            _ => {
                return Err(StdlibError::Message(
                    "net.port_open host must be string".to_string(),
                ))
            }
        };
        let port = match &args[1] {
            Value::Int(p) => *p as u16,
            _ => {
                return Err(StdlibError::Message(
                    "net.port_open port must be int".to_string(),
                ))
            }
        };

        let addr = format!("{host}:{port}");
        let timeout = Duration::from_secs(2);
        let result = addr
            .to_socket_addrs()
            .map_err(|e| StdlibError::Message(format!("resolve failed: {e}")))?
            .any(|socket_addr| TcpStream::connect_timeout(&socket_addr, timeout).is_ok());

        Ok(Value::Bool(result))
    }

    fn resolve(args: Vec<Value>) -> Result<Value, StdlibError> {
        let host = match args.first() {
            Some(Value::Str(s)) => s,
            _ => {
                return Err(StdlibError::Message(
                    "net.resolve(hostname) expects string".to_string(),
                ))
            }
        };

        let addr = format!("{host}:0");
        let first = addr
            .to_socket_addrs()
            .ok()
            .and_then(|mut iter| iter.next())
            .map(|a| a.ip().to_string());

        Ok(match first {
            Some(ip) => Value::Str(Arc::from(ip)),
            None => Value::Null,
        })
    }
}




pub mod udp {
    use super::{ModuleFn, StdlibError};
    use ast::Value;
    use std::collections::HashMap;

    pub fn register() -> HashMap<String, ModuleFn> {
        let mut map: HashMap<String, ModuleFn> = HashMap::new();
        map.insert("test".to_string(), test as ModuleFn);
        map
    }

    fn test(_args: Vec<Value>) -> Result<Value, StdlibError> {
        println!("Claude пидорас ебаный");
        Ok(Value::Bool(true))
    }
}