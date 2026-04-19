
mod lexer;
mod parser;
mod ast;
mod error;
mod semantic;
mod codegen;
mod compiler;

use compiler::compile;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file.hulk>", args[0]);
        process::exit(1);
    }
    let filename = &args[1];
    let source = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading '{}': {}", filename, err);
        process::exit(1);
    });

    if let Err(e) = compile(&source, "output.ll", true, filename) {
        process::exit(1);
    }
}
