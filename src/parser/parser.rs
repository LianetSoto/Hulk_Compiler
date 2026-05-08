use lalrpop_util::lalrpop_mod;
use crate::lexer::Token;
use crate::ast::Program;
use crate::error::CompilerError;
use lalrpop_util::ParseError;
use crate::error::Span;

lalrpop_mod!(grammar);

pub fn parse_program(tokens: Vec<(usize, Token, usize)>) -> Result<Program, CompilerError> {
    let parser = grammar::ProgramParser::new();
    let token_iter = tokens.into_iter().map(|tok| Ok(tok));
    parser.parse(token_iter)
        .map(|boxed| *boxed)
        .map_err(|err| {
            match err {
                ParseError::User { error } => error,
                ParseError::UnrecognizedToken { token, .. } => {
                    let msg = format!("Unrecognized token: {:?}", token);
                    let span = Some(Span::new(token.0, token.2));
                    CompilerError::ParserError { msg, span }
                }
                ParseError::UnrecognizedEof { location, .. } => {
                    let msg = "Unexpected end of input".to_string();
                    let span = Some(Span::new(location, location));
                    CompilerError::ParserError { msg, span }
                }
                _ => {
                    let msg = format!("Parse error: {:?}", err);
                    CompilerError::ParserError { msg, span: None }
                }
            }
        })
}