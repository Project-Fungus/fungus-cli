use std::hash::{Hash, Hasher};

use logos::{Lexer, Logos};

// Implemented using information from the [GNU assembler documentation](https://sourceware.org/binutils/docs/as/)
// and the [ARM developer documentation](https://developer.arm.com/documentation/).
#[derive(Logos, Debug, PartialEq, Eq, Hash)]
pub enum Token<'source> {
    #[error]
    Error,

    #[regex(r"(?imx) [\s && [^\n]]+ # all whitespace except for newlines")]
    Whitespace,

    #[token("\n")]
    #[token("\r\n")]
    #[token(";")]
    Newline,

    #[regex(r"(?imx) /\* (?: [^\*] | \*[^/] )* \*/", parse_multiline_comment)]
    #[regex(r"(?imx) // [^\n]*", parse_cstyle_line_comment)]
    #[regex(r"(?imx) # [^\n]*", parse_single_char_line_comment)]
    #[regex(r"(?imx) @ [^\n]*", parse_single_char_line_comment)]
    Comment(&'source str),

    #[regex(r"(?imx) [a-zA-Z_.$][a-zA-Z0-9_.$]*")]
    #[regex(r#"(?imx) " (?: [^"] | \\. )* " "#)]
    // Also used to represent string literals
    Symbol(&'source str),

    // A label is a symbol followed by a colon
    #[regex(r"(?imx) [a-zA-Z_.$][a-zA-Z0-9_.$]*:")]
    #[regex(r#"(?imx) " (?: [^"] | \\. )* ": "#)]
    Label(&'source str),

    // A directive is a symbol preceded by a dot
    #[regex(r"(?imx) \.[a-zA-Z_.$][a-zA-Z0-9_.$]*")]
    #[regex(r#"(?imx) \." (?: [^"] | \\. )* "#)]
    Directive(&'source str),

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
    #[regex(r"(?imx) r\d+ # r0-r15", parse_register)]
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
pub fn lex(s: &str) -> Vec<Token> {
    let lexer = Token::lexer(s);
    lexer.collect()
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
        let tokens = lex("R1 sP");
        assert_eq!(tokens, vec![Register(1), Whitespace, Register(13)]);
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(lex("  \n\t "), vec![Whitespace, Newline, Whitespace])
    }

    #[test]
    fn test_instruction() {
        assert_eq!(lex("add"), vec![Symbol("add")]);
        assert_eq!(lex("addne"), vec![Symbol("addne")]);
        assert_eq!(
            lex("YIELDS R0"),
            vec![Symbol("YIELDS"), Whitespace, Register(0)]
        );
    }

    #[test]
    fn test_float() {
        assert_eq!(lex("0e0"), vec![FloatingPoint(HashableFloat(0.0))]);
        assert_eq!(lex("0e+1"), vec![FloatingPoint(HashableFloat(1.0))]);
        assert_eq!(lex("0e-1"), vec![FloatingPoint(HashableFloat(-1.0))]);
        assert_eq!(lex("0e1e-1"), vec![FloatingPoint(HashableFloat(0.1))]);
        assert_eq!(lex("0e-1.45"), vec![FloatingPoint(HashableFloat(-1.45))]);
        assert_eq!(
            lex("0e-1.45e+2"),
            vec![FloatingPoint(HashableFloat(-1.45e2))]
        );
    }

    #[test]
    fn lex_radix_sort() {
        assert!(!lex(include_str!("../benches/radix_sort.s")).contains(&Error))
    }
}
