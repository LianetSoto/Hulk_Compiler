mod lexer;
mod ast;
mod error;
mod parser;

use parser::parse_program;
use ast::PrettyPrinter;
use ast::node::Node;   // ← necesario para usar .accept()

fn main() {
    let codigo = r#"
        print(42 + 3 * 5 / 2 - 89a);
    "#;

    match parse_program(codigo) {
        Ok(program) => {
            let mut printer = PrettyPrinter::new();
            program.accept(&mut printer);
            println!("AST:\n{}", printer.into_string());
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}