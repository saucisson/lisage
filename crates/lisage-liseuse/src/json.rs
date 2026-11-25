//! Minimal JSON parser (hand-written, no external dependency — same
//! policy as the trace serializer in `trace.rs`). Used by the trace
//! deserializer (D1-4, issue #158). Integers only: the trace schema
//! carries no floats.

/// A parsed JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// Boolean.
    Bool(bool),
    /// Integer.
    Num(i64),
    /// String.
    Str(String),
    /// Array.
    Arr(Vec<Json>),
    /// Object (insertion order preserved).
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Object field lookup.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// Integer value.
    pub fn as_num(&self) -> Option<i64> {
        match self {
            Json::Num(value) => Some(*value),
            _ => None,
        }
    }

    /// String value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(value) => Some(value),
            _ => None,
        }
    }

    /// Boolean value.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Array value.
    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }
}

/// Recursive-descent parser over a char vector.
struct Parser {
    chars: Vec<char>,
    position: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.position += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.position += 1;
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.next() {
            Some(c) if c == expected => Ok(()),
            other => Err(format!("expected '{expected}', got {other:?}")),
        }
    }

    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => Ok(Json::Str(self.parse_string()?)),
            Some('t') => self.parse_literal("true", Json::Bool(true)),
            Some('f') => self.parse_literal("false", Json::Bool(false)),
            Some('n') => self.parse_literal("null", Json::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            other => Err(format!("unexpected character {other:?}")),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: Json) -> Result<Json, String> {
        for expected in literal.chars() {
            self.expect(expected)?;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<Json, String> {
        let mut text = String::new();
        if self.peek() == Some('-') {
            text.push('-');
            self.position += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                self.position += 1;
            } else {
                break;
            }
        }
        if text.is_empty() || text == "-" {
            return Err("invalid number".to_string());
        }
        if matches!(self.peek(), Some('.' | 'e' | 'E')) {
            return Err("floats are not part of the trace schema".to_string());
        }
        text.parse::<i64>()
            .map(Json::Num)
            .map_err(|error| format!("invalid number '{text}': {error}"))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut output = String::new();
        loop {
            match self.next() {
                None => return Err("unterminated string".to_string()),
                Some('"') => return Ok(output),
                Some('\\') => match self.next() {
                    Some('"') => output.push('"'),
                    Some('\\') => output.push('\\'),
                    Some('/') => output.push('/'),
                    Some('n') => output.push('\n'),
                    Some('t') => output.push('\t'),
                    Some('r') => output.push('\r'),
                    Some('b') => output.push('\u{0008}'),
                    Some('f') => output.push('\u{000C}'),
                    Some('u') => {
                        let mut code = 0u32;
                        for _ in 0..4 {
                            let digit = self
                                .next()
                                .and_then(|c| c.to_digit(16))
                                .ok_or("invalid \\u escape")?;
                            code = code * 16 + digit;
                        }
                        output.push(char::from_u32(code).ok_or("invalid \\u code point")?);
                    }
                    other => return Err(format!("invalid escape {other:?}")),
                },
                Some(c) => output.push(c),
            }
        }
    }

    fn parse_array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.position += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.next() {
                Some(',') => {}
                Some(']') => return Ok(Json::Arr(items)),
                other => return Err(format!("expected ',' or ']', got {other:?}")),
            }
        }
    }

    fn parse_object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.position += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.next() {
                Some(',') => {}
                Some('}') => return Ok(Json::Obj(entries)),
                other => return Err(format!("expected ',' or '}}', got {other:?}")),
            }
        }
    }
}

/// Parses one JSON document (integers only).
pub fn parse(text: &str) -> Result<Json, String> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        position: 0,
    };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.position != parser.chars.len() {
        return Err("trailing characters after the JSON document".to_string());
    }
    Ok(value)
}
