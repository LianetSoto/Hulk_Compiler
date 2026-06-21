use crate::gen_lex::dfa::Dfa;
use crate::lexer::token::Token as LexerToken;
use crate::gen_lex::regex_ast::RegexParser;
use crate::gen_lex::nfa::Nfa;
use crate::error::{CompilerError, Span};

fn escape_regex_literal(pattern: &str) -> String {
    let mut escaped = String::new();
    for ch in pattern.chars() {
        match ch {
            '|' | '*' | '.' | '?' | '(' | ')' | '\\' | '+' | '[' | ']' | '{' | '}' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub struct Lexer {
    pub dfa: Dfa,
}

pub fn build_lexer(patterns: Vec<(&str, &str)>) -> Lexer {
    use crate::gen_lex::nfa::NfaState;
    use crate::gen_lex::utils::Edge;
    use std::collections::HashSet;

    let mut states = vec![NfaState { edges: vec![], is_accept: false }];
    let mut token_map = Vec::new();
    let start = 0;

    // Collect alphabet
    let mut alphabet = HashSet::new();
    for (_, pattern) in &patterns {
        for c in pattern.chars() {
            alphabet.insert(c);
        }
    }
    let alphabet: Vec<char> = alphabet.into_iter().collect();

    for (name, pattern) in patterns {
        let regex_str = match name {
            "Number" | "Identifier" => pattern.to_string(),
            _ => escape_regex_literal(pattern),
        };
        let mut regex_parser = RegexParser::new(&regex_str);
        let regex_ast = regex_parser.parse();
        let (pat_start, pat_end) = Nfa::build_from_ast(&regex_ast, &mut states);
        states[0].edges.push(Edge::Epsilon(pat_start));
        states[pat_end].is_accept = true;
        token_map.push((pat_end, name.to_string()));
    }

    let nfa = Nfa { states, start, end: 0 };
    let dfa = Dfa::from_nfa(&nfa, &alphabet, &token_map);
    Lexer { dfa }
}

impl Lexer {
    pub fn tokenize(&self, input: &str) -> Result<Vec<(usize, LexerToken, usize)>, CompilerError> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Saltar espacios
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }

            // Saltar comentarios
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // skip \n
                }
                continue;
            }

            // Números: entero o decimal
            if chars[i].is_ascii_digit() {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i < chars.len() && chars[i] == '.' {
                    let dot_pos = i;
                    i += 1;
                    let frac_start = i;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    if frac_start == i {
                        // No decimal digits after '.', retroceder y aceptar solo la parte entera
                        i = dot_pos;
                    }
                }
                let token_value: String = chars[start..i].iter().collect();
                tokens.push((start, LexerToken::Number(token_value.parse().unwrap()), i));
                continue;
            }

            // Identificadores / palabras reservadas
            if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let token_value: String = chars[start..i].iter().collect();
                let lexer_token = match token_value.as_str() {
                    "is" => LexerToken::Is,
                    "as" => LexerToken::As,
                    "let" => LexerToken::Let,
                    "in" => LexerToken::In,
                    "if" => LexerToken::If,
                    "else" => LexerToken::Else,
                    "elif" => LexerToken::Elif,
                    "while" => LexerToken::While,
                    "for" => LexerToken::For,
                    "function" => LexerToken::Function,
                    "true" => LexerToken::True,
                    "false" => LexerToken::False,
                    "PI" => LexerToken::Pi,
                    "E" => LexerToken::E,
                    "type" => LexerToken::Type,
                    "inherits" => LexerToken::Inherits,
                    "new" => LexerToken::New,
                    "protocol" => LexerToken::Protocol,
                    "extends" => LexerToken::Extends,
                    "range" => LexerToken::Range,
                    "Number" => LexerToken::NumberType,
                    "String" => LexerToken::StringType,
                    "Boolean" => LexerToken::BooleanType,
                    "Object" => LexerToken::ObjectType,
                    _ => LexerToken::Identifier(token_value),
                };
                tokens.push((start, lexer_token, i));
                continue;
            }

            // Strings: "..." with basic escape handling
            if chars[i] == '"' {
                let start = i;
                i += 1; // consume opening quote
                let mut string_content = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        match chars[i + 1] {
                            'n' => { string_content.push('\n'); i += 2; }
                            't' => { string_content.push('\t'); i += 2; }
                            '"' => { string_content.push('"'); i += 2; }
                            '\\' => { string_content.push('\\'); i += 2; }
                            c => {
                            // Carácter inválido después de la barra invertida
                            return Err(CompilerError::LexerError {
                                msg: format!("Invalid escape sequence: '\\{}'", c),
                                span: Span::new(i, i + 2),
                            });
                        }
                        }
                    } else {
                        string_content.push(chars[i]);
                        i += 1;
                    }
                }
                if i < chars.len() && chars[i] == '"' {
                    i += 1; // consume closing quote
                    tokens.push((start, LexerToken::Str(string_content), i));
                    continue;
                }
                else {
        return Err(CompilerError::LexerError {
        msg: "Unclosed string literal".to_string(),
        span: Span::new(start, i),
    });
        
            }
            }

            // Concat operators
            if chars[i] == '@' {
                if i + 1 < chars.len() && chars[i + 1] == '@' {
                    tokens.push((i, LexerToken::ConcatSpace, i + 2));
                    i += 2;
                } else {
                    tokens.push((i, LexerToken::Concat, i + 1));
                    i += 1;
                }
                continue;
            }

            // Dot operator
            if chars[i] == '.' {
                tokens.push((i, LexerToken::Dot, i + 1));
                i += 1;
                continue;
            }

            let mut current_state = 0;
            let mut last_accept_state: Option<usize> = None;
            let mut last_accept_pos: Option<usize> = None;
            let mut j = i;

            // Simular DFA
            while j < chars.len() {
                if let Some(&next_state) = self.dfa.transitions.get(&(current_state, chars[j])) {
                    current_state = next_state;
                    j += 1;
                    
                    // Verificar si este estado es aceptador
                    if self.dfa.accept_states.contains_key(&current_state) {
                        last_accept_state = Some(current_state);
                        last_accept_pos = Some(j - 1);
                    }
                } else {
                    break;
                }
            }

            if let (Some(accept_state), Some(pos)) = (last_accept_state, last_accept_pos) {
                let token_value: String = chars[i..=pos].iter().collect();
                let token_type = self.dfa.accept_states.get(&accept_state).cloned().unwrap_or_else(|| "UNKNOWN".to_string());
                let lexer_token = match token_type.as_str() {
                    "Number" => LexerToken::Number(token_value.parse().unwrap()),
                    "Identifier" => LexerToken::Identifier(token_value),
                    "Let" => LexerToken::Let,
                    "In" => LexerToken::In,
                    "If" => LexerToken::If,
                    "Else" => LexerToken::Else,
                    "Elif" => LexerToken::Elif,
                    "While" => LexerToken::While,
                    "For" => LexerToken::For,
                    "Function" => LexerToken::Function,
                    "Print" => LexerToken::Print,
                    "True" => LexerToken::True,
                    "False" => LexerToken::False,
                    "Pi" => LexerToken::Pi,
                    "E" => LexerToken::E,
                    "Type" => LexerToken::Type,
                    "Sin" => LexerToken::Sin,
                    "Cos" => LexerToken::Cos,
                    "Tan" => LexerToken::Tan,
                    "Sqrt" => LexerToken::Sqrt,
                    "Log" => LexerToken::Log,
                    "Exp" => LexerToken::Exp,
                    "Rand" => LexerToken::Rand,
                    "Arrow" => LexerToken::Arrow,
                    "RArrow" => LexerToken::RArrow,
                    "Eq" => LexerToken::Eq,
                    "Assign" => LexerToken::Assign,
                    "EqEq" => LexerToken::EqEq,
                    "Neq" => LexerToken::Neq,
                    "Leq" => LexerToken::Leq,
                    "Geq" => LexerToken::Geq,
                    "Lt" => LexerToken::Lt,
                    "Gt" => LexerToken::Gt,
                    "And" => LexerToken::And,
                    "Or" => LexerToken::Or,
                    "Not" => LexerToken::Not,
                    "Plus" => LexerToken::Plus,
                    "Minus" => LexerToken::Minus,
                    "Mult" => LexerToken::Mult,
                    "Div" => LexerToken::Div,
                    "Percent" => LexerToken::Percent,
                    "Power" => LexerToken::Power,
                    "LParen" => LexerToken::LParen,
                    "RParen" => LexerToken::RParen,
                    "LBrace" => LexerToken::LBrace,
                    "RBrace" => LexerToken::RBrace,
                    "Comma" => LexerToken::Comma,
                    "Semicolon" => LexerToken::Semicolon,
                    "COLON" => LexerToken::Colon,
                    "Dot" => LexerToken::Dot,
                    "New" => LexerToken::New,
                    "protocol" => LexerToken::Protocol,
                    "extends" => LexerToken::Extends,
                    "is" => LexerToken::Is,
                    "as" => LexerToken::As,
                    _ => {
                        // For unknown, perhaps skip or error
                        i += 1;
                        continue;
                    }
                };
                tokens.push((i, lexer_token, pos + 1));
                i = pos + 1;
            } else {
                    return Err(CompilerError::LexerError {
                    msg: format!("Carácter no reconocido: '{}'", chars[i]),
                    span: Span::new(i, i + 1),
                });
            }
        }
        Ok(tokens)
    }
}