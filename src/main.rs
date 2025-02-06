#![feature(ascii_char)]

use anyhow::Result;
use interpreter::interpret;

pub mod interpreter;
pub mod parser;

fn main() -> Result<()> {
    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let tokens = parser::tokenize(&input);
        println!("Tokens: {:?}", tokens);

        interpret(tokens)?;
    }

    Ok(())
}
