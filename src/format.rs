use core::fmt;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Part<'a, 'b> {
    Plain(&'a str),
    Argument(&'b str),
}

pub struct Format<'a, 'b> {
    input: &'a str,
    vars: &'b HashMap<String, String>,
    consumed: usize,
}

#[derive(Debug)]
pub enum FormatError<'a> {
    MissingValue(&'a str),
    MissingClosingBracket(usize),
    BracketEscapeInvalid(usize),
}

impl fmt::Display for FormatError<'_> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue(name) => fmt.write_fmt(format_args!("env:{name}: missing value")),
            Self::MissingClosingBracket(idx) => fmt.write_fmt(format_args!("Missing bracket at position {idx}")),
            Self::BracketEscapeInvalid(idx) => fmt.write_fmt(format_args!("Unsupported bracket escape at position {idx}")),
        }
    }
}

impl<'a, 'b> Format<'a, 'b> {
    pub fn new(input: &'a str, vars: &'b HashMap<String, String>) -> Self {
        Self {
            input,
            vars,
            consumed: 0
        }
    }

    pub fn next(&mut self) -> Option<Result<Part<'a, 'b>, FormatError>> {
        const ARG_START: char = '{';
        const ARG_END: char = '}';

        if self.input.is_empty() {
            return None;
        }

        if self.input.as_bytes()[0] == ARG_START as u8 {
            //double brackets not allowed
            if self.input.as_bytes()[1] == ARG_START as u8 {
                return Some(Err(FormatError::BracketEscapeInvalid(self.consumed + 1)))
            };

            if let Some(idx) = self.input.find(ARG_END) {
                let key = &self.input[1..idx];
                if let Some(value) = self.vars.get(key) {
                    let new_input = &self.input[idx+1..];

                    if let Some(true) = new_input.as_bytes().get(0).map(|byt| *byt == ARG_END as u8) {
                        return Some(Err(FormatError::BracketEscapeInvalid(self.consumed + key.len() + 1)))
                    }

                    self.input = new_input;
                    self.consumed = self.consumed.saturating_add(key.len() + 2);
                    Some(Ok(Part::Argument(value.as_str())))
                } else {
                    Some(Err(FormatError::MissingValue(key)))
                }
            } else {
                Some(Err(FormatError::MissingClosingBracket(self.consumed)))
            }
        } else if let Some(idx) = self.input.find(ARG_START) {
            let result = &self.input[..idx];
            self.consumed = self.consumed.saturating_add(result.len());
            self.input = &self.input[result.len()..];
            Some(Ok(Part::Plain(result)))
        } else {
            let result = Part::Plain(self.input);
            self.consumed = self.consumed.saturating_add(self.input.len());
            self.input = &self.input[self.input.len()..];
            Some(Ok(result))
        }
    }
}
