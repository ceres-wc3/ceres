use std::collections::HashMap;
use std::iter::Peekable;
use std::str::from_utf8;

use crate::parser::crlf::Lines;

#[derive(Debug)]
pub struct Entry<'src> {
    pub id:     &'src str,
    pub values: HashMap<&'src str, &'src str>,
}

#[derive(Debug)]
pub struct Entries<'src> {
    lines: Peekable<Lines<'src>>,
}

impl<'src> Entries<'src> {
    pub fn new(source: &'src [u8]) -> Entries<'src> {
        Entries {
            lines: Lines::new(source).peekable(),
        }
    }
}

fn parse_entry_start(mut input: &[u8]) -> Option<&str> {
    if input[0] == b'[' {
        input = &input[1..];
        if input.is_empty() {
            return None;
        }

        let (end, _) = input.iter().enumerate().find(|(_, c)| **c == b']')?;

        return Some(from_utf8(&input[..end]).ok()?);
    }

    None
}

fn parse_entry_value(input: &[u8]) -> Option<(&str, &str)> {
    if input.starts_with(b"//") {
        return None;
    }

    let equals = input
        .iter()
        .enumerate()
        .find(|(_, c)| **c == b'=')
        .map(|(i, _)| i);

    if let Some(equals) = equals {
        let key = from_utf8(&input[..equals]).ok()?;
        let value = from_utf8(&input[equals + 1..]).ok()?;

        if key.is_empty() || value.is_empty() {
            return None;
        }

        Some((key, value))
    } else {
        None
    }
}

impl<'src> Iterator for Entries<'src> {
    type Item = Entry<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        self.lines.find_map(|l| parse_entry_start(l)).map(|id| {
            let mut values = HashMap::default();

            loop {
                if self.lines.peek().is_none() {
                    break;
                }

                if self
                    .lines
                    .peek()
                    .and_then(|l| parse_entry_start(l))
                    .is_some()
                {
                    break;
                }

                if let Some((key, value)) = self.lines.next().and_then(|l| parse_entry_value(l)) {
                    values.insert(key, value);
                }
            }

            Entry { id, values }
        })
    }
}
