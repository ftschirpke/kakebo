use std::io;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Frame;
use ratatui::Terminal;
use thiserror::Error;

use crate::errors::KakeboError;

use self::actions::TuiAction;

pub mod actions;
mod editor;
pub mod table;

pub fn open_widget(mut widget: impl TuiWidget) -> Result<(), KakeboError> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| widget.render(f))?;
        let action = widget.handle_events();
        if let Some(TuiAction::Exit) = action {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

pub trait TuiWidget {
    fn handle_events(&mut self) -> Option<actions::TuiAction>;
    fn perform_tui_action(&mut self, action: actions::TuiAction) -> Option<actions::TuiAction>;
    fn perform_editing_action(&mut self, action: actions::EditingAction);
    fn render(&mut self, frame: &mut Frame);
}

#[derive(Debug, Error)]
pub enum TuiErrors {
    #[error("Invalid value entered: {0}")]
    InvalidValue(String),
}
