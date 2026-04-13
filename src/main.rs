mod lexer;

use lexer::tokenize;

fn main() {
    let codigo = "print 42 + 3.14 a";
    let tokens = tokenize(codigo);
    println!("Tokens: {:?}", tokens);
}