use crate::error::CompilerError;
use crate::parser::parse_program;
use crate::semantic::TypeChecker;
use crate::codegen::LlvmCodeGen;
use inkwell::context::Context;
use std::process::Command;

pub fn compile(source_code: &str, output_ir: &str, execute: bool) -> Result<(), CompilerError> {
    // 1. Parse
    let ast = parse_program(source_code)?;

    // 2. Type checking
    let mut type_checker = TypeChecker::new();
    type_checker
        .check(&ast)
        .map_err(|errors| {
            let msg = errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
            CompilerError::ParserError { msg }
        })?;

    // 3. Codegen
    let context = Context::create();
    let mut codegen = LlvmCodeGen::new(&context, "hulk_module");
    codegen.compile(&ast).map_err(|msg| CompilerError::ParserError { msg })?;
    codegen.write_to_file(output_ir).map_err(|e| CompilerError::ParserError { msg: e })?;

    // 4. Optional execution
    if execute {
        let exec_path = output_ir.replace(".ll", "");
        compile_and_run(output_ir, &exec_path)?;
    }

    Ok(())
}

fn compile_and_run(ir_file: &str, exec_path: &str) -> Result<(), CompilerError> {
    // Compile IR to executable with clang
    let clang_output = Command::new("clang-15")
        .args(&[ir_file, "-o", exec_path, "-lm"])
        .output()
        .map_err(|e| CompilerError::ParserError {
            msg: format!("Failed to run clang: {}", e),
        })?;
    if !clang_output.status.success() {
        let stderr = String::from_utf8_lossy(&clang_output.stderr);
        return Err(CompilerError::ParserError {
            msg: format!("clang compilation failed:\n{}", stderr),
        });
    }       

    // Ensure executable exists
    if !std::path::Path::new(exec_path).exists() {
        return Err(CompilerError::ParserError {
            msg: format!("Executable '{}' not found after compilation", exec_path),
        });
    }

    // Run the executable
    let output = Command::new(format!("./{}", exec_path))
        .output()
        .map_err(|e| CompilerError::ParserError {
            msg: format!("Failed to run executable '{}': {}", exec_path, e),
        })?;

    // Print output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}