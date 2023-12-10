use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub enum EntryKind {
    Article,
    Book,
    Booklet,
    Conference,
    Inbook,
    Incollection,
    Inproceedings,
    Manual,
    PhdThesis,
    Misc,
    Proceedings,
    Techreport,
    Unpublished,
}

impl EntryKind {
    pub fn try_from_ascii_u8(data: &[u8]) -> Result<Self, String> {
        match data {
            b"article" => Ok(Self::Article),
            b"book" => Ok(Self::Book),
            b"booklet" => Ok(Self::Booklet),
            b"conference" => Ok(Self::Conference),
            b"inbook" => Ok(Self::Inbook),
            b"incollection" => Ok(Self::Incollection),
            b"inproceedings" => Ok(Self::Inproceedings),
            b"manual" => Ok(Self::Manual),
            b"phdThesis" => Ok(Self::PhdThesis),
            b"misc" => Ok(Self::Misc),
            b"proceedings" => Ok(Self::Proceedings),
            b"techreport" => Ok(Self::Techreport),
            b"unpublished" => Ok(Self::Unpublished),
            i => Err(format!(
                "Invalid entry kind: '{}'",
                std::str::from_utf8(i).unwrap()
            )),
        }
    }
}

pub struct BibtexEntry {
    kind: EntryKind,
    name: String,
    author: String,
    year: String,
    title: String,
}

struct BibtexParser<'a> {
    data: &'a [u8], // we assume ascii content in bibtex entries
    cursor: usize,
}

struct KV {
    key: (usize, usize),
    value: (usize, usize),
}

impl<'a> BibtexParser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, cursor: 0 }
    }

    fn advance(&mut self) {
        self.cursor += 1;
    }

    fn skip_whitespace(&mut self) {
        while self.cursor < self.data.len() && self.data[self.cursor].is_ascii_whitespace() {
            self.cursor += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.cursor).copied()
    }

    fn consume(&mut self) -> Option<u8> {
        let c = self.data.get(self.cursor).copied();
        if c.is_some() {
            self.cursor += 1;
        }
        c
    }

    fn parse_kind(&mut self) -> Result<EntryKind, String> {
        if self.consume() != Some(b'@') {
            return Err("Invalid starting character. First non-whitespace character in entry needs to be '@'".to_string());
        }

        let type_start = self.cursor;
        while self.consume() != Some(b'{') {
            if self.cursor == self.data.len() {
                return Err("Unterminated entry type.".to_string());
            }
        }

        EntryKind::try_from_ascii_u8(&self.data[type_start..self.cursor - 1])
    }

    fn parse_key(&mut self) -> Result<&'a str, String> {
        let key_start = self.cursor;
        while let Some(c) = self.peek() {
            if c != b'=' && !c.is_ascii_whitespace() {
                self.advance()
            } else {
                break;
            }
        }
        let key_end = self.cursor;

        self.skip_whitespace();
        if self.consume() != Some(b'=') {
            return Err(format!(
                "Unterminated key: '{}'",
                std::str::from_utf8(&self.data[key_start..self.cursor - 1]).unwrap()
            ));
        }

        let key = std::str::from_utf8(&self.data[key_start..key_end]).unwrap();
        Ok(key)
    }

    fn parse_name(&mut self) -> Result<String, String> {
        let start = self.cursor;
        while self.consume() != Some(b',') {
            if self.peek().is_none() {
                return Err("Unterminated name, expected ','.".to_string());
            }
        }
        return Ok(std::str::from_utf8(&self.data[start..self.cursor - 1])
            .unwrap()
            .to_string());
    }

    fn parse_value(&mut self) -> Result<String, String> {
        let mut depth = 0;
        if let Some(b'{') = self.peek() {
            self.advance();
            depth += 1;
        }
        let start = self.cursor;
        while let Some(c) = self.consume() {
            match c {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(format!("Unmatched closed curly brace during parsing of value. Value so far: {}.", std::str::from_utf8(&self.data[start..self.cursor]).unwrap()));
                    }
                }
                _ => (),
            }

            if depth == 0 {
                if c == b',' {
                    let end = self.cursor - 1;
                    let value = std::str::from_utf8(&self.data[start..end]).unwrap();
                    return Ok(value.to_string());
                }
                if c == b'}' {
                    let reset = self.cursor;
                    let end = self.cursor - 1;
                    self.skip_whitespace();
                    let c = self.peek();
                    if c == Some(b',') {
                        // end of value in the middle of an entry
                        self.advance();
                        let value = std::str::from_utf8(&self.data[start..end]).unwrap();
                        return Ok(value.to_string());
                    } else if c == Some(b'}') {
                        let value = std::str::from_utf8(&self.data[start..end]).unwrap();
                        return Ok(value.to_string());
                    } else {
                        self.cursor = reset;
                    }
                }
            }
        }

        Err(format!(
            "Unterminated value: '{}'",
            std::str::from_utf8(&self.data[start..self.cursor - 1]).unwrap()
        ))
    }
}

