// Copyright 2026 Quantova Inc
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

const MAX_DEPTH: usize = 32;

pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = Parser {
        b: text.as_bytes(),
        at: 0,
    };
    let value = p.value(0)?;
    p.ws();
    if p.at != p.b.len() {
        return Err("trailing bytes after the vector".to_string());
    }
    Ok(value)
}

impl Json {
    pub fn get(&self, key: &str) -> Result<&Json, String> {
        match self {
            Json::Object(fields) => fields
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
                .ok_or_else(|| format!("the vector has no field {key}")),
            _ => Err(format!(
                "field {key} is read from something that is not an object"
            )),
        }
    }

    pub fn str_of(&self, key: &str) -> Result<String, String> {
        match self.get(key)? {
            Json::Str(s) => Ok(s.clone()),
            _ => Err(format!("field {key} is not a string")),
        }
    }

    pub fn num_of(&self, key: &str) -> Result<u128, String> {
        let text = match self.get(key)? {
            Json::Num(n) => n.as_str(),
            Json::Str(s) => s.as_str(),
            _ => return Err(format!("field {key} is not a number")),
        };
        if text.is_empty()
            || !text.bytes().all(|c| c.is_ascii_digit())
            || (text.len() > 1 && text.starts_with('0'))
        {
            return Err(format!("field {key} is not a canonical whole number"));
        }
        text.parse()
            .map_err(|_| format!("field {key} is out of the 128 bit range"))
    }

    pub fn u64_of(&self, key: &str) -> Result<u64, String> {
        u64::try_from(self.num_of(key)?).map_err(|_| format!("field {key} does not fit 64 bits"))
    }

    pub fn u32_of(&self, key: &str) -> Result<u32, String> {
        u32::try_from(self.num_of(key)?).map_err(|_| format!("field {key} does not fit 32 bits"))
    }

    pub fn u8_of(&self, key: &str) -> Result<u8, String> {
        u8::try_from(self.num_of(key)?).map_err(|_| format!("field {key} does not fit 8 bits"))
    }

    pub fn bool_of(&self, key: &str) -> Result<bool, String> {
        match self.get(key)? {
            Json::Bool(b) => Ok(*b),
            _ => Err(format!("field {key} is not true or false")),
        }
    }

    pub fn num_of_hex(&self, key: &str) -> Result<u128, String> {
        let text = self.str_of(key)?;
        let digits = text
            .strip_prefix("0x")
            .ok_or_else(|| format!("field {key} is not 0x hex"))?;
        if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("field {key} is not 0x hex"));
        }
        u128::from_str_radix(digits, 16).map_err(|_| format!("field {key} is out of range"))
    }
}

struct Parser<'a> {
    b: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.at < self.b.len() && matches!(self.b[self.at], b' ' | b'\t' | b'\n' | b'\r') {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.at).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected {} at byte {}", c as char, self.at))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err("the vector nests too deeply".to_string());
        }
        self.ws();
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') => self.word("true", Json::Bool(true)),
            Some(b'f') => self.word("false", Json::Bool(false)),
            Some(b'n') => self.word("null", Json::Null),
            Some(c) if c.is_ascii_digit() => {
                let start = self.at;
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.at += 1;
                }
                if matches!(self.peek(), Some(b'.' | b'e' | b'E' | b'-' | b'+')) {
                    return Err(
                        "the vector carries a number that is not a whole number".to_string()
                    );
                }
                Ok(Json::Num(
                    std::str::from_utf8(&self.b[start..self.at])
                        .map_err(|_| "a number is not text".to_string())?
                        .to_string(),
                ))
            }
            _ => Err(format!("unexpected input at byte {}", self.at)),
        }
    }

    fn word(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.b[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(format!("unexpected input at byte {}", self.at))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let start = self.at;
        loop {
            match self.peek() {
                None => return Err("a string is not terminated".to_string()),
                Some(b'"') => break,
                Some(b'\\') => return Err("escaped strings are not used in vectors".to_string()),
                Some(c) if c < 0x20 => return Err("a string holds a control byte".to_string()),
                Some(_) => self.at += 1,
            }
        }
        let text = std::str::from_utf8(&self.b[start..self.at])
            .map_err(|_| "a string is not UTF-8".to_string())?
            .to_string();
        self.at += 1;
        Ok(text)
    }

    fn array(&mut self, depth: usize) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.ws();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(format!("expected , or ] at byte {}", self.at)),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut fields: Vec<(String, Json)> = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Json::Object(fields));
        }
        loop {
            self.ws();
            let key = self.string()?;
            if fields.iter().any(|(k, _)| *k == key) {
                return Err(format!("the vector names field {key} twice"));
            }
            self.ws();
            self.expect(b':')?;
            let value = self.value(depth + 1)?;
            fields.push((key, value));
            self.ws();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Json::Object(fields));
                }
                _ => return Err(format!("expected , or }} at byte {}", self.at)),
            }
        }
    }
}
