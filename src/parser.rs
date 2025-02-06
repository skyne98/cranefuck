// > < + - . , [ ]

use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Increment,
    Decrement,
    MoveRight,
    MoveLeft,
    Output,
    Input,
    LoopStart,
    LoopEnd,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    input
        .chars()
        .filter_map(|c| match c {
            '+' => Some(Token::Increment),
            '-' => Some(Token::Decrement),
            '>' => Some(Token::MoveRight),
            '<' => Some(Token::MoveLeft),
            '.' => Some(Token::Output),
            ',' => Some(Token::Input),
            '[' => Some(Token::LoopStart),
            ']' => Some(Token::LoopEnd),
            _ => None,
        })
        .collect()
}

#[derive(Error, Debug)]
pub enum IrError {
    #[error("unmatched loop at index {index}")]
    UnmatchedLoop { index: usize },

    #[error("generic error")]
    Generic(#[from] anyhow::Error),
}

#[derive(Debug, PartialEq, Clone)]
pub enum IrLoopType {
    Start,
    End,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Ir {
    Data(i64),
    Move(isize),
    IO(bool),
    Loop(IrLoopType, usize),
}

pub fn to_ir(tokens: impl AsRef<[Token]>) -> Result<Vec<Ir>, IrError> {
    let tokens = tokens.as_ref();
    let mut ir_ops = Vec::with_capacity(tokens.len());
    let mut token_to_ir_map = HashMap::with_capacity(tokens.len());

    for (instruction_pointer, token) in tokens.iter().enumerate() {
        match token {
            Token::MoveRight => {
                if let Some(Ir::Move(ref mut amount)) = ir_ops.last_mut() {
                    *amount += 1;
                } else {
                    ir_ops.push(Ir::Move(1))
                }
            }
            Token::MoveLeft => {
                if let Some(Ir::Move(ref mut amount)) = ir_ops.last_mut() {
                    *amount -= 1;
                } else {
                    ir_ops.push(Ir::Move(-1))
                }
            }
            Token::Increment => {
                if let Some(Ir::Data(ref mut amount)) = ir_ops.last_mut() {
                    *amount += 1;
                } else {
                    ir_ops.push(Ir::Data(1))
                }
            }
            Token::Decrement => {
                if let Some(Ir::Data(ref mut amount)) = ir_ops.last_mut() {
                    *amount -= 1;
                } else {
                    ir_ops.push(Ir::Data(-1))
                }
            }
            Token::Output => ir_ops.push(Ir::IO(false)),
            Token::Input => ir_ops.push(Ir::IO(true)),
            Token::LoopStart => {
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
                                ir_ops.push(Ir::Loop(IrLoopType::Start, temporary_pointer));
                                break;
                            }
                        }
                        _ => (),
                    }

                    temporary_pointer += 1;
                    if temporary_pointer == tokens.len() {
                        return Err(IrError::UnmatchedLoop {
                            index: instruction_pointer,
                        });
                    }
                }
            }
            Token::LoopEnd => {
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
                                ir_ops.push(Ir::Loop(IrLoopType::End, temporary_pointer));
                                break;
                            }
                        }
                        _ => (),
                    }

                    temporary_pointer -= 1;
                    if temporary_pointer == tokens.len() {
                        return Err(IrError::UnmatchedLoop {
                            index: instruction_pointer,
                        });
                    }
                }
            }
        }

        token_to_ir_map.insert(instruction_pointer, ir_ops.len() - 1);
    }

    // Put proper IR indices for loops, instead of the token ones
    for ref mut ir_op in ir_ops.iter_mut() {
        match ir_op {
            Ir::Loop(_, ref mut index) => *index = *token_to_ir_map.get(index).unwrap(),
            _ => (),
        }
    }

    Ok(ir_ops)
}
