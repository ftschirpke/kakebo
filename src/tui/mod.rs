use ratatui::Frame;
use thiserror::Error;

pub mod actions;
mod editor;
pub mod table;

pub trait TuiWidget {
    fn handle_events(&mut self) -> Option<actions::TuiAction>;
    fn render(&mut self, frame: &mut Frame);
}

#[derive(Debug, Error)]
pub enum TuiErrors {
    #[error("Invalid value entered: {0}")]
    InvalidValue(String),
}
