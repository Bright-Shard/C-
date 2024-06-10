mod lexer;

const CODE: &str = include_str!("../Cඞඞ.sus");

fn main() {
    let tokens = lexer::lex("Cඞඞ.sus", CODE);
    println!("{}", &tokens);
}
