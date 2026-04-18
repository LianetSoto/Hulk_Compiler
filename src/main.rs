mod lexer;
mod ast;
mod error;
mod parser;
mod semantic;  // <-- agregar

use parser::parse_program;
use ast::PrettyPrinter;
use ast::node::Node;
use semantic::type_checker::TypeChecker;

fn main() {
    let codigo = r#"
        print(42 + a);
    "#;

    match parse_program(codigo) {
        Ok(program) => {
            // 1. Pretty print del AST
            let mut printer = PrettyPrinter::new();
            program.accept(&mut printer);
            println!("AST:\n{}", printer.into_string());

            // 2. Análisis semántico
            let mut checker = TypeChecker::new();
            match checker.check(&program) {
                Ok(()) => println!("✅ Semantic analysis passed!"),
                Err(errors) => {
                    eprintln!("❌ Semantic errors:");
                    for err in errors {
                        eprintln!("  {}", err);
                    }
                }
            }
        }
        Err(e) => eprintln!("Parsing error: {}", e),
    }
}