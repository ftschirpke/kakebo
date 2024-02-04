use std::fmt::{self, Display, Formatter};
use std::iter::once;

use crate::format::format_value;

#[derive(Debug)]
pub enum CurrentReference<'a> {
    Str(&'a mut String),
    Data(&'a mut i32),
}

impl Display for CurrentReference<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CurrentReference::Str(s) => write!(f, "{}", s),
            CurrentReference::Data(d) => write!(f, "{}", format_value(**d)),
        }
    }
}

#[derive(Debug)]
pub enum CurrentValue {
    Str(String),
    Data(i32),
}

impl Display for CurrentValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CurrentValue::Str(s) => write!(f, "{}", s),
            CurrentValue::Data(d) => write!(f, "{}", format_value(*d)),
        }
    }
}

impl From<CurrentReference<'_>> for CurrentValue {
    fn from(r: CurrentReference) -> Self {
        match r {
            CurrentReference::Str(s) => Self::Str(s.clone()),
            CurrentReference::Data(d) => Self::Data(*d),
        }
    }
}

#[derive(Debug, Default)]
pub struct Editor {
    cursor_position: usize,
    s: Option<String>,
    copy_buffer: Option<CurrentValue>,
}

impl Editor {
    pub fn is_editing(&self) -> bool {
        self.s.is_some()
    }

    pub fn start_editing(&mut self, s: String) {
        self.cursor_position = s.len();
        self.s = Some(s);
    }

    pub fn stop_editing(&mut self) -> String {
        self.s.take().unwrap_or_default()
    }

    pub fn copy(&mut self, r: impl Into<CurrentValue>) {
        self.copy_buffer = Some(r.into());
    }

    pub fn paste(&mut self) -> Option<CurrentValue> {
        self.copy_buffer.take()
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some(s) = &mut self.s {
            let before = s.chars().take(self.cursor_position);
            let after = s.chars().skip(self.cursor_position);
            self.s = Some(before.chain(once(c)).chain(after).collect());
            self.cursor_position += 1;
        }
    }

    pub fn delete_left(&mut self) {
        if let Some(s) = &mut self.s {
            if self.cursor_position > 0 {
                let before = s.chars().take(self.cursor_position - 1);
                let after = s.chars().skip(self.cursor_position);
                self.s = Some(before.chain(after).collect());
                self.cursor_position -= 1;
            }
        }
    }

    pub fn delete_right(&mut self) {
        if let Some(s) = &mut self.s {
            if self.cursor_position < s.len() {
                let before = s.chars().take(self.cursor_position);
                let after = s.chars().skip(self.cursor_position + 1);
                self.s = Some(before.chain(after).collect());
            }
        }
    }

    pub fn move_left(&mut self) {
        if self.s.is_some() && self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if let Some(s) = &mut self.s {
            if self.cursor_position < s.len() {
                self.cursor_position += 1;
            }
        }
    }

    pub fn value(&self) -> &str {
        self.s.as_deref().unwrap_or_default()
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }
}
