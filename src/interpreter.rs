use std::io::{BufRead, Read, Write};

use anyhow::Result;
use thiserror::Error;

use crate::parser::Token;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("parse int error")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("unmatched loop at index {index}")]
    UnmatchedLoop { index: usize },

    #[error("generic error")]
    Generic(anyhow::Error),
}

pub fn interpret(tokens: impl AsRef<[Token]>) -> Result<u8, RuntimeError> {
    let mut memory = vec![0; 30_000];
    let mut instruction_pointer = 0;
    let mut data_pointer = 0;
    let tokens = tokens.as_ref();

    loop {
        if instruction_pointer == tokens.len() {
            return Ok(memory[data_pointer]);
        }

        let token = &tokens[instruction_pointer];
        match token {
            Token::MoveRight => {
                if data_pointer + 1 >= memory.len() {
                    data_pointer = 0;
                } else {
                    data_pointer += 1;
                }
            }
            Token::MoveLeft => {
                if data_pointer == 0 {
                    data_pointer = memory.len() - 1;
                } else {
                    data_pointer -= 1;
                }
            }
            Token::Increment => memory[data_pointer] += 1,
            Token::Decrement => memory[data_pointer] -= 1,
            Token::Output => {
                let value = memory[data_pointer] as u8 as char;
                print!("{}", value);
                std::io::stdout().flush()?;
            }
            Token::Input => {
                let mut buffer = [0; 1];
                std::io::stdin().read_exact(&mut buffer)?;
                memory[data_pointer] = buffer[0];
            }
            Token::LoopStart => {
                let data_value = memory[data_pointer];

                if data_value == 0 {
                    let mut counter = 0;
                    let mut temporary_pointer = instruction_pointer + 1;

                    loop {
                        let token = &tokens[temporary_pointer];

                        match token {
                            Token::LoopStart => counter += 1,
                            Token::LoopEnd => {
                                if counter > 0 {
                                    counter -= 1;
                                } else {
                                    instruction_pointer = temporary_pointer + 1;
                                    break;
                                }
                            }
                            _ => (),
                        }

                        temporary_pointer += 1;
                    }
                }
            }
            Token::LoopEnd => {
                let data_value = memory[data_pointer];

                if data_value != 0 {
                    let mut counter = 0;
                    let mut temporary_pointer = instruction_pointer - 1;

                    loop {
                        let token = &tokens[temporary_pointer];

                        match token {
                            Token::LoopEnd => counter += 1,
                            Token::LoopStart => {
                                if counter > 0 {
                                    counter -= 1;
                                } else {
                                    instruction_pointer = temporary_pointer;
                                    break;
                                }
                            }
                            _ => (),
                        }

                        temporary_pointer -= 1;
                    }
                }
            }
        }

        instruction_pointer += 1;
    }
}
