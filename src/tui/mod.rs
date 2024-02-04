use thiserror::Error;

pub mod actions;
mod editor;
pub mod table;

pub trait TuiWidget {
    fn handle_events(&mut self) -> Option<actions::TuiAction>;
    // fn render(&self);
}

#[derive(Debug, Error)]
pub enum TuiErrors {
    #[error("Invalid value entered: {0}")]
    InvalidValue(String),
}
