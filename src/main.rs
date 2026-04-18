mod lexer;
mod parser;
mod ast;
mod semantic;
mod codegen;
mod error;
mod compiler;

use compiler::compile;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Uso: {} <archivo.hulk>", args[0]);
        process::exit(1);
    }
    let filename = &args[1];
    let source_code = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error leyendo '{}': {}", filename, err);
        process::exit(1);
    });

    match compile(&source_code, "output.ll", true) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error de compilación: {}", e);
            process::exit(1);
        }
    }
}