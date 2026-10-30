use anyhow::{Context as _, Result};

use core::{
    fmt::{Debug, Display, Formatter},
    iter::{self, Peekable},
    str::{Chars, FromStr as _},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::asm::KEYWORDS;

use r8cpu::regs::Reg;

/// A source code token.
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// Hexadecimal byte literal (`0x00`).
    ByteLiteral(u8),
    /// Comma.
    Comma,
    /// Comment.
    Comment(String),
    /// Identifier.
    Identifier(String),
    /// Illegal token.
    Illegal(String),
    /// Include directive.
    Include(String),
    /// Assembler keyword.
    Keyword(String),
    /// Label definition.
    LabelDef(String),
    /// Newline character (`\n`).
    Newline,
    /// Closing parenthesis (`)`).
    ParenClose,
    /// Opening parenthesis (`(`).
    ParenOpen,
    /// Plus sign (`+`).
    Plus,
    /// Register name.
    Register(Reg),
    /// String literal.
    StringLiteral(String),
    /// Hexadecimal word literal (`0x0000`).
    WordLiteral(u16),
}

use Token::*;

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}

pub struct Tokenizer<'src> {
    /// Stores the source code being assembled.
    pub chars: Peekable<Chars<'src>>,
    /// Debug mode.
    pub debug: bool,
    /// Working directory (used to find included files).
    pub dir: Option<PathBuf>,
}

impl<'src> Tokenizer<'src> {
    /// Prints `msg` if in debug mode.
    pub fn debug_print(&self, msg: impl AsRef<str>) {
        if self.debug {
            println!("{}", msg.as_ref());
        }
    }

    /// Processes an include directive.
    ///
    /// # Errors
    ///
    /// * Any errors returned by [`tokenize_source_file`].
    pub fn include(&mut self, raw_path: &str) -> Result<Vec<Token>> {
        let mut path = PathBuf::from(raw_path);
        if path.is_relative()
            && let Some(parent) = self.dir.clone()
        {
            path = parent.join(path);
        }
        tokenize_source_file(&path)
    }

    #[must_use]
    pub fn new(source: &'src str) -> Self {
        Self {
            debug: false,
            chars: source.chars().peekable(),
            dir: None,
        }
    }

    #[must_use]
    pub fn new_with_debug(source: &'src str) -> Self {
        let mut tokenizer = Self::new(source);
        tokenizer.debug = true;
        tokenizer
    }

    /// Reads a comment token.
    pub fn read_comment(&mut self) -> Token {
        self.chars.next(); // skip ';' prefix
        Comment(self.read_rest_of_line())
    }

    /// Reads a hex literal token.
    pub fn read_hex_literal(&mut self) -> Token {
        self.chars.next();
        self.chars.next(); // skip "0x" prefix
        let literal: String =
            iter::from_fn(|| self.chars.next_if(char::is_ascii_hexdigit)).collect();
        match literal.len() {
            2 => match u8::from_str_radix(&literal, 16) {
                Ok(value) => ByteLiteral(value),
                Err(_) => Illegal(literal),
            },
            4 => match u16::from_str_radix(&literal, 16) {
                Ok(value) => WordLiteral(value),
                Err(_) => Illegal(literal),
            },
            _ => Illegal(literal),
        }
    }

    /// Reads an identifier, register name, or keyword.
    pub fn read_identifier(&mut self) -> Token {
        let ident: String = iter::from_fn(|| {
            self.chars
                .next_if(|&ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
        .collect();
        self.debug_print(format!("ident: {ident}"));
        match ident.as_str() {
            _ if let Ok(reg) = Reg::from_str(&ident) => Register(reg),
            "include" => Include(self.read_rest_of_line()),
            kw if KEYWORDS.contains(&kw) => Keyword(ident),
            label if let Some(&':') = self.chars.peek() => {
                self.chars.next();
                LabelDef(label.to_owned())
            }
            _ => Identifier(ident),
        }
    }

    /// Reads the remainder of the current line.
    pub fn read_rest_of_line(&mut self) -> String {
        self.skip_whitespace();
        iter::from_fn(|| self.chars.next_if(|&ch| ch != '\r' && ch != '\n')).collect()
    }

    /// Reads a string literal token.
    pub fn read_string(&mut self) -> Token {
        self.chars.next(); // skip opening " delimiter
        let string: String = iter::from_fn(|| self.chars.next_if(|&ch| ch != '"')).collect();
        self.chars.next(); // skip closing " delimiter
        StringLiteral(string)
    }

    /// Reads a given token.
    pub fn read_token(&mut self, token: Token) -> Token {
        self.chars.next();
        token
    }

    /// Advances to the next non-whitespace, non-newline character.
    pub fn skip_whitespace(&mut self) {
        while self
            .chars
            .next_if(|&ch| ch.is_whitespace() && ch != '\n')
            .is_some()
        {}
    }

    /// Scans tokens from the source code.
    ///
    /// # Errors
    ///
    /// * If processing an `include` directive fails.
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        self.skip_whitespace();
        while let Some(next_char) = self.chars.peek() {
            let next = *next_char;
            let token = match next {
                '"' => self.read_string(),
                '(' => self.read_token(ParenOpen),
                ')' => self.read_token(ParenClose),
                '+' => self.read_token(Plus),
                ',' => self.read_token(Comma),
                '0' => self.read_hex_literal(),
                ';' => self.read_comment(),
                '\n' => self.read_token(Newline),
                ch if ch.is_alphabetic() => self.read_identifier(),
                ch => self.read_token(Illegal(ch.to_string())),
            };
            self.debug_print(format!("token: {token}"));
            if let Include(path) = token {
                tokens.extend(self.include(&path)?);
            } else {
                tokens.push(token);
            }
            self.skip_whitespace();
        }
        Ok(tokens)
    }
}

/// Tokenize `input`.
///
/// # Errors
///
/// * As for [`Tokenizer::tokenize`].
pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    Tokenizer::new(input).tokenize()
}

/// Tokenize source file at `path`.
///
/// # Errors
///
/// * Any errors returned by [`fs::read_to_string`].
pub fn tokenize_source_file(path: impl AsRef<Path>) -> Result<Vec<Token>> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).context(format!("{}", path.display()))?;
    let mut tokenizer = Tokenizer::new(&source);
    if let Some(parent) = path.parent() {
        tokenizer.dir = Some(PathBuf::from(parent));
    }
    tokenizer.tokenize()
}

/// Tokenize `input` with verbose debugging.
///
/// # Errors
///
/// * As for [`Tokenizer::tokenize`].
pub fn tokenize_with_debug(input: &str) -> Result<Vec<Token>> {
    Tokenizer::new_with_debug(input).tokenize()
}
