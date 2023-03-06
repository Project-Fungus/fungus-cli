use std::usize;

/// ARM assembly tokens
#[derive(Debug, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, token_start: usize, token_end: usize) -> Self {
        Token {
            kind,
            span: Span {
                start: token_start,
                end: token_end,
            },
        }
    }
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        // When comparing whitespace tokens, treat all whitespace as equal
        match (self.kind, other.kind) {
            (TokenKind::Whitespace, TokenKind::Whitespace) => true,
            _ => self.kind == other.kind && self.span == other.span,
        }
    }
}

/// Span of the token in the source string, both ends inclusive
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Kinds of ARM assembly tokens including whitespace and comments
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub enum TokenKind {
    /// A sequence of whitespace including newlines
    Whitespace,
    /// A comment
    Comment,
    /// A word
    Word,
    /// A comma
    Comma,
    /// A colon
    Colon,
    // /// A label, starting at the first column of a line and ending with whitespace
    // Label,
    // /// An instruction
    // Instruction,
    // /// A directive
    // Directive,
}
