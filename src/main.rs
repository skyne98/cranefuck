use anyhow::Result;
use clap::Parser;
use std::fs;
use std::io::{self, Read, Write};

mod interpreter;
mod jit;
mod parser;

/// A robust Brainfuck CLI tool with REPL, file, and piped input support.
#[derive(Parser, Debug)]
#[command(
    name = "brainfuck-cli",
    version = "1.0",
    about = "A robust Brainfuck CLI tool with REPL, file, and piped input support."
)]
struct Args {
    /// Path to a Brainfuck source file
    #[arg(short, long)]
    file: Option<String>,

    /// Execution mode: 'jit' (default) or 'interpreter'
    #[arg(short, long, default_value = "jit")]
    mode: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let verbose = args.verbose;

    // Determine the source of the Brainfuck code.
    let brainfuck_code = if let Some(file_path) = args.file {
        if verbose {
            println!("Reading Brainfuck code from file: {}", file_path);
        }
        fs::read_to_string(file_path)?
    } else if !atty::is(atty::Stream::Stdin) {
        if verbose {
            println!("Reading Brainfuck code from piped input...");
        }
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        if verbose {
            println!("Entering Brainfuck REPL mode. Type your code and press Enter.");
        } else {
            println!("Brainfuck REPL (press Ctrl+C to exit):");
        }
        let mut input = String::new();
        print!("> ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut input)?;
        input
    };

    if verbose {
        println!("Brainfuck code loaded: {:?}", brainfuck_code);
    }

    // Tokenize and convert code to an intermediate representation.
    let tokens = parser::tokenize(&brainfuck_code);
    if verbose {
        println!("Tokens: {:?}", tokens);
    }
    let ir = parser::to_ir(tokens)?;
    if verbose {
        println!("Intermediate Representation (IR): {:?}", ir);
    }

    // Execute the Brainfuck code based on the selected mode.
    match args.mode.as_str() {
        "interpreter" => {
            if verbose {
                println!("Executing Brainfuck code in interpreter mode...");
            }
            let result = interpreter::interpret(ir)?;
            println!("Result: {}", result);
        }
        "jit" => {
            if verbose {
                println!("Executing Brainfuck code in JIT mode...");
            }
            jit::jit(ir);
        }
        other => {
            eprintln!(
                "Error: Invalid mode '{}'. Use 'interpreter' or 'jit'.",
                other
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
