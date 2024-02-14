use std::time::Duration;

use crossterm::event::{poll, read, Event, KeyCode, KeyEventKind};

#[derive(Debug, Clone, Copy)]
pub enum TuiAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Next,
    Prev,
    Edit,
    Replace,
    Delete,
    Copy,
    Paste,
    ToTop,
    ToBottom,
    ToStart,
    ToEnd,
    EditStart,
    EditEnd,
    Select,
    Exit,
    AddRow,
    RemoveRow,
}

#[derive(Debug, Clone, Copy)]
pub enum EditingAction {
    InsertChar(char),
    MoveLeft,
    MoveRight,
    DeleteLeft,
    DeleteRight,
    CancelEditing,
    StopEditing,
}

pub fn key_pressed() -> Option<KeyCode> {
    if poll(Duration::from_millis(50)).ok()? {
        if let Event::Key(key) = read().ok()? {
            if key.kind == KeyEventKind::Press {
                return Some(key.code);
            }
        }
    }
    None
}

pub fn widget_action() -> Option<TuiAction> {
    match key_pressed()? {
        KeyCode::Char(c) => match c {
            'k' => Some(TuiAction::MoveUp),
            'j' => Some(TuiAction::MoveDown),
            'h' => Some(TuiAction::MoveLeft),
            'l' => Some(TuiAction::MoveRight),
            'n' => Some(TuiAction::Next),
            'N' => Some(TuiAction::Prev),
            'i' => Some(TuiAction::Edit),
            'r' => Some(TuiAction::Replace),
            'd' => Some(TuiAction::Delete),
            'y' => Some(TuiAction::Copy),
            'p' => Some(TuiAction::Paste),
            'g' => Some(TuiAction::ToTop),
            'G' => Some(TuiAction::ToBottom),
            '_' | '0' => Some(TuiAction::ToStart),
            '$' => Some(TuiAction::ToEnd),
            'I' => Some(TuiAction::EditStart),
            'A' => Some(TuiAction::EditEnd),
            '+' => Some(TuiAction::AddRow),
            '-' => Some(TuiAction::RemoveRow),
            _ => None,
        },
        KeyCode::Up => Some(TuiAction::MoveUp),
        KeyCode::Down => Some(TuiAction::MoveDown),
        KeyCode::Left => Some(TuiAction::MoveLeft),
        KeyCode::Right => Some(TuiAction::MoveRight),
        KeyCode::Enter => Some(TuiAction::Select),
        KeyCode::Esc => Some(TuiAction::Exit),
        _ => None,
    }
}

pub fn widget_editing_action() -> Option<EditingAction> {
    match key_pressed()? {
        KeyCode::Char(c) => {
            if c.is_alphanumeric() || c.is_ascii_whitespace() || ".,-'!?".contains(c) {
                Some(EditingAction::InsertChar(c))
            } else {
                None
            }
        }
        KeyCode::Left => Some(EditingAction::MoveLeft),
        KeyCode::Right => Some(EditingAction::MoveRight),
        KeyCode::Enter => Some(EditingAction::StopEditing),
        KeyCode::Esc => Some(EditingAction::CancelEditing),
        KeyCode::Backspace => Some(EditingAction::DeleteLeft),
        KeyCode::Delete => Some(EditingAction::DeleteRight),
        _ => None,
    }
}
