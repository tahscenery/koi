use crate::source::Cursor;
use crate::syntax::{self, SyntaxKind};
use unicode_xid::UnicodeXID;

/// Checks if the given character is a valid start of an identifier. A valid
/// start of an identifier is any Unicode code point that satisfies the
/// `XID_Start` property.
fn is_identifier_start(c: char) -> bool {
    // Fast-path for ASCII identifiers
    ('a' <= c && c <= 'z')
        || ('A' <= c && c <= 'Z')
        || c == '_'
        || c.is_xid_start()
}

/// Checks if the given character is a valid continuation of an identifier.
/// A valid continuation of an identifier is any Unicode code point that
/// satisfies the `XID_Continue` property.
fn is_identifier_continue(c: char) -> bool {
    // Fast-path for ASCII identifiers
    ('a' <= c && c <= 'z')
        || ('A' <= c && c <= 'Z')
        || ('0' <= c && c <= '9')
        || c == '_'
        || c.is_xid_continue()
}

/// Checks if the given character is a grouping delimiter.
#[allow(dead_code)]
fn is_grouping_delimiter(c: char) -> bool {
    matches!(c, '{' | '}' | '[' | ']' | '(' | ')')
}

/// Checks if the given character is a recognised symbol.
#[rustfmt::skip]
fn is_symbol(c: char) -> bool {
    match c {
        '&' | '*' | '@' | '!' | '^' | ':' | ',' | '$' | '.' | '–' | '—' | '=' |
        '-' | '%' | '+' | '#' | '?' | ';' | '£' | '~' | '|' | '/' | '\\'| '<' |
        '>' | '{' | '}' | '[' | ']' | '(' | ')' => true,
        _ => false,
    }
}

/// Checks if the given character is a digit.
fn is_digit(c: char) -> bool {
    matches!(c, '0'..='9')
}

/// Checks if the given character is a whitespace delimiter.
fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

#[derive(Clone, Debug, PartialEq)]
pub enum LexerMode {
    Normal,
    Grouping,
}

impl Default for LexerMode {
    fn default() -> Self {
        Self::Normal
    }
}

pub struct Lexer {
    cursor: Cursor,
    consumed_chars: Vec<char>,
    mode_stack: Vec<LexerMode>,
}

impl Lexer {
    pub fn new(source: String) -> Self {
        Self {
            cursor: Cursor::new(source),
            consumed_chars: Vec::new(),
            mode_stack: vec![LexerMode::Normal],
        }
    }

    #[allow(dead_code)]
    pub(crate) fn push_mode(&mut self, mode: LexerMode) {
        self.mode_stack.push(mode);
    }

    #[allow(dead_code)]
    pub(crate) fn pop_mode(&mut self) -> Option<LexerMode> {
        self.mode_stack.pop()
    }

    fn current_mode(&self) -> LexerMode {
        self.mode_stack.last().cloned().unwrap_or_default()
    }
}

impl Lexer {
    /// Retrieves the next character in the iterator.
    fn next_char(&mut self) -> Option<char> {
        self.cursor.advance().map(|c| {
            self.consumed_chars.push(c);
            c
        })
    }

    /// Peeks the next character without consuming it.
    fn peek(&self) -> char {
        self.peek_at(0)
    }

    /// Peeks the character at the given index without consuming it.
    fn peek_at(&self, n: usize) -> char {
        self.cursor.nth(n)
    }

    /// Checks if the `Lexer` has reached the end of the input.
    pub(crate) fn is_at_end(&self) -> bool {
        self.cursor.source_len() == 0
    }

    #[allow(dead_code)]
    pub(crate) fn current_pos(&self) -> usize {
        self.cursor.pos
    }

    /// Attempts to consume the next character if it matches the provided
    /// character `c`. Returns a `bool` indicating if it was successful or not.
    #[allow(dead_code)]
    fn consume(&mut self, c: char) -> bool {
        if self.peek() == c {
            self.next_char();
            true
        } else {
            false
        }
    }

