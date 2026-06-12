use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "vexcore",
    version,
    about = "Vexcore scripting language runtime"
)]
struct Cli {
    script: Option<PathBuf>,
    #[arg(long)]
    repl: bool,
    #[arg(long)]
    debug: bool,
    #[arg(long)]
    no_cache: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.repl {
        if let Err(e) = repl::run_repl(cli.debug) {
            eprintln!("REPL error: {e}");
            std::process::exit(1);
        }
        return;
    }

    let Some(script_path) = cli.script else {
        eprintln!("Usage: vexcore <script.vcl> | vexcore --repl [--debug]");
        std::process::exit(2);
    };

    let source = match fs::read_to_string(&script_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read {}: {e}", script_path.display());
            std::process::exit(1);
        }
    };

    let bytecode_path = script_path.with_extension("vcbc");
    let code = if !cli.no_cache && should_use_cache(&script_path, &bytecode_path) {
        if cli.debug {
            eprintln!("[debug] loading from cache: {}", bytecode_path.display());
        }
        match interpreter::load_bytecode(&bytecode_path) {
            Ok(c) => c,
            Err(e) => {
                if cli.debug {
                    eprintln!("[debug] cache load failed: {e}, recompiling...");
                }
                compile_script(&source, &script_path, &bytecode_path, cli.debug)
            }
        }
    } else {
        compile_script(&source, &script_path, &bytecode_path, cli.debug)
    };

    let base_dir = script_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut interp = interpreter::Interpreter::new(base_dir);
    if let Err(e) = interp.execute_program_from_bytecode(&code) {
        eprintln!("Runtime error: {e}");
        std::process::exit(1);
    }
}

fn should_use_cache(vcl_path: &PathBuf, vcbc_path: &PathBuf) -> bool {
    if !vcbc_path.exists() {
        return false;
    }
    match (fs::metadata(vcl_path), fs::metadata(vcbc_path)) {
        (Ok(vcl_meta), Ok(vcbc_meta)) => match (vcl_meta.modified(), vcbc_meta.modified()) {
            (Ok(vcl_time), Ok(vcbc_time)) => vcl_time <= vcbc_time,
            _ => false,
        },
        _ => false,
    }
}

fn compile_script(
    source: &str,
    _vcl_path: &PathBuf,
    vcbc_path: &PathBuf,
    debug: bool,
) -> Vec<interpreter::Instr> {
    let tokens = match lexer::tokenize(source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lex error: {e}");
            std::process::exit(1);
        }
    };

    if debug {
        eprintln!("[debug] token count: {}", tokens.len());
    }

    let program = match parser::parse(tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {e}");
            std::process::exit(1);
        }
    };

    if debug {
        eprintln!("[debug] statements: {}", program.statements.len());
    }

    let mut compiler = interpreter::Compiler::new();
    let code = match compiler.compile_program(&program) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Compile error: {e}");
            std::process::exit(1);
        }
    };

    // Сохраняем кэш
    let _ = interpreter::save_bytecode(&code, vcbc_path);

    code
}
