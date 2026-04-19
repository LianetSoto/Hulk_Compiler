use super::token::Token;
use crate::error::CompilerError;
use logos::Logos;
use crate::error::Span;

// Lexer personalizado que implementa Iterator para LALRPOP
pub struct Lexer<'input> {
    inner: logos::Lexer<'input, Token>,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            inner: Token::lexer(input),
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Result<(usize, Token, usize), CompilerError>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.inner.next()?;
        let span = self.inner.span();
        let start = span.start;
        let end = span.end;
        match token {
            Ok(tok) => Some(Ok((start, tok, end))),
            Err(()) => {
                let span = self.inner.span();
                let slice = self.inner.slice();
                let ch = slice.chars().next().unwrap_or('?');
                Some(Err(CompilerError::UnexpectedCharacter {
                    ch,
                    span: Span::new(span.start, span.end),
                }))
            }
        }
    }
}
