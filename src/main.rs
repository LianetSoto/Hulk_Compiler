mod lexer;
mod parser;
mod ast;
mod error;
mod semantic;
mod codegen;
mod compiler;
mod compiler_dev;   
mod gen_lex;
mod transform;

use compiler::compile;
use error::{report_std_error, CompilerError, SourceMap};
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file.hulk>", args[0]);
        process::exit(1);
    }
    let filename = &args[1];
    let source = match fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("(0,0) LEXICAL: cannot read file: {}", e);
            process::exit(1);
        }
    };

    // DEVELOPMENT MODE (cargo run --features dev)
    #[cfg(feature = "dev")]
    {
        if let Err(e) = compiler_dev::compile(
            &source,
            "output.ll",
            true,           // execute
            filename,
            false,          // print_parsed
            true,           // print_typed
            true,           // print_mono
        ) {
            let source_map = SourceMap::new(source);
            error::report_error(&e, &source_map, filename);
            process::exit(1);
        }
        process::exit(0);
    }

    // CI / PRODUCTION MODE (make build, cargo run)
    #[cfg(not(feature = "dev"))]
    {
        match compile(&source) {
            Ok(()) => process::exit(0),
            Err(errors) => {
                let source_map = SourceMap::new(source);
                let mut exit_code = 0;
                for err in &errors {
                    report_std_error(err, &source_map);
                    let code = match err {
                        CompilerError::LexerError { .. } => 1,
                        CompilerError::ParserError { .. }       => 2,
                        _ => 3,   // semantic and other errors
                    };
                    if code < exit_code || exit_code == 0 {
                        exit_code = code;
                    }
                }
                process::exit(exit_code);
            }
        }
    }
}