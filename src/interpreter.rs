use std::{
    collections::VecDeque,
    io::{BufRead, Read, Write},
};

use anyhow::Result;
use thiserror::Error;

use crate::parser::{Ir, IrLoopType};

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("parse int error")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("generic error")]
    Generic(#[from] anyhow::Error),
}

pub fn interpret(ir_ops: impl AsRef<[Ir]>, ignore_io: bool) -> Result<u8, RuntimeError> {
    let mut memory = vec![0; 30_000];
    let mut instruction_pointer = 0;
    let mut data_pointer = 0;
    let ops = ir_ops.as_ref();
    let mut input_buffer: VecDeque<char> = VecDeque::new();

    // Set terminal to raw mode to allow reading stdin one key at a time
    // let mut stdout = std::io::stdout().into_raw_mode()?;
    // Use asynchronous stdin
    // let mut stdin = termion::async_stdin().keys();

    loop {
        if instruction_pointer >= ops.len() {
            return Ok(memory[data_pointer]);
        }

        let op = &ops[instruction_pointer];
        match op {
            Ir::Move(amount) => {
                data_pointer =
                    ((data_pointer as isize + amount).rem_euclid(memory.len() as isize)) as usize;
            }
            Ir::Data(amount) => {
                memory[data_pointer] = memory[data_pointer].wrapping_add_signed(*amount as i8);
            }
            Ir::IO(true) => {
                if ignore_io == false {
                    if input_buffer.len() == 0 {
                        let mut line = String::new();
                        std::io::stdin().read_line(&mut line)?;
                        line = line.replace("\r\n", "\n");
                        input_buffer.extend(line.chars());
                    }

                    let character = input_buffer.pop_front().unwrap();

                    if character == '\n' {
                        memory[data_pointer] = 10;
                    } else {
                        memory[data_pointer] = character as u8;
                    }
                }
            }
            Ir::IO(false) => {
                if ignore_io == false {
                    let value = memory[data_pointer] as u8;
                    let char = if value == 10 {
                        if cfg!(windows) {
                            "\r\n".to_string()
                        } else {
                            "\n".to_string()
                        }
                    } else {
                        (value as char).to_string()
                    };
                    print!("{}", char);
                    std::io::stdout().flush()?;
                }
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
