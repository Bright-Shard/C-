#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenType {
    And,
    Or,
    Xor,
    Not,

    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,

    /// `>-`
    Feather,
    /// `->`
    Arrow,

    Ampersand,
    Pipe,
    Caret,
    Tilde,
    LShift,
    RShift,

    Incr,
    Decr,
    Plus,
    Minus,
    Mul,
    Div,
    Pow,
    Modulo,

    Pub,

    Packed,
    Struct,
    Enum,
    Union,

    Fn,
    Defer,
    If,
    Then,
    Else,
    While,
    Loop,
    Continue,
    Break,

    Equal,
    Semi,
    Colon,
    Comma,
    Dot,
    LParens,
    RParens,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    String,
    // Char,
    Ident,
    Num,
}

#[derive(Debug)]
pub struct Tokens<'a> {
    /// The entire code file
    pub code: &'a str,
    /// Sorted list containing the position of all line breaks
    pub line_breaks: Vec<usize>,
    /// Token spans in the code
    pub spans: Vec<(&'a str, usize, usize)>,
    /// Respective token types
    pub types: Vec<TokenType>,
}

mod kw {
    pub const CONTINUE: &[u8] = b"continue";
    pub const PACKED: &[u8] = b"packed";
    pub const STRUCT: &[u8] = b"struct";
    pub const UNION: &[u8] = b"union";
    pub const DEFER: &[u8] = b"defer";
    pub const WHILE: &[u8] = b"while";
    pub const BREAK: &[u8] = b"break";
    pub const ENUM: &[u8] = b"enum";
    pub const THEN: &[u8] = b"then";
    pub const ELSE: &[u8] = b"else";
    pub const LOOP: &[u8] = b"loop";
    pub const AND: &[u8] = b"and";
    pub const XOR: &[u8] = b"xor";
    pub const NOT: &[u8] = b"not";
    pub const PUB: &[u8] = b"pub";
    pub const OR: &[u8] = b"or";
    pub const FN: &[u8] = b"fn";
    pub const IF: &[u8] = b"if";
}

mod op {
    pub const EQUALS: &[u8] = b"==";
    pub const NOT_EQUALS: &[u8] = b"!=";
    pub const LESS_EQUAL: &[u8] = b"<=";
    pub const GREATER_EQUAL: &[u8] = b">=";
    pub const FEATHER: &[u8] = b">-";
    pub const ARROW: &[u8] = b"->";
    pub const L_SHIFT: &[u8] = b"<<";
    pub const R_SHIFT: &[u8] = b">>";
    pub const INCR: &[u8] = b"++";
    pub const DECR: &[u8] = b"--";
    pub const POW: &[u8] = b"**";
    pub const MODULO: &[u8] = b"%";
    pub const LESS_THAN: &[u8] = b"<";
    pub const GREATER_THAN: &[u8] = b">";
    pub const AMPERSAND: &[u8] = b"&";
    pub const PIPE: &[u8] = b"|";
    pub const CARET: &[u8] = b"^";
    pub const TILDE: &[u8] = b"~";
    pub const PLUS: &[u8] = b"+";
    pub const MINUS: &[u8] = b"-";
    pub const MUL: &[u8] = b"*";
    pub const DIV: &[u8] = b"/";
    pub const EQUAL: &[u8] = b"=";
    pub const SEMI: &[u8] = b";";
    pub const COLON: &[u8] = b":";
    pub const COMMA: &[u8] = b",";
    pub const DOT: &[u8] = b".";
    pub const L_PARENS: &[u8] = b"(";
    pub const R_PARENS: &[u8] = b")";
    pub const L_BRACKET: &[u8] = b"[";
    pub const R_BRACKET: &[u8] = b"]";
    pub const L_BRACE: &[u8] = b"{";
    pub const R_BRACE: &[u8] = b"}";
}

