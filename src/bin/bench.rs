use std::hint::black_box;
use std::time::Instant;

fn main() {
    let scripts = [
        ("bench/compute.vcl", 25usize),
        ("bench/imports.vcl", 25usize),
    ];

    for (script, runs) in scripts {
        if let Err(e) = run_benchmark(script, runs) {
            eprintln!("{script}: {e}");
            std::process::exit(1);
        }
    }
}

fn run_benchmark(script: &str, runs: usize) -> Result<(), Box<dyn std::error::Error>> {
    let script_path = std::path::PathBuf::from(script);
    let source = std::fs::read_to_string(&script_path)?;
    let tokens = lexer::tokenize(&source)?;
    let program = parser::parse(tokens)?;
    let mut translator = vexcore::bytecode_translator::BytecodeTranslator::new();

    let warmup = translator.translate_program(black_box(&program));
    if let Err(e) = warmup {
        return Err(Box::new(std::io::Error::other(format!(
            "warmup failed: {e}"
        ))));
    }

    let start = Instant::now();
    for _ in 0..runs {
        let result = translator.translate_program(black_box(&program));
        if let Err(e) = result {
            return Err(Box::new(std::io::Error::other(format!(
                "run failed: {e}"
            ))));
        }
    }
    let elapsed = start.elapsed();
    let avg = elapsed / runs as u32;

    println!(
        "{script}: {runs} runs, total {:?}, avg {:?}",
        elapsed, avg
    );

    Ok(())
}
