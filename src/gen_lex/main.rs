mod regex_ast;
mod dfa;
mod nfa;
mod lexer;
mod utils;

use crate::dfa::Dfa;
use crate::nfa::Nfa;
use crate::regex_ast::RegexParser;
use crate::lexer::Lexer;

fn main() {
    let patterns = vec![
        // --- Palabras Reservadas ---
        ("Let", "let"),
        ("In", "in"),
        ("If", "if"),
        ("Else", "else"),
        ("Elif", "elif"),
        ("While", "while"),
        ("For", "for"),
        ("Function", "function"),
        ("Print", "print"),
        ("True", "true"),
        ("False", "false"),
        ("Pi", "PI"),
        ("E", "E"),
        ("Sin", "sin"),
        ("Cos", "cos"),
        ("Tan", "tan"),
        ("Sqrt", "sqrt"),
        ("Log", "log"),
        ("Exp", "exp"),
        ("Rand", "rand"),

        // --- Multi-char operators (longest match first) ---
        ("Arrow", "=>"),
        ("Eq", "="),
        ("Assign", ":="),
        ("EqEq", "=="),
        ("Neq", "!="),
        ("Leq", "<="),
        ("Geq", ">="),

        // --- Single-char operators ---
        ("Lt", "<"),
        ("Gt", ">"),
        ("And", "&"),
        ("Or", "|"),
        ("Not", "!"),
        ("Plus", "+"),
        ("Minus", "-"),
        ("Mult", "*"),
        ("Div", "/"),
        ("Percent", "%"),
        ("Power", "^"),
        
        // --- Símbolos de Puntuación ---
        ("LParen", "("),
        ("RParen", ")"),
        ("LBrace", "{"),
        ("RBrace", "}"),
        ("Comma", ","),
        ("Semicolon", ";"),
        ("COLON", ":"),

        // --- Literales Complejos ---
        // Números: Uno o más dígitos
        ("Number", "(0|1|2|3|4|5|6|7|8|9)(0|1|2|3|4|5|6|7|8|9)*\\.?((0|1|2|3|4|5|6|7|8|9)(0|1|2|3|4|5|6|7|8|9)*"), 
        
        // Identificadores: Letra seguida de letras o números
        ("Identifier", "(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z)(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z|0|1|2|3|4|5|6|7|8|9|_)*"),
    ];

    // 1. Convertir strings a ASTs
    let mut asts = Vec::new();
    for (name, regex_str) in patterns {
        let mut parser = RegexParser::new(regex_str);
        asts.push((name.to_string(), parser.parse()));
    }

    // 2. Construir NFA Global
    let (nfa_global, token_map) = Nfa::join_all(asts.iter().map(|(n, a)| (n.clone(), a)).collect());

    // 3. Construir DFA
    let alphabet = vec![
        'l','e','t','i','n','f','s','e','w','o','u','p','r','b','d','y','h','g','k','c','v','m','x','z','q','j', 'a', 'w',
        '=','<','>','!','&','|','+','-','*','/','^','%',
        '(',')','{','}',',',';',':',
        '0','1','2','3','4','5','6','7','8','9','_', ' ','\n','\t', '.','"','\\'
    ];
    let dfa = Dfa::from_nfa(&nfa_global, &alphabet, &token_map);

    // 4. Probar con una línea de código HULK
    let input = "let _x = 5 in print(_x + 10) * 2;";
    let lexer = Lexer { dfa };
    let result = lexer.tokenize(input);

    println!("Tokens encontrados:");
    for token in result {
        println!("  {}: {}", token.token_type, token.value);
    }
}