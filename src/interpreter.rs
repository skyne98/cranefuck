use std::io::{BufRead, Read, Write};

use anyhow::Result;
use thiserror::Error;

use crate::parser::{Ir, IrLoopType, Token};

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("parse int error")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("unmatched loop at index {index}")]
    UnmatchedLoop { index: usize },

    #[error("generic error")]
    Generic(#[from] anyhow::Error),
}

pub fn interpret(ir_ops: impl AsRef<[Ir]>) -> Result<i64, RuntimeError> {
    let mut memory = vec![0; 30_000];
    let mut instruction_pointer = 0;
    let mut data_pointer = 0;
    let ops = ir_ops.as_ref();

    loop {
        if instruction_pointer >= ops.len() {
            return Ok(memory[data_pointer]);
        }

        let op = &ops[instruction_pointer];
        match op {
            Ir::Move(amount) => {
                if data_pointer as isize + amount >= memory.len() as isize {
                    data_pointer =
                        (amount - (memory.len() as isize - data_pointer as isize)) as usize;
                } else if data_pointer as isize + amount < 0 {
                    data_pointer =
                        (memory.len() as isize - (amount - data_pointer as isize)) as usize;
                } else {
                    data_pointer = ((data_pointer as isize) + amount) as usize;
                }
            }
            Ir::Data(amount) => memory[data_pointer] += amount,
            Ir::IO(true) => {
                let mut buffer = [0; 1];
                std::io::stdin().read_exact(&mut buffer)?;

                if buffer[0] == 0xA {
                    println!("newline detected");
                    buffer[0] = 10;
                }

                memory[data_pointer] = buffer[0] as i64;
            }
            Ir::IO(false) => {
                let value = memory[data_pointer] as u8 as char;
                print!("{}", value);
                std::io::stdout().flush()?;
            }
            Ir::Loop(IrLoopType::Start, loop_match) => {
                let value = memory[data_pointer];
                if value == 0 {
                    instruction_pointer = loop_match + 1;
                    continue;
                }
            }
            Ir::Loop(IrLoopType::End, loop_match) => {
                let value = memory[data_pointer];
                if value != 0 {
                    instruction_pointer = *loop_match;
                    continue;
                }
            }
        }

        instruction_pointer += 1;
    }
}
