use interpreter::Interpreter;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

pub fn run_repl(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut rl = DefaultEditor::new()?;
    let mut interp = Interpreter::new(".");

    println!("Vexcore REPL. Type :quit to exit.");
    loop {
        let line = rl.readline("vcl> ");
        match line {
            Ok(input) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == ":quit" || trimmed == ":q" {
                    break;
                }

                let wrapped = ensure_semi(trimmed);
                match lexer::tokenize(&wrapped)
                    .map_err(|e| e.to_string())
                    .and_then(|t| parser::parse(t).map_err(|e| e.to_string()))
                {
                    Ok(program) => {
                        if debug {
                            println!("[debug] parsed {} statement(s)", program.statements.len());
                        }
                        if let Err(e) = interp.execute_program(&program) {
                            eprintln!("Runtime error: {e}");
                        }
                    }
                    Err(e) => eprintln!("Parse error: {e}"),
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Readline error: {err}");
                break;
            }
        }
    }

    Ok(())
}

fn ensure_semi(input: &str) -> String {
    if input.ends_with(';') || input.ends_with('}') {
        input.to_string()
    } else {
        format!("{input};")
    }
}
