mod parser;

use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use logos::{Lexer, Logos};

// Implemented using information from the [GNU assembler documentation](https://sourceware.org/binutils/docs/as/)
// and the [ARM developer documentation](https://developer.arm.com/documentation/).
#[derive(Logos, Debug, PartialEq, Eq, Hash)]
pub enum Token<'source> {
    #[error]
    Error,

    /// All whitespace except for newlines
    #[regex(r"(?imx) [\s && [^\r\n]]+")]
    Whitespace,

    #[token("\n")]
    #[token("\r")]
    #[token("\r\n")]
    #[token(";")]
    Newline,

    #[regex(r"(?imx) /\* (?: [^\*] | \*[^/] )* \*/", parse_multiline_comment)]
    #[regex(r"(?imx) // [^\n]*", parse_cstyle_line_comment)]
    #[regex(r"(?imx) @ [^\n]*", parse_single_char_line_comment)]
    Comment(&'source str),

    /// Used to represent labels, registers, instructions, directives, and string literals
    /// In the next pass, the parser will replace instructions and directives with a `KeySymbol` variant, and other
    /// symbols with a `RelativeSymbol` variant
    #[regex(r"(?imx) [a-zA-Z_.$][a-zA-Z0-9_.$]*", parse_unquoted_symbol)]
    #[regex(r#"(?imx) " (?: [^"] | \\. )* " "#, parse_quoted_symbol)]
    Symbol(String),

    /// Each statement (delimited by newlines) begins with zero or more labels, followed by a "key symbol" which can be
    /// either an instruction or a directive.
    KeySymbol(String),
    /// Used to represent labels, registers, and string literals.
    /// Holds the distance from the last occurrence of the symbol in the source code or 0 if this is
    /// the first occurrence of that symbol.
    RelativeSymbol(usize),

    /// A label is a symbol followed by a colon
    #[token(":")]
    Colon,

    // Constants
    #[regex(r"(?imx) 0b[01]+", parse_binary_integer)]
    #[regex(r"(?imx) 0[0-7]+", parse_octal_integer)]
    #[regex(r"(?imx) (?: [1-9][0-9]*) | 0", parse_decimal_integer)]
    #[regex(r"(?imx) 0x[0-9a-f]+", parse_hexadecimal_integer)]
    Integer(i64),

    #[regex(
        r"(?imx) 0 e [+-]? [0-9]* (?: \.[0-9]*)? (?: e [+-]? [0-9]+)?",
        parse_floating_point
    )]
    FloatingPoint(HashableFloat),

    #[regex(r#"(?imx) ' (?: [^"] | \\. ) ' "#)]
    Character(&'source str),

    #[token(",")]
    Comma,

    // Expressions
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,

    // Operators
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("%")]
    Remainder,
    #[token("<<")]
    ShiftLeft,
    #[token(">>")]
    ShiftRight,

    #[token("~")]
    BitwiseNot,
    #[token("&")]
    BitwiseAnd,
    #[token("|")]
    BitwiseOr,
    #[token("^")]
    BitwiseXor,
    #[token("!")]
    BitwiseOrNot,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("==")]
    Equals,
    #[token("<>")]
    #[token("!=")]
    NotEquals,
    #[token("<")]
    LessThan,
    #[token(">")]
    GreaterThan,
    #[token("<=")]
    LessThanOrEquals,
    #[token(">=")]
    GreaterThanOrEquals,

    #[token("&&")]
    LogicalAnd,
    #[token("||")]
    LogicalOr,

    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("#")]
    Hash,
}

#[must_use]
pub fn lex(s: &str) -> Vec<(Token, Range<usize>)> {
    let lexer = Token::lexer(s).spanned();

    // Perform a simple parsing pass, replacing `Symbol`s with `KeySymbol`s and `RelativeSymbol`s
    parser::parse(lexer)
}

#[inline]
fn parse_multiline_comment<'source>(lex: &mut Lexer<'source, Token<'source>>) -> &'source str {
    &lex.slice()[2..lex.slice().len() - 2]
}

#[inline]
fn parse_cstyle_line_comment<'source>(lex: &mut Lexer<'source, Token<'source>>) -> &'source str {
    &lex.slice()[2..]
}

#[inline]
fn parse_single_char_line_comment<'source>(
    lex: &mut Lexer<'source, Token<'source>>,
) -> &'source str {
    &lex.slice()[1..]
}

#[inline]
fn parse_unquoted_symbol<'source>(lex: &mut Lexer<'source, Token<'source>>) -> String {
    lex.slice().to_ascii_lowercase()
}

#[inline]
fn parse_quoted_symbol<'source>(lex: &mut Lexer<'source, Token<'source>>) -> String {
    let s = lex.slice();
    s[1..s.len() - 1].to_ascii_lowercase()
}

#[inline]
fn parse_binary_integer<'source>(lex: &mut Lexer<'source, Token<'source>>) -> i64 {
    i64::from_str_radix(&lex.slice()[2..], 2).unwrap()
}

