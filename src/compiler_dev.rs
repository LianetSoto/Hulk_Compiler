use crate::error::{CompilerError, SourceMap, report_error};
use crate::parser::parse_program;
use crate::semantic::TypeChecker;
use crate::codegen::LlvmCodeGen;
use crate::transform::MonomorphizationPass;
use crate::gen_lex::lexer::build_lexer;
use crate::ast::{PrettyPrinter, Node};
use inkwell::context::Context;
use std::process::Command;
use std::process;

/// Compiles the given HULK source code.
///
/// # Arguments
/// * `source_code` - The HULK source code as a string.
/// * `output_ir` - Path where the LLVM IR file will be written.
/// * `execute` - Whether to compile the IR to an executable and run it.
/// * `filename` - The name of the source file (used for error reporting).
/// * `print_ast` - If true, prints the Abstract Syntax Tree after parsing.
pub fn compile(source_code: &str, output_ir: &str, execute: bool, filename: &str, 
               print_parsed:bool, print_typed:bool, print_mono: bool) 
        -> Result<(), CompilerError> 
{
    let source_map = SourceMap::new(source_code.to_string());


    let lexer = build_lexer();
        let tokens = match lexer.tokenize(source_code) {
        Ok(t) => t,
        Err(e) => {
            report_error(&e, &source_map, filename);
            process::exit(1);
        }
    };
    
    // 1. Syntactic analysis (parsing)
    let mut ast = match parse_program(tokens) {
        Ok(prog) => prog,
        Err(e) => {
            report_error(&e, &source_map, filename);
            process::exit(1);
        }
    };

    if print_parsed {
        let mut printer = PrettyPrinter::new();
        ast.accept(&mut printer);
        println!("=== Abstract Syntax Tree ===\n{}", printer.into_string());
    }

    // 2. Semantic analysis (type checking)
    let mut type_checker = TypeChecker::new();
    if let Err(errors) = type_checker.check(&mut ast, print_typed) {
        for err in errors {
            report_error(&err, &source_map, filename);
        }
        process::exit(1);
    }

    // 3. Monomorphization (AST → AST without generics)
    let mut mono_pass = MonomorphizationPass::new();
    if let Err(err) = mono_pass.run(&mut ast) {
        report_error(&err, &source_map, filename);
        process::exit(1);
    }

    if print_mono {
        let mut printer = PrettyPrinter::new();
        ast.accept(&mut printer);
        println!("=== AST after monomorphization ===\n{}", printer.into_string());
    }
    
    // 4. Code generation (LLVM IR)
    let context = Context::create();
    let mut codegen = LlvmCodeGen::new(&context, "hulk_module");
    codegen.set_flattened_types(type_checker.get_flattened_types().clone());
    if let Err(err) = codegen.compile(&mut ast) {
        report_error(&err, &source_map, filename);
        process::exit(1);
    }

    if let Err(err) = codegen.write_to_file(output_ir) {
        report_error(&err, &source_map, filename);
        process::exit(1);
    }

    // 5. Optional execution
    if execute {
        let exec_path = output_ir.replace(".ll", "");
        compile_and_run(output_ir, &exec_path)?;
    }
    
    Ok(())
}

/// Compiles the LLVM IR file to an executable and runs it.
fn compile_and_run(ir_file: &str, exec_path: &str) -> Result<(), CompilerError> {
    // Compile IR to executable using clang
    let clang_output = Command::new("clang-17")
        .args(&[ir_file, "-o", exec_path, "-lm"])
        .output()
        .map_err(|e| CompilerError::IoError(format!("Failed to run clang: {}", e)))?;

    if !clang_output.status.success() {
        let stderr = String::from_utf8_lossy(&clang_output.stderr);
        return Err(CompilerError::CodegenError {
            msg: format!("clang compilation failed:\n{}", stderr),
            span: None,
        });
    }

    // Ensure the executable was created
    if !std::path::Path::new(exec_path).exists() {
        return Err(CompilerError::IoError(format!("Executable '{}' not found after compilation", exec_path)));
    }

    // Run the executable
    let output = Command::new(format!("./{}", exec_path))
        .output()
        .map_err(|e| CompilerError::IoError(format!("Failed to run executable '{}': {}", exec_path, e)))?;

    // Print the program's output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}