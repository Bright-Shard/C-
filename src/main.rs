mod tokenizer;

const CODE: &str = include_str!("../Cඞඞ.sus");

fn main() {
    let tokens = tokenizer::tokenize("Cඞඞ.sus", CODE);
    println!("{}", &tokens);
}