    /// Consumes the input while the given `predicate` holds true. Returns the
    /// count of characters traversed.
    fn consume_while<F>(&mut self, predicate: F) -> usize
    where
        F: Fn(char) -> bool,
    {
        let mut consumed = 0;
        while predicate(self.peek()) && !self.is_at_end() {
            self.next_char();
            consumed += 1;
        }
        consumed
    }

    /// Consumes the input while the given `predicate` holds true, building a
    /// `Vec<char>` for all the characters consumed.
    fn consume_build<F>(&mut self, predicate: F) -> Vec<char>
    where
        F: Fn(char) -> bool,
    {
        let mut vec = Vec::new();
        while predicate(self.peek()) && !self.is_at_end() {
            if let Some(c) = self.next_char() {
                vec.push(c);
            }
        }
        vec
    }
}

impl Lexer {
    fn tokenize_normal(&mut self) -> Option<(SyntaxKind, String)> {
        let kind = match self.next_char()? {
            c if is_whitespace(c) => self.lex_whitespace(c),
            c if is_symbol(c) => self.lex_symbol(c),
            c if is_identifier_start(c) => self.lex_identifier(c),
            c if is_digit(c) => self.lex_number(c),
            c => todo!("Lexer::tokenize_normal({:?})", c),
        };

        let consumed = self.consumed_chars.drain(..).collect();
        Some((kind, consumed))
    }

    fn lex_whitespace(&mut self, _: char) -> SyntaxKind {
        self.consume_while(is_whitespace);
        SyntaxKind::Whitespace
    }

    /// Matches any character that is a valid symbol.
    ///
    /// _TODO:_ Perhaps we could handle cases with confused symbols, such as
    /// U+037E, the Greek question mark, which looks like a semicolon (compare
    /// ';' with ';').
    fn lex_symbol(&mut self, symbol: char) -> SyntaxKind {
        match symbol {
            '?' => {
                if (self.peek(), self.peek_at(1)) == ('?', '?') {
                    // Consume the next two question marks
                    self.next_char();
                    self.next_char();
                    SyntaxKind::Kwd_Unimplemented
                } else {
                    SyntaxKind::Sym_Question
                }
            }
            _ => {
                if let Some(symbol) =
                    syntax::symbol_from_two_chars(symbol, self.peek())
                {
                    self.next_char();
                    symbol
                } else {
                    syntax::symbol_from_char(symbol)
                }
            }
        }
    }

    /// Matches every character that can be part of an identifier. This includes
    /// upper and lower-case letters, decimal digits and the underscore.
    fn lex_identifier(&mut self, first_char: char) -> SyntaxKind {
        let rest = self.consume_build(is_identifier_continue);
        let vec = [&vec![first_char], &rest[..]].concat();
        let string: String = vec.into_iter().collect();
        self.lex_keyword_or_identifier(string)
    }

    /// Attempts to match the provided `string` to a keyword, returning a
    /// `TokenKind::Keyword` if a match is found, otherwise a
    /// `TokenKind::Identifier`.
    #[rustfmt::skip]
    fn lex_keyword_or_identifier(&mut self, string: String) -> SyntaxKind {
        match &*string {
            "alias"     => SyntaxKind::Kwd_Alias,
            "and"       => SyntaxKind::Kwd_And,
            "as"        => SyntaxKind::Kwd_As,
            "const"     => SyntaxKind::Kwd_Const,
            "else"      => SyntaxKind::Kwd_Else,
            "extend"    => SyntaxKind::Kwd_Extend,
            "external"  => SyntaxKind::Kwd_External,
            "for"       => SyntaxKind::Kwd_For,
            "function"  => SyntaxKind::Kwd_Function,
            "if"        => SyntaxKind::Kwd_If,
            "import"    => SyntaxKind::Kwd_Import,
            "in"        => SyntaxKind::Kwd_In,
            "internal"  => SyntaxKind::Kwd_Internal,
            "let"       => SyntaxKind::Kwd_Let,
            "match"     => SyntaxKind::Kwd_Match,
            "module"    => SyntaxKind::Kwd_Module,
            "not"       => SyntaxKind::Kwd_Not,
            "of"        => SyntaxKind::Kwd_Of,
            "or"        => SyntaxKind::Kwd_Or,
            "public"    => SyntaxKind::Kwd_Public,
            "ref"       => SyntaxKind::Kwd_Ref,
            "return"    => SyntaxKind::Kwd_Return,
            "take"      => SyntaxKind::Kwd_Take,
            "type"      => SyntaxKind::Kwd_Type,
            "var"       => SyntaxKind::Kwd_Var,
            "where"     => SyntaxKind::Kwd_Where,
            "while"     => SyntaxKind::Kwd_While,
            "with"      => SyntaxKind::Kwd_With,
            _           => SyntaxKind::Identifier,
        }
    }

