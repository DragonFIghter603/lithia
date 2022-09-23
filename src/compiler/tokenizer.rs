use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Peekable;
use std::num::{ParseFloatError, ParseIntError};
use std::str::{Chars, FromStr};
use std::str::pattern::Pattern;
use crate::compiler::compiler::{Loc, ParseError};
use crate::returnable::Returnable;
use crate::variable::Value;

pub(crate) struct Tokens(Vec<Token>);

impl Tokens {
    pub(crate) fn get_tokens(self) -> Vec<Token> {
        self.0
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Token {
    EndStmt(Loc),
    Ident(String, Loc),
    Bracket(Bracket, Loc),
    String(String, Loc),
    NumberLiteral(String, String, Loc),
    Assign(Loc),
    TypeSep(Loc),
    PathSep(Loc),
    ArgSep(Loc),
    EOF(Loc)
}

pub(crate) fn value_from_numer_literal(tok: Token) -> Returnable<Value, ParseError> {
    if let Token::NumberLiteral(val, typ, loc) = tok {
        let r: Result<Value, Box<dyn Error>> = try {
            match typ.as_str() {
                "u8" => Value::U8(u8::from_str(&val)?),
                "u16" => Value::U16(u16::from_str(&val)?),
                "u32" => Value::U32(u32::from_str(&val)?),
                "u64" => Value::U64(u64::from_str(&val)?),
                "u128" => Value::U128(u128::from_str(&val)?),

                "i8" => Value::I8(i8::from_str(&val)?),
                "i16" => Value::I16(i16::from_str(&val)?),
                "i32" => Value::I32(i32::from_str(&val)?),
                "i64" => Value::I64(i64::from_str(&val)?),
                "i128" => Value::I128(i128::from_str(&val)?),

                "f32" => Value::F32(f32::from_str(&val)?),
                "f64" => Value::F64(f64::from_str(&val)?),
                other => Err(loc.error(format!("Provided type '{}' is not valid as a number type", other)))?
            }
        };
        match r {
            Ok(v) => Returnable::Ok(v),
            Err(e) => Returnable::Err(loc.error(format!("Error while parsing number literal: {}", e)))
        }
    }
    else {
        Returnable::Err(tok.loc().error(format!("Unexpected token {:?} '{}', expected NumberLiteral", tok, tok)))
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum Side{
    Open,
    Close
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum Bracket{
    Curly(Side),
    Pointy(Side),
    Square(Side),
    Round(Side)
}

pub(crate) fn tokenize(code: &str) -> Result<Tokens, ParseError> {
    let mut input_iter = ParserIter::new(code);

    let mut tokens = Vec::<Token>::new();

    while let Some(&c) = input_iter.peek() {
        if c.is_whitespace() {
            input_iter.next();
        }
        else if c == ';' {
            tokens.push(Token::EndStmt(input_iter.here()));
            input_iter.next();
        }
        else if c == ',' {
            tokens.push(Token::ArgSep(input_iter.here()));
            input_iter.next();
        }
        else if c == '=' {
            tokens.push(Token::Assign(input_iter.here()));
            input_iter.next();
        }
        else if c == ':' {
            input_iter.next();
            if let Some(token) = tokens.pop() {
                if let Token::TypeSep(loc) = token {
                    tokens.push(Token::PathSep(loc.clone()))
                }
                else{
                    tokens.push(token);
                    tokens.push(Token::TypeSep(input_iter.here()))
                }
            }
            else{
                tokens.push(Token::TypeSep(input_iter.here()))
            }
        }
        else if c.is_contained_in("[]{}<>()") {
            tokens.push(Token::Bracket(match c {
                '{' => Bracket::Curly(Side::Open),
                '}' => Bracket::Curly(Side::Close),
                '<' => Bracket::Pointy(Side::Open),
                '>' => Bracket::Pointy(Side::Close),
                '[' => Bracket::Square(Side::Open),
                ']' => Bracket::Square(Side::Close),
                '(' => Bracket::Round(Side::Open),
                ')' => Bracket::Round(Side::Close),
                _ => unreachable!()
            }, input_iter.here()));
            input_iter.next();
        }
        else if c == '"' {
            tokens.push(tokenize_string(&mut input_iter))
        }
        else if c.is_ascii_digit() {
            tokens.push(tokenize_number_literal(&mut input_iter)?)
        }
        else if c.is_alphabetic() || c == '_' {
            tokens.push(tokenize_ident(&mut input_iter))
        }
        else {
            return Err(input_iter.here().error(format!("Unexpected char '{}'", c)));
        }
    }
    tokens.push(Token::EOF(input_iter.here()));

    Ok(Tokens(tokens))
}

fn tokenize_string(input_iter: &mut ParserIter) -> Token{
    let loc = input_iter.here();
    let mut ident = String::new();
    input_iter.next();
    while let Some(&c) = input_iter.peek() {
        if c != '"' {
            ident.push(c);
            input_iter.next();
        }
        else{
            input_iter.next();
            break;
        }
    }
    return Token::String(ident, loc);
}

fn tokenize_ident(input_iter: &mut ParserIter) -> Token{
    let loc = input_iter.here();
    let mut ident = String::new();
    while let Some(&c) = input_iter.peek() {
        if c.is_alphanumeric() || c == '_' {
            ident.push(c);
            input_iter.next();
        }
        else{
            break;
        }
    }
    return Token::Ident(ident, loc);
}

fn tokenize_number_literal(input_iter: &mut ParserIter) -> Result<Token, ParseError>{
    let loc = input_iter.here();
    let mut val = String::new();
    let mut ty = String::new();
    let mut had_dot = false;
    while let Some(c) = input_iter.peek() {
        if c.is_ascii_digit() {
            val.push(*c);
            input_iter.next();
        }
        else if c == &'.' {
            if !had_dot {
                val.push(*c);
                had_dot = true;
                input_iter.next();
            }
            else{
                input_iter.next();
                return Err(input_iter.here().error("Encountered decimal point for a second time in this number".to_string()));
            }
        }
        else{
            break;
        }
    }
    let here = input_iter.here();
    if let Some(c) = input_iter.peek() {
        if !c.is_alphabetic() {
            return Err(here.error(format!("Expected token after number to be alphabetic: '{}'", c)));
        }
    }
    else {
        return Err(input_iter.here().error("Expected Some token after number".to_string()));
    }
    while let Some(&c) = input_iter.peek() {
        if c.is_alphanumeric() || c == '_' {
            ty.push(c);
            input_iter.next();
        }
        else{
            break;
        }
    }
    Ok(Token::NumberLiteral(val, ty, loc))
}

#[derive(Clone)]
struct ParserIter<'a> {
    original: String,
    iter: Peekable<Chars<'a>>,
    index: usize
}

impl ParserIter<'_> {
    fn new(input: &str) -> ParserIter {
        ParserIter {
            original: input.clone().to_string(),
            iter: input.chars().peekable(),
            index: 0
        }
    }

    fn next(&mut self) -> Option<char>{
        self.index += 1;
        self.iter.next()
    }

    fn peek(&mut self) -> Option<&char>{
        self.iter.peek()
    }

    fn here(&self) -> Loc {
        Loc {
            original: self.original.clone(),
            index: self.index
        }
    }
}

impl Token {
    pub(crate) fn loc(&self) -> &Loc {
        match self {
            Token::EndStmt(loc) => loc,
            Token::Ident(_, loc) => loc,
            Token::Bracket(_, loc) => loc,
            Token::String(_, loc) => loc,
            Token::NumberLiteral(_, _, loc) => loc,
            Token::Assign(loc) => loc,
            Token::TypeSep(loc) => loc,
            Token::PathSep(loc) => loc,
            Token::ArgSep(loc) => loc,
            Token::EOF(loc) => loc
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Token::EndStmt(_) => ";".to_string(),
            Token::Ident(ident, _) => ident.to_string(),
            Token::Bracket(Bracket::Curly(side), _) => format!("{}\n", Bracket::Curly(side.clone())),
            Token::Bracket(bracket, _) => format!("{}", bracket),
            Token::String(string, _) => format!("\"{}\"", string),
            Token::NumberLiteral(num, n_type, _) => format!("{}{}", num, n_type),
            Token::Assign(_) => "=".to_string(),
            Token::TypeSep(_) => ":".to_string(),
            Token::PathSep(_) => "::".to_string(),
            Token::ArgSep(_) => ",".to_string(),
            Token::EOF(_) => "".to_string()
        })
    }
}

impl Display for Tokens {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for t in self.0.iter() {
            s.push_str(&format!("{} ", t))
        }
        write!(f, "{}", s.replace(";", ";\n"))
    }
}


impl Display for Bracket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Bracket::Curly(Side::Open) => "{",
            Bracket::Curly(Side::Close) => "}",
            Bracket::Pointy(Side::Open) => "<",
            Bracket::Pointy(Side::Close) => ">",
            Bracket::Square(Side::Open) => "[",
            Bracket::Square(Side::Close) => "]",
            Bracket::Round(Side::Open) => "(",
            Bracket::Round(Side::Close) => ")"
        })
    }
}
