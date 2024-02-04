use std::io;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use self::tui::actions::TuiAction;
use self::tui::table::{StatefulTable, StatefulTableBuilder, TableData};
use crate::tui::TuiWidget;

pub mod errors;
mod format;
mod parse;
pub mod tui;

fn create_table() -> StatefulTable {
    StatefulTableBuilder::new()
        .table_data(
            TableData::new(
                "Test Table".into(),
                &["Col 1".into(), "Col 2".into()],
                &["Row 1".into(), "Row 2".into()],
                &[1, 2, 3, 4],
            )
            .unwrap(),
        )
        .build()
}

fn main() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut stateful_table = create_table();

    loop {
        terminal.draw(|f| stateful_table.render(f))?;
        let action = stateful_table.handle_events();
        if let Some(TuiAction::Exit) = action {
            break;
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
