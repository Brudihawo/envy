use std::collections::HashMap;

pub struct Lexer<'a> {
    text: &'a str,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, cursor: 0 }
    }

    fn next_token(&mut self) -> Option<&'a str> {
        if self.cursor + 1 > self.text.as_bytes().len() {
            return None;
        }

        let text = std::str::from_utf8(&self.text.as_bytes()[self.cursor..]).unwrap();
        let mut word_start = None;

        for (index, c) in text.char_indices() {
            self.cursor += c.len_utf8();
            if word_start.is_none() {
                if !c.is_alphanumeric() {
                    continue;
                }

                word_start = Some(index);
            } else {
                if c.is_alphanumeric() {
                    continue;
                }

                let word = &text.as_bytes()[word_start.unwrap()..index];
                let word = std::str::from_utf8(word).unwrap();
                return Some(word);
            }
        }

        if word_start.is_none() {
            return None;
        }

        return Some(text);
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