pub fn tokenize(code: &str) -> Tokens<'_> {
    let mut line = 1;
    let mut line_start = code.as_ptr() as usize;

    let mut line_breaks = Vec::new();
    let mut spans = Vec::new();
    let mut types = Vec::new();

    let bcode = code.as_bytes();
    let start_addr = bcode.as_ptr() as usize;
    let mut input = bcode;
    while !input.is_empty() {
        // save line breaks
        while !input.is_empty() && input[0] == b'\n' {
            let addr = input.as_ptr() as usize;
            line_breaks.push(addr - start_addr);
            input = &input[1..];
            line_start = input.as_ptr() as usize;
            line += 1;
        }

        if input.is_empty() {
            break;
        }

        // ignore whitespace
        while input[0].is_ascii_whitespace() {
            input = &input[1..];
        }

        // ignore comments
        if input.starts_with(b"//") {
            input = &input[2..];
            while input[0] != b'\n' {
                input = &input[1..];
            }
            continue;
        }

        // operators
        {
            let mut op_len;
            let is_valid = 'valid: {
                op_len = 2;
                if input.len() >= op_len {
                    match &input[..op_len] {
                        op::EQUALS => {
                            types.push(TokenType::Equals);
                            break 'valid true;
                        }
                        op::NOT_EQUALS => {
                            types.push(TokenType::NotEquals);
                            break 'valid true;
                        }
                        op::LESS_EQUAL => {
                            types.push(TokenType::LessEqual);
                            break 'valid true;
                        }
                        op::GREATER_EQUAL => {
                            types.push(TokenType::GreaterEqual);
                            break 'valid true;
                        }
                        op::FEATHER => {
                            types.push(TokenType::Feather);
                            break 'valid true;
                        }
                        op::ARROW => {
                            types.push(TokenType::Arrow);
                            break 'valid true;
                        }
                        op::L_SHIFT => {
                            types.push(TokenType::LShift);
                            break 'valid true;
                        }
                        op::R_SHIFT => {
                            types.push(TokenType::RShift);
                            break 'valid true;
                        }
                        op::INCR => {
                            types.push(TokenType::Incr);
                            break 'valid true;
                        }
                        op::DECR => {
                            types.push(TokenType::Decr);
                            break 'valid true;
                        }
                        op::POW => {
                            types.push(TokenType::Pow);
                            break 'valid true;
                        }
                        _ => {}
                    }
                }

                op_len = 1;
                if input.len() >= op_len {
                    match &input[..op_len] {
                        op::MODULO => {
                            types.push(TokenType::Modulo);
                            break 'valid true;
                        }
                        op::LESS_THAN => {
                            types.push(TokenType::LessThan);
                            break 'valid true;
                        }
                        op::GREATER_THAN => {
                            types.push(TokenType::GreaterThan);
                            break 'valid true;
                        }
                        op::AMPERSAND => {
                            types.push(TokenType::Ampersand);
                            break 'valid true;
                        }
                        op::PIPE => {
                            types.push(TokenType::Pipe);
                            break 'valid true;
                        }
                        op::CARET => {
                            types.push(TokenType::Caret);
                            break 'valid true;
                        }
                        op::TILDE => {
                            types.push(TokenType::Tilde);
                            break 'valid true;
                        }
                        op::PLUS => {
                            types.push(TokenType::Plus);
                            break 'valid true;
                        }
                        op::MINUS => {
                            types.push(TokenType::Minus);
                            break 'valid true;
                        }
                        op::MUL => {
                            types.push(TokenType::Mul);
                            break 'valid true;
                        }
                        op::DIV => {
                            types.push(TokenType::Div);
                            break 'valid true;
                        }
                        op::EQUAL => {
                            types.push(TokenType::Equal);
                            break 'valid true;
                        }
                        op::SEMI => {
                            types.push(TokenType::Semi);
                            break 'valid true;
                        }
                        op::COLON => {
                            types.push(TokenType::Colon);
                            break 'valid true;
                        }
                        op::COMMA => {
                            types.push(TokenType::Comma);
                            break 'valid true;
                        }
                        op::DOT => {
                            types.push(TokenType::Dot);
                            break 'valid true;
                        }
                        op::L_PARENS => {
                            types.push(TokenType::LParens);
                            break 'valid true;
                        }
                        op::R_PARENS => {
                            types.push(TokenType::RParens);
                            break 'valid true;
                        }
                        op::L_BRACKET => {
                            types.push(TokenType::LBracket);
                            break 'valid true;
                        }
                        op::R_BRACKET => {
                            types.push(TokenType::RBracket);
                            break 'valid true;
                        }
                        op::L_BRACE => {
                            types.push(TokenType::LBrace);
                            break 'valid true;
                        }
                        op::R_BRACE => {
                            types.push(TokenType::RBrace);
                            break 'valid true;
                        }
                        _ => {}
                    }
                }

                false
            };

            if is_valid {
                let col = input.as_ptr() as usize - line_start;
                let span_slice = unsafe { std::str::from_utf8_unchecked(&input[..op_len]) };
                spans.push((span_slice, line, col));
                input = &input[op_len..];
                continue;
            }
        }

        // strings
        if input[0] == b'"' {
            let mut is_valid = false;

            let start_str_addr = input.as_ptr() as usize;
            input = &input[1..];
            while !input.is_empty() {
                if input.starts_with(br#"\""#) {
                    input = &input[2..];
                    continue;
                }

                if input[0] == b'"' {
                    is_valid = true;
                    input = &input[1..];
                    break;
                }

                // strings support line breaks
                if input[0] == b'\n' {
                    let addr = input.as_ptr() as usize;
                    line_breaks.push(addr - start_addr);
                    line_start = input.as_ptr() as usize;
                    line += 1;
                }

                input = &input[1..];
            }

            if is_valid {
                let end_str_addr = input.as_ptr() as usize;
                let start = start_str_addr - start_addr;
                let end = end_str_addr - start_addr;

                types.push(TokenType::String);
                let col = bcode.as_ptr() as usize + start - line_start;
                let span_slice = unsafe { std::str::from_utf8_unchecked(&bcode[start..end]) };
                spans.push((span_slice, line, col));
                continue;
            } else {
                let start = start_str_addr - start_addr;
                let end = bcode.len().min(start + 20);
                panic!(
                    "Unfinished string at line {line} ({:?})",
                    std::str::from_utf8(&bcode[start..end])
                );
            }
        }

        // identifiers
        if input[0].is_ascii_alphabetic() || input[0] == b'_' {
            let start_ident_addr = input.as_ptr() as usize;

            input = &input[1..];
            while input[0].is_ascii_alphanumeric() || input[0] == b'_' {
                input = &input[1..];
            }

            let end_ident_addr = input.as_ptr() as usize;
            let start = start_ident_addr - start_addr;
            let end = end_ident_addr - start_addr;

            let col = bcode.as_ptr() as usize + start - line_start;
            let ident_slice = &bcode[start..end];

            let mut token_len;
            let is_keyword = 'kw: {
                // keywords

                token_len = 8;
                if ident_slice.len() >= token_len {
                    if &ident_slice[..token_len] == kw::CONTINUE {
                        types.push(TokenType::Continue);
                        break 'kw true;
                    }
                }

                token_len = 6;
                if ident_slice.len() >= token_len {
                    match &ident_slice[..token_len] {
                        kw::PACKED => {
                            types.push(TokenType::Packed);
                            break 'kw true;
                        }
                        kw::STRUCT => {
                            types.push(TokenType::Struct);
                            break 'kw true;
                        }
                        _ => {}
                    }
                }

                token_len = 5;
                if ident_slice.len() >= token_len {
                    match &ident_slice[..token_len] {
                        kw::UNION => {
                            types.push(TokenType::Union);
                            break 'kw true;
                        }
                        kw::DEFER => {
                            types.push(TokenType::Defer);
                            break 'kw true;
                        }
                        kw::WHILE => {
                            types.push(TokenType::While);
                            break 'kw true;
                        }
                        kw::BREAK => {
                            types.push(TokenType::Break);
                            break 'kw true;
                        }
                        _ => {}
                    }
                }

                token_len = 4;
                if ident_slice.len() >= token_len {
                    match &ident_slice[..token_len] {
                        kw::ENUM => {
                            types.push(TokenType::Enum);
                            break 'kw true;
                        }
                        kw::THEN => {
                            types.push(TokenType::Then);
                            break 'kw true;
                        }
                        kw::ELSE => {
                            types.push(TokenType::Else);
                            break 'kw true;
                        }
                        kw::LOOP => {
                            types.push(TokenType::Loop);
                            break 'kw true;
                        }
                        _ => {}
                    }
                }

                token_len = 3;
                if ident_slice.len() >= token_len {
                    match &ident_slice[..token_len] {
                        kw::AND => {
                            types.push(TokenType::And);
                            break 'kw true;
                        }
                        kw::XOR => {
                            types.push(TokenType::Xor);
                            break 'kw true;
                        }
                        kw::NOT => {
                            types.push(TokenType::Not);
                            break 'kw true;
                        }
                        kw::PUB => {
                            types.push(TokenType::Pub);
                            break 'kw true;
                        }
                        _ => {}
                    }
                }

                token_len = 2;
                if ident_slice.len() >= token_len {
                    match &ident_slice[..token_len] {
                        kw::OR => {
                            types.push(TokenType::Or);
                            break 'kw true;
                        }
                        kw::FN => {
                            types.push(TokenType::Fn);
                            break 'kw true;
                        }
                        kw::IF => {
                            types.push(TokenType::If);
                            break 'kw true;
                        }
                        _ => {}
                    }
                }

                false
            };

            if !is_keyword {
                types.push(TokenType::Ident);
            }

            let span_slice = unsafe { std::str::from_utf8_unchecked(ident_slice) };
            spans.push((span_slice, line, col));
            continue;
        }

        // numbers
        if input[0].is_ascii_digit() {
            let start_ident_addr = input.as_ptr() as usize;

            // todo: support hex (0x), octal (0o) and binary (0b)
            let mut has_point = false;
            input = &input[1..];
            while input[0].is_ascii_digit() || input[0] == b'.' {
                if input[0] == b'.' {
                    if has_point {
                        break;
                    } else {
                        has_point = true;
                    }
                }

                input = &input[1..];
            }

            let end_ident_addr = input.as_ptr() as usize;
            let start = start_ident_addr - start_addr;
            let end = end_ident_addr - start_addr;

            types.push(TokenType::Num);
            let col = bcode.as_ptr() as usize + start - line_start;
            let span_slice = unsafe { std::str::from_utf8_unchecked(&bcode[start..end]) };
            spans.push((span_slice, line, col));
            continue;
        }

        panic!(
            "Cannot parse token at line {line} ({:?})",
            std::str::from_utf8(&input[..(input.len().min(20))]).unwrap()
        );
    }

    Tokens {
        code,
        line_breaks,
        spans,
        types,
    }
}
