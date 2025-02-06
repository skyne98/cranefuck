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

        let ir = parser::to_ir(tokens.clone())?;
        println!("IR: {:?}", ir);

        let result = interpret(ir)?;
        println!("Result: {}", result);
    }

    Ok(())
}
