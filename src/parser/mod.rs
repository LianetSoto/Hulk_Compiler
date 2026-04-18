use lalrpop_util::lalrpop_mod;
use crate::lexer::Lexer;
use crate::ast::Program;
use crate::error::CompilerError;

lalrpop_mod!(grammar);

pub fn parse_program(input: &str) -> Result<Program, CompilerError> {
    let lexer = Lexer::new(input);
    let parser = grammar::ProgramParser::new();
    match parser.parse(lexer) {
        Ok(program) => Ok(*program),
        Err(err) => Err(CompilerError::ParserError { msg: format!("{:?}", err) }),
    }
}