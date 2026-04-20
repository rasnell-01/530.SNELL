//! Lexer for the Turtle Graphics language.
//!
//! Produces a `TokenStream`: a parallel list of tokens and the 1-based source
//! line number on which each token appears.  The parser uses line numbers to
//! produce useful error messages.
//!
//! Keywords are case-insensitive.  Comments start with `#`.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // --- Drawing commands ---
    Forward,
    Turn,
    Pen,
    Color,   // color <r> <g> <b>  — set drawing color (0-255 per channel)

    // --- Variables and I/O ---
    Set,
    Print,

    // --- Control flow ---
    DoTimes,

    // --- Procedure support ---
    To,

    // --- Block delimiters ---
    LBrace,
    RBrace,

    // --- Expression delimiters ---
    LParen,
    RParen,

    // --- Arithmetic operators ---
    Plus,
    Minus,
    Star,
    Slash,

    // --- Value tokens ---
    Number(f64),
    Ident(String),
}

/// Output of the lexer.
pub struct TokenStream {
    pub tokens: Vec<Token>,
    /// `lines[i]` is the 1-based source line of `tokens[i]`.
    pub lines: Vec<usize>,
}

/// Tokenize `input` into a `TokenStream`.
pub fn tokenize(input: &str) -> TokenStream {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut lines_vec: Vec<usize> = Vec::new();
    let mut i = 0;
    let mut line = 1usize;

    while i < chars.len() {
        let tok_line = line;

        match chars[i] {
            '\n'           => { line += 1; i += 1; continue; }
            ' ' | '\t' | '\r' => { i += 1; continue; }

            '#' => {
                while i < chars.len() && chars[i] != '\n' { i += 1; }
                continue;
            }

            '{' => { tokens.push(Token::LBrace); i += 1; }
            '}' => { tokens.push(Token::RBrace); i += 1; }
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }

            '+' => { tokens.push(Token::Plus);  i += 1; }
            '*' => { tokens.push(Token::Star);  i += 1; }
            '/' => { tokens.push(Token::Slash); i += 1; }

            '-' => {
                let next_numeric = chars.get(i + 1)
                    .map(|c| c.is_ascii_digit() || *c == '.')
                    .unwrap_or(false);
                if next_numeric {
                    i += 1;
                    let (n, consumed) = parse_number(&chars[i..]);
                    tokens.push(Token::Number(-n));
                    i += consumed;
                } else {
                    tokens.push(Token::Minus);
                    i += 1;
                }
            }

            c if c.is_ascii_digit() || c == '.' => {
                let (n, consumed) = parse_number(&chars[i..]);
                tokens.push(Token::Number(n));
                i += consumed;
            }

            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let token = match word.to_lowercase().as_str() {
                    "forward" | "fd"            => Token::Forward,
                    "turn"    | "rt" | "right"  => Token::Turn,
                    "pen"                        => Token::Pen,
                    "color"   | "colour"         => Token::Color,
                    "set"                        => Token::Set,
                    "print"                      => Token::Print,
                    "dotimes" | "repeat"         => Token::DoTimes,
                    "to"      | "define"         => Token::To,
                    _                            => Token::Ident(word),
                };
                tokens.push(token);
            }

            _ => { i += 1; continue; }
        }

        lines_vec.push(tok_line);
    }

    TokenStream { tokens, lines: lines_vec }
}

