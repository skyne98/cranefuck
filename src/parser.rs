// > < + - . , [ ]

#[derive(Debug, PartialEq)]
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
