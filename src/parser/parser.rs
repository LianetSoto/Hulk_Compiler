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
        .map(|boxed| *boxed)
        .map_err(|err| {
            match err {
                ParseError::User { error } => error, 
                
                _ => {
                    let msg = format!("{:?}", err);
                    let span = match err {
                        ParseError::UnrecognizedToken { token, .. } => {
                            Some(Span::new(token.0, token.2))
                        }
                        ParseError::UnrecognizedEof { location, .. } => {
                            Some(Span::new(location, location))
                        }
                        _ => None,
                    };
                    CompilerError::ParserError { msg, span }
                }
            }
        })
}