use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use logos::{Lexer, Logos};

// Implemented using information from the [GNU assembler documentation](https://sourceware.org/binutils/docs/as/)
// and the [ARM developer documentation](https://developer.arm.com/documentation/).
#[derive(Logos, Clone, Debug, PartialEq, Eq, Hash)]
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

    #[regex(r"(?imx) [a-zA-Z_.$][a-zA-Z0-9_.$]*", parse_unquoted_symbol)]
    #[regex(r#"(?imx) " (?: [^"] | \\. )* " "#, parse_quoted_symbol)]
    /// Also used to represent string literals
    Symbol(String),

    /// A label is a symbol followed by a colon
    #[regex(r"(?imx) [a-zA-Z_.$][a-zA-Z0-9_.$]*:", parse_unquoted_label)]
    #[regex(r#"(?imx) " (?: [^"] | \\. )* ": "#, parse_quoted_label)]
    Label(String),

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

    // TODO: Note that this representation for registers is only valid for ARMv7, ARMv8 uses x0-x30, w0-w30, and some more special registers
    // r0-r15
    #[regex(r"(?imx) r\d+", parse_register)]
    // a1-a4
    #[regex(r"(?imx) a\d", parse_a_register)]
    // v1-v8
    #[regex(r"(?imx) v\d", parse_v_register)]
    #[regex(r"(?imx) tr | sb", |_| 9)]
    #[regex(r"(?imx) ip", |_| 12)]
    #[regex(r"(?imx) sp", |_| 13)]
    #[regex(r"(?imx) lr", |_| 14)]
    #[regex(r"(?imx) pc", |_| 15)]
    Register(u8),

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
    #[token(":")]
    Colon,
}

#[must_use]
pub fn lex(s: &str) -> Vec<(Token, Range<usize>)> {
    Token::lexer(s).spanned().collect()
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
fn parse_unquoted_label<'source>(lex: &mut Lexer<'source, Token<'source>>) -> String {
    let s = lex.slice();
    s[0..s.len() - 1].to_ascii_lowercase()
}

#[inline]
fn parse_quoted_label<'source>(lex: &mut Lexer<'source, Token<'source>>) -> String {
    let s = lex.slice();
    s[1..s.len() - 2].to_ascii_lowercase()
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

#[inline]
fn parse_register<'source>(lex: &mut Lexer<'source, Token<'source>>) -> Result<u8, ()> {
    match lex.slice()[1..].parse() {
        Ok(n) if n <= 15 => Ok(n),
        _ => Err(()),
    }
}

#[inline]
fn parse_a_register<'source>(lex: &mut Lexer<'source, Token<'source>>) -> Result<u8, ()> {
    match lex.slice()[1..].parse::<u8>() {
        Ok(n) if n <= 4 => Ok(n - 1),
        _ => Err(()),
    }
}

#[inline]
fn parse_v_register<'source>(lex: &mut Lexer<'source, Token<'source>>) -> Result<u8, ()> {
    match lex.slice()[1..].parse::<u8>() {
        Ok(n) if n <= 8 => Ok(n + 3),
        _ => Err(()),
    }
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
    use itertools::Itertools;

    use super::Token::*;
    use super::*;

    #[test]
    fn test_registers() {
        let tokens = lex("R1 sP");
        assert_eq!(
            tokens,
            vec![
                (Register(1), 0..2),
                (Whitespace, 2..3),
                (Register(13), 3..5)
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
        assert_eq!(lex("add"), vec![(Symbol("add".to_owned()), 0..3)]);
        assert_eq!(lex("addne"), vec![(Symbol("addne".to_owned()), 0..5)]);
        assert_eq!(
            lex("YIELDS R0"),
            vec![
                (Symbol("yields".to_owned()), 0..6),
                (Whitespace, 6..7),
                (Register(0), 7..9)
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
    fn lex_radix_sort() {
        assert!(!lex(include_str!("../../benches/radix_sort.s"))
            .iter()
            .map(|(t, _)| t)
            .contains(&Error))
    }

    #[test]
    fn test_labels() {
        assert_eq!(
            lex("main: MAIN: \"main\": \"MAIN\":"),
            vec![
                (Label("main".to_owned()), 0..5),
                (Whitespace, 5..6),
                (Label("main".to_owned()), 6..11),
                (Whitespace, 11..12),
                (Label("main".to_owned()), 12..19),
                (Whitespace, 19..20),
                (Label("main".to_owned()), 20..27),
            ]
        )
    }

    #[test]
    fn test_directives() {
        assert_eq!(
            lex(".word .WORD \".word\" \".WORD\""),
            vec![
                (Symbol(".word".to_owned()), 0..5),
                (Whitespace, 5..6),
                (Symbol(".word".to_owned()), 6..11),
                (Whitespace, 11..12),
                (Symbol(".word".to_owned()), 12..19),
                (Whitespace, 19..20),
                (Symbol(".word".to_owned()), 20..27),
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