impl BibtexEntry {
    pub fn try_from_str(content: &str) -> Result<Self, String> {
        let mut parser = BibtexParser::new(content.as_bytes());
        parser.skip_whitespace();
        let kind = parser.parse_kind()?;
        let name = parser.parse_name()?;
        let mut author = None;
        let mut year = None;
        let mut title = None;
        loop {
            parser.skip_whitespace();
            let key = parser.parse_key()?;
            parser.skip_whitespace();
            println!("{}", key);
            match key {
                "author" => author = Some(parser.parse_value()?),
                "year" => year = Some(parser.parse_value()?),
                "title" => title = Some(parser.parse_value()?),
                _ => {
                    let _ = parser.parse_value()?;
                }
            }
            parser.skip_whitespace();
            match parser.peek() {
                Some(b'}') => {
                    return Ok(BibtexEntry {
                        kind,
                        name,
                        title: title.ok_or("No title field in bibtex entry")?,
                        author: author.ok_or("No author field in bibtex entry")?,
                        year: year.ok_or("No year field in bibtex entry")?,
                    })
                } // we've reached the end of the entry
                None => {
                    // we've reached the end of the entry prematurely
                    return Err("Premature end of entry".to_string());
                }
                Some(_) => {
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn end_same_line() {
        let input = "@inproceedings{zhai2018autoencoder,
title={Autoencoder and its various variants},
author={Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang},
booktitle={2018 IEEE international conference on systems, man, and cybernetics (SMC)},
pages={415--419},
year={2018},
organization={IEEE} }";
        let entry = BibtexEntry::try_from_str(input).unwrap();
        assert_eq!(
            entry.author,
            "Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang"
        );
        assert_eq!(entry.year, "2018");
    }

    #[test]
    fn end_new_line() {
        let input = "@inproceedings{zhai2018autoencoder,
title={Autoencoder and its various variants},
author={Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang},
booktitle={2018 IEEE international conference on systems, man, and cybernetics (SMC)},
pages={415--419},
year={2018},
organization={IEEE}
}";
        let entry = BibtexEntry::try_from_str(input).unwrap();
        assert_eq!(
            entry.author,
            "Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang"
        );
        assert_eq!(entry.title, "Autoencoder and its various variants");
        assert_eq!(entry.year, "2018");
    }

    #[test]
    fn no_braces_title() {
        let input = "@inproceedings{zhai2018autoencoder,
title=Autoencoder and its various variants,
author={Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang},
booktitle={2018 IEEE international conference on systems, man, and cybernetics (SMC)},
pages={415--419},
year={2018},
organization={IEEE} }";
        let entry = BibtexEntry::try_from_str(input).unwrap();
        assert_eq!(
            entry.author,
            "Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang"
        );
        assert_eq!(entry.title, "Autoencoder and its various variants");
        assert_eq!(entry.year, "2018");
    }

    #[test]
    fn braces_middle_title() {
        let input = "@inproceedings{zhai2018autoencoder,
title=Autoencoder and its {various} variants,
author={Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang},
booktitle={2018 IEEE international conference on systems, man, and cybernetics (SMC)},
pages={415--419},
year={2018},
organization={IEEE} }";
        let entry = BibtexEntry::try_from_str(input).unwrap();
        assert_eq!(
            entry.author,
            "Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang"
        );
        assert_eq!(entry.title, "Autoencoder and its {various} variants");
        assert_eq!(entry.year, "2018");
    }

    #[test]
    fn spaces() {
        let input = "@inproceedings{zhai2018autoencoder,
title=Autoencoder and its {various} variants,
author = {Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang}   ,
booktitle={2018 IEEE international conference on systems, man, and cybernetics (SMC)},
pages={415--419},
year={2018},
organization={IEEE} }";
        let entry = BibtexEntry::try_from_str(input).unwrap();
        assert_eq!(
            entry.author,
            "Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang"
        );
        assert_eq!(entry.year, "2018");
    }
}
