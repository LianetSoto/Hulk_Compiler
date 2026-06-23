use crate::error::CompilerError;
use crate::parser::parse_program;
use crate::semantic::TypeChecker;
use crate::codegen::LlvmCodeGen;
use crate::transform::MonomorphizationPass;
use crate::gen_lex::lexer::build_lexer;
use inkwell::context::Context;
use std::process::Command;

pub fn compile(source_code: &str) -> Result<(), Vec<CompilerError>> {

    // Lexer
    let lexer = build_lexer();
    let tokens = match lexer.tokenize(source_code) {
        Ok(t) => t,
        Err(e) => return Err(vec![e]),
    };
    
    // Parser
    let mut ast = match parse_program(tokens) {
        Ok(prog) => prog,
        Err(e) => return Err(vec![e]),   
    };

    // Type checking
    let mut type_checker = TypeChecker::new();
    if let Err(errors) = type_checker.check(&mut ast, true) {
        return Err(errors);             
    }

    // Monomorphization
    let mut mono_pass = MonomorphizationPass::new();
    if let Err(err) = mono_pass.run(&mut ast) {
        return Err(vec![err]);           
    }

    // Code Generation LLVM
    let context = Context::create();
    let mut codegen = LlvmCodeGen::new(&context, "hulk_module");
    codegen.set_flattened_types(type_checker.get_flattened_types().clone());
    if let Err(err) = codegen.compile(&mut ast) {
        return Err(vec![err]);           
    }

    if let Err(err) = codegen.write_to_file("output.ll") {
        return Err(vec![err]);          
    }

    let clang_status = Command::new("clang-15")
        .args(&["output.ll", "-o", "output", "-lm"])
        .status()
        .map_err(|e| vec![CompilerError::IoError(format!("clang failed: {}", e))])?;

    if !clang_status.success() {
        return Err(vec![CompilerError::CodegenError {
            msg: "clang compilation failed".to_string(),
            span: None,
        }]);
    }

    if !std::path::Path::new("output").exists() {
        return Err(vec![CompilerError::IoError("output executable not created".to_string())]);
    }

    Ok(())
}