#[inline]
fn parse_octal_integer<'source>(lex: &mut Lexer<'source, Token<'source>>) -> i64 {
    i64::from_str_radix(&lex.slice()[1..], 8).unwrap()
}

#[inline]
fn parse_decimal_integer<'source>(lex: &mut Lexer<'source, Token<'source>>) -> i64 {
    lex.slice().parse().unwrap()
}

#[inline]
fn parse_hexadecimal_integer<'source>(lex: &mut Lexer<'source, Token<'source>>) -> i64 {
    i64::from_str_radix(&lex.slice()[2..], 16).unwrap()
}

#[inline]
fn parse_floating_point<'source>(lex: &mut Lexer<'source, Token<'source>>) -> HashableFloat {
    HashableFloat(lex.slice()[2..].parse().unwrap())
}

#[derive(Debug, Clone, Copy)]
pub struct HashableFloat(f64);

impl Hash for HashableFloat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let bits = self.0.to_bits();
        bits.hash(state);
    }
}

impl PartialEq for HashableFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for HashableFloat {}

#[cfg(test)]
mod tests {
    use super::Token::*;
    use super::*;

    #[test]
    fn test_registers() {
        let tokens = lex("add sP");
        assert_eq!(
            tokens,
            vec![
                (KeySymbol("add".to_owned()), 0..3),
                (Whitespace, 3..4),
                (RelativeSymbol(0), 4..6)
            ]
        );
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(
            lex("  \n\t "),
            vec![(Whitespace, 0..3), (Newline, 3..4), (Whitespace, 4..6)]
        )
    }

    #[test]
    fn test_instruction() {
        assert_eq!(lex("add"), vec![(KeySymbol("add".to_owned()), 0..3)]);
        assert_eq!(lex("addne"), vec![(KeySymbol("addne".to_owned()), 0..5)]);
        assert_eq!(
            lex("YIELDS R0"),
            vec![
                (KeySymbol("yields".to_owned()), 0..6),
                (Whitespace, 6..7),
                (RelativeSymbol(0), 7..9)
            ]
        );
    }

    #[test]
    fn test_float() {
        assert_eq!(lex("0e0"), vec![(FloatingPoint(HashableFloat(0.0)), 0..3)]);
        assert_eq!(lex("0e+1"), vec![(FloatingPoint(HashableFloat(1.0)), 0..4)]);
        assert_eq!(
            lex("0e-1"),
            vec![(FloatingPoint(HashableFloat(-1.0)), 0..4)]
        );
        assert_eq!(
            lex("0e1e-1"),
            vec![(FloatingPoint(HashableFloat(0.1)), 0..6)]
        );
        assert_eq!(
            lex("0e-1.45"),
            vec![(FloatingPoint(HashableFloat(-1.45)), 0..7)]
        );
        assert_eq!(
            lex("0e-1.45e+2"),
            vec![(FloatingPoint(HashableFloat(-1.45e2)), 0..10)]
        );
    }

    #[test]
    fn test_different_symbols_hash_differently() {
        let mut set = std::collections::HashSet::new();
        set.insert(Symbol("add".to_owned()));
        set.insert(Symbol("sub".to_owned()));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_directives() {
        assert_eq!(
            lex(".word;.WORD;\".word\";\".WORD\""),
            vec![
                (KeySymbol(".word".to_owned()), 0..5),
                (Newline, 5..6),
                (KeySymbol(".word".to_owned()), 6..11),
                (Newline, 11..12),
                (KeySymbol(".word".to_owned()), 12..19),
                (Newline, 19..20),
                (KeySymbol(".word".to_owned()), 20..27),
            ]
        )
    }

    #[test]
    fn relative_symbols() {
        assert_eq!(
            lex("r1: r1: r1 r1, r1;; add r0, r1"),
            vec![
                (RelativeSymbol(0), 0..2),
                (Colon, 2..3),
                (Whitespace, 3..4),
                (RelativeSymbol(3), 4..6),
                (Colon, 6..7),
                (Whitespace, 7..8),
                (KeySymbol("r1".to_owned()), 8..10),
                (Whitespace, 10..11),
                (RelativeSymbol(5), 11..13),
                (Comma, 13..14),
                (Whitespace, 14..15),
                (RelativeSymbol(3), 15..17),
                (Newline, 17..18),
                (Newline, 18..19),
                (Whitespace, 19..20),
                (KeySymbol("add".to_owned()), 20..23),
                (Whitespace, 23..24),
                (RelativeSymbol(0), 24..26),
                (Comma, 26..27),
                (Whitespace, 27..28),
                (RelativeSymbol(9), 28..30),
            ]
        )
    }

    #[test]
    fn test_windows_carriage_return_handling() {
        assert_eq!(
            lex("\r\n\n \r\r"),
            vec![
                (Newline, 0..2),
                (Newline, 2..3),
                (Whitespace, 3..4),
                (Newline, 4..5),
                (Newline, 5..6),
            ]
        )
    }
}