    /// Matches any valid sequence of digits that can form an integer or float
    /// literal.
    ///
    /// The lexer doesn't verify if the the number literal is correctly
    /// formatted in binary, octal, or hexadecimal. Essentially, only integers
    /// should use the aforementioned bases and must start with `0` followed by
    /// a letter to differentiate the which base is desired.
    fn lex_number(&mut self, _: char) -> SyntaxKind {
        fn is_digit_continue(c: char) -> bool {
            matches!(c, '_' | '0'..='9' | 'a'..='z' | 'A'..='Z')
        }

        // Consume while we find underscores, digits, or letters (for base
        // literals such as hexadecimal `0xfff` or binary `0b101`).
        self.consume_while(is_digit_continue);

        // Check if there's a decimal point.
        if self.peek() == '.' && self.peek_at(1) != '.' {
            self.next_char();
            self.consume_while(is_digit_continue);
            SyntaxKind::Lit_Float
        } else {
            SyntaxKind::Lit_Integer
        }
    }
}

impl Iterator for Lexer {
    type Item = (SyntaxKind, String);

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_mode() {
            LexerMode::Normal => self.tokenize_normal(),
            LexerMode::Grouping => todo!("LexerMode::Grouping"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(input: impl Into<String> + Clone, kind: SyntaxKind) {
        let mut lexer = Lexer::new(input.clone().into());
        assert_eq!(lexer.next(), Some((kind, input.into())));
    }

    #[test]
    fn test_lex_keywords() {
        check("???", SyntaxKind::Kwd_Unimplemented);
        check("alias", SyntaxKind::Kwd_Alias);
        check("and", SyntaxKind::Kwd_And);
        check("as", SyntaxKind::Kwd_As);
        check("const", SyntaxKind::Kwd_Const);
        check("else", SyntaxKind::Kwd_Else);
        check("extend", SyntaxKind::Kwd_Extend);
        check("external", SyntaxKind::Kwd_External);
        check("for", SyntaxKind::Kwd_For);
        check("function", SyntaxKind::Kwd_Function);
        check("if", SyntaxKind::Kwd_If);
        check("import", SyntaxKind::Kwd_Import);
        check("in", SyntaxKind::Kwd_In);
        check("internal", SyntaxKind::Kwd_Internal);
        check("let", SyntaxKind::Kwd_Let);
        check("match", SyntaxKind::Kwd_Match);
        check("module", SyntaxKind::Kwd_Module);
        check("not", SyntaxKind::Kwd_Not);
        check("of", SyntaxKind::Kwd_Of);
        check("or", SyntaxKind::Kwd_Or);
        check("public", SyntaxKind::Kwd_Public);
        check("ref", SyntaxKind::Kwd_Ref);
        check("return", SyntaxKind::Kwd_Return);
        check("take", SyntaxKind::Kwd_Take);
        check("type", SyntaxKind::Kwd_Type);
        check("var", SyntaxKind::Kwd_Var);
        check("where", SyntaxKind::Kwd_Where);
        check("while", SyntaxKind::Kwd_While);
        check("with", SyntaxKind::Kwd_With);
    }

    #[test]
    fn test_lex_symbols() {
        check("&", SyntaxKind::Sym_Ampersand);
        check("*", SyntaxKind::Sym_Asterisk);
        check("@", SyntaxKind::Sym_At);
        check("\\", SyntaxKind::Sym_BackSlash);
        check("!", SyntaxKind::Sym_Bang);
        check("^", SyntaxKind::Sym_Caret);
        check(":", SyntaxKind::Sym_Colon);
        check(",", SyntaxKind::Sym_Comma);
        check("$", SyntaxKind::Sym_Dollar);
        check(".", SyntaxKind::Sym_Dot);
        check("—", SyntaxKind::Sym_EmDash);
        check("–", SyntaxKind::Sym_EnDash);
        check("=", SyntaxKind::Sym_Eq);
        check("/", SyntaxKind::Sym_ForwardSlash);
        check("-", SyntaxKind::Sym_Minus);
        check("%", SyntaxKind::Sym_Percent);
        check("|", SyntaxKind::Sym_Pipe);
        check("+", SyntaxKind::Sym_Plus);
        check("#", SyntaxKind::Sym_Pound);
        check("?", SyntaxKind::Sym_Question);
        check(";", SyntaxKind::Sym_Semicolon);
        check("£", SyntaxKind::Sym_Sterling);
        check("~", SyntaxKind::Sym_Tilde);

        check("<", SyntaxKind::Sym_Lt);
        check(">", SyntaxKind::Sym_Gt);
        check("<=", SyntaxKind::Sym_LtEq);
        check(">=", SyntaxKind::Sym_GtEq);
        check("<-", SyntaxKind::Sym_LThinArrow);
        check("->", SyntaxKind::Sym_RThinArrow);
        check("=>", SyntaxKind::Sym_ThickArrow);

        check("{", SyntaxKind::Sym_LBrace);
        check("}", SyntaxKind::Sym_RBrace);
        check("[", SyntaxKind::Sym_LBracket);
        check("]", SyntaxKind::Sym_RBracket);
        check("(", SyntaxKind::Sym_LParen);
        check(")", SyntaxKind::Sym_RParen);
    }

    #[test]
    fn test_lex_literal_numbers() {
        check("0", SyntaxKind::Lit_Integer);
        check("123", SyntaxKind::Lit_Integer);
        check("123.321", SyntaxKind::Lit_Float);
    }

    #[test]
    fn test_lex_identifiers() {
        check("a", SyntaxKind::Identifier);
        check("abc", SyntaxKind::Identifier);
        check("abc123", SyntaxKind::Identifier);
        check("abc_123_abc", SyntaxKind::Identifier);
        check("abc_123_abc_123", SyntaxKind::Identifier);
    }

    #[test]
    fn test_lex_identifiers_unicode() {
        // Latin-extended
        check("åçéîñøœßü", SyntaxKind::Identifier);
        check("njerëzore", SyntaxKind::Identifier);
        check("čovjek", SyntaxKind::Identifier);
        check("člověk", SyntaxKind::Identifier);

        // Other scripts
        check("بشري", SyntaxKind::Identifier); // Arabic
        check("ሰው", SyntaxKind::Identifier); // Amharic
        check("մարդ", SyntaxKind::Identifier); // Armenian
        check("মানব", SyntaxKind::Identifier); // Bengali
        check("人的", SyntaxKind::Identifier); // Chinese
        check("человек", SyntaxKind::Identifier); // Cyrillic
        check("मानव", SyntaxKind::Identifier); // Devanagari
        check("ადამიანური", SyntaxKind::Identifier); // Gregorian
        check("άνθρωπος", SyntaxKind::Identifier); // Greek
        check("માનવ", SyntaxKind::Identifier); // Gujarati
        check("אנוש", SyntaxKind::Identifier); // Hebrew
        check("ヒューマン", SyntaxKind::Identifier); // Japanese (Katakana)
        check("ಮಾನವ", SyntaxKind::Identifier); // Kannada
        check("មនុស្ស", SyntaxKind::Identifier); // Khmer
        check("인간", SyntaxKind::Identifier); // Korean
        check("ມະນຸດ", SyntaxKind::Identifier); // Lao
        check("മനുഷ്യൻ", SyntaxKind::Identifier); // Malayalam
        check("လူ့", SyntaxKind::Identifier); // Myanmar
        check("ମାନବ", SyntaxKind::Identifier); // Odia
        check("มนุษย์", SyntaxKind::Identifier); // Thai
    }
}
