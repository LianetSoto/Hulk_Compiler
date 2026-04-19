use lalrpop_util::lalrpop_mod;
use crate::lexer::Lexer;
use crate::ast::Program;
use crate::error::CompilerError;
use lalrpop_util::ParseError;
use crate::error::Span;

lalrpop_mod!(grammar);

pub fn parse_program(input: &str) -> Result<Program, CompilerError> {
    let lexer = Lexer::new(input);
    let parser = grammar::ProgramParser::new();
    parser.parse(lexer)
        .map(|boxed_program| *boxed_program)   // ← Desempaqueta el Box<Program> a Program
        .map_err(|err| {
            let msg = format!("{:?}", err);
            let span = match err {
                ParseError::UnrecognizedToken { token, expected: _ } => {
                    Some(Span::new(token.0, token.2))
                }
                ParseError::UnrecognizedEof { location, expected: _ } => {
                    Some(Span::new(location, location))
                }
                ParseError::InvalidToken { location: _ } => None,
                _ => None,
            };
            CompilerError::ParserError { msg, span }
        })
}