fn parse_number(chars: &[char]) -> (f64, usize) {
    let mut s = String::new();
    let mut seen_dot = false;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '.' if !seen_dot => { seen_dot = true; s.push('.'); i += 1; }
            c if c.is_ascii_digit() => { s.push(c); i += 1; }
            _ => break,
        }
    }
    (s.parse::<f64>().unwrap_or(0.0), i.max(1))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{tokenize, Token};

    /// Convenience: lex `src` and return only the token types, dropping line info.
    fn toks(src: &str) -> Vec<Token> {
        tokenize(src).tokens
    }

    // ── Keywords ──────────────────────────────────────────────────────────────

    #[test]
    fn lex_all_keywords() {
        let src = "forward turn pen color set print dotimes to";
        assert_eq!(
            toks(src),
            vec![
                Token::Forward, Token::Turn, Token::Pen, Token::Color,
                Token::Set, Token::Print, Token::DoTimes, Token::To,
            ]
        );
    }

    #[test]
    fn lex_keyword_aliases() {
        // fd → Forward, rt → Turn, repeat → DoTimes, define → To, colour → Color
        let src = "fd rt repeat define colour";
        assert_eq!(
            toks(src),
            vec![Token::Forward, Token::Turn, Token::DoTimes, Token::To, Token::Color]
        );
    }

    #[test]
    fn lex_keywords_are_case_insensitive() {
        assert_eq!(toks("FORWARD"), vec![Token::Forward]);
        assert_eq!(toks("Turn"),    vec![Token::Turn]);
        assert_eq!(toks("PEN"),     vec![Token::Pen]);
        assert_eq!(toks("DoTimes"), vec![Token::DoTimes]);
    }

    // ── Numbers ───────────────────────────────────────────────────────────────

    #[test]
    fn lex_positive_integer() {
        assert_eq!(toks("42"), vec![Token::Number(42.0)]);
    }

    #[test]
    fn lex_positive_float() {
        assert_eq!(toks("3.14"), vec![Token::Number(3.14)]);
    }

    #[test]
    fn lex_negative_integer() {
        assert_eq!(toks("-90"), vec![Token::Number(-90.0)]);
    }

    #[test]
    fn lex_negative_float() {
        assert_eq!(toks("-1.5"), vec![Token::Number(-1.5)]);
    }

    #[test]
    fn lex_zero() {
        assert_eq!(toks("0"), vec![Token::Number(0.0)]);
    }

    // ── Identifiers ───────────────────────────────────────────────────────────

    #[test]
    fn lex_identifier() {
        assert_eq!(toks("myVar"), vec![Token::Ident("myVar".into())]);
    }

    #[test]
    fn lex_single_letter_identifier() {
        assert_eq!(toks("n"), vec![Token::Ident("n".into())]);
    }

    #[test]
    fn lex_identifier_with_underscore() {
        assert_eq!(toks("my_proc"), vec![Token::Ident("my_proc".into())]);
    }

    // ── Operators and delimiters ──────────────────────────────────────────────

    #[test]
    fn lex_arithmetic_operators() {
        assert_eq!(
            toks("+ - * /"),
            vec![Token::Plus, Token::Minus, Token::Star, Token::Slash]
        );
    }

    #[test]
    fn lex_delimiters() {
        assert_eq!(
            toks("{ } ( )"),
            vec![Token::LBrace, Token::RBrace, Token::LParen, Token::RParen]
        );
    }

    /// A '-' not immediately followed by a digit becomes a Minus operator.
    #[test]
    fn lex_minus_as_binary_operator() {
        // "(n - 5)" → LParen Ident("-") Minus Number RParen
        let ts = toks("(n - 5)");
        assert!(matches!(ts[2], Token::Minus));
        assert!(matches!(ts[3], Token::Number(v) if (v - 5.0).abs() < 1e-12));
    }

    /// A '-' immediately followed by a digit is a negative-number prefix.
    #[test]
    fn lex_negative_prefix_no_space() {
        // "turn -90" → Turn, Number(-90)
        let ts = toks("turn -90");
        assert_eq!(ts[0], Token::Turn);
        assert!(matches!(ts[1], Token::Number(v) if (v + 90.0).abs() < 1e-12));
    }

    // ── Comments ──────────────────────────────────────────────────────────────

    #[test]
    fn lex_comment_is_skipped() {
        // Everything after '#' to end-of-line is dropped.
        let ts = toks("# this is a comment\nforward");
        assert_eq!(ts, vec![Token::Forward]);
    }

    #[test]
    fn lex_inline_comment() {
        let ts = toks("forward 100 # move ahead");
        assert_eq!(ts, vec![Token::Forward, Token::Number(100.0)]);
    }

    #[test]
    fn lex_empty_input() {
        assert!(toks("").is_empty());
    }

    #[test]
    fn lex_whitespace_only() {
        assert!(toks("   \t\n  ").is_empty());
    }

    // ── Line numbers ──────────────────────────────────────────────────────────

    #[test]
    fn lex_line_numbers_start_at_one() {
        let stream = tokenize("forward");
        assert_eq!(stream.lines[0], 1);
    }

    #[test]
    fn lex_line_numbers_increment_on_newline() {
        let stream = tokenize("forward\nturn\npen");
        assert_eq!(stream.lines, vec![1, 2, 3]);
    }

    #[test]
    fn lex_line_numbers_skip_comment_lines() {
        // token on line 1, then a comment line, then token on line 3
        let stream = tokenize("forward\n# comment\nturn");
        assert_eq!(stream.lines[0], 1);
        assert_eq!(stream.lines[1], 3);
    }

    // ── Full small programs ───────────────────────────────────────────────────

    #[test]
    fn lex_simple_program() {
        let ts = toks("pen 1\nforward 100\nturn 90");
        assert_eq!(
            ts,
            vec![
                Token::Pen,     Token::Number(1.0),
                Token::Forward, Token::Number(100.0),
                Token::Turn,    Token::Number(90.0),
            ]
        );
    }

    #[test]
    fn lex_expression_in_parens() {
        let ts = toks("(n + 5)");
        assert_eq!(
            ts,
            vec![
                Token::LParen,
                Token::Ident("n".into()),
                Token::Plus,
                Token::Number(5.0),
                Token::RParen,
            ]
        );
    }
}
