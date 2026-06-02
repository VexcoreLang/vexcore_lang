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

    let tokens = match lexer::tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lex error: {e}");
            std::process::exit(1);
        }
    };

    if cli.debug {
        eprintln!("[debug] token count: {}", tokens.len());
    }

    let program = match parser::parse(tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {e}");
            std::process::exit(1);
        }
    };

    if cli.debug {
        eprintln!("[debug] statements: {}", program.statements.len());
    }

    let base_dir = script_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut interp = interpreter::Interpreter::new(base_dir);
    if let Err(e) = interp.execute_program(&program) {
        eprintln!("Runtime error: {e}");
        std::process::exit(1);
    }
}
