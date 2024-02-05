use std::iter::once;

use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::errors::KakeboError;
use crate::format::format_value;
use crate::parse::parse_value;

use super::actions::{widget_action, widget_editing_action, EditingAction, TuiAction};
use super::editor::{CurrentReference, CurrentValue, Editor};
use super::TuiWidget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TablePosition {
    Name,
    Header(usize),
    RowName(usize),
    Data { col: usize, row: usize },
}

#[derive(Debug, Default)]
pub struct TableData {
    name: String,
    col_names: Box<[String]>,
    row_names: Box<[String]>,
    data: Box<[i32]>,
}

impl TableData {
    pub fn new(
        name: String,
        col_names: &[String],
        row_names: &[String],
        data: &[i32],
    ) -> Option<Self> {
        if col_names.len() * row_names.len() != data.len() {
            return None;
        }
        Some(Self {
            name,
            col_names: col_names.into(),
            row_names: row_names.into(),
            data: data.into(),
        })
    }

    pub fn get(&self, col: usize, row: usize) -> Option<i32> {
        if col >= self.cols() || row >= self.rows() {
            return None;
        }
        Some(self.data[row * self.cols() + col])
    }

    pub fn get_mut(&mut self, col: usize, row: usize) -> Option<&mut i32> {
        if col >= self.cols() || row >= self.rows() {
            return None;
        }
        Some(&mut self.data[row * self.cols() + col])
    }

    pub fn cols(&self) -> usize {
        self.col_names.len()
    }

    pub fn rows(&self) -> usize {
        self.row_names.len()
    }
}

#[derive(Debug)]
pub struct StatefulTableBuilder {
    name: String,
    data: TableData,
    required_data_fields: Vec<(usize, usize)>,
    on_save: fn(&mut TableData),
}

impl Default for StatefulTableBuilder {
    fn default() -> Self {
        Self::new(String::default())
    }
}

impl StatefulTableBuilder {
    pub fn new(name: String) -> Self {
        Self {
            name,
            data: TableData::default(),
            required_data_fields: Vec::new(),
            on_save: |_: &mut TableData| {},
        }
    }

    pub fn table_data(mut self, data: TableData) -> Self {
        self.data = data;
        self
    }

    pub fn required_data_fields(
        mut self,
        required_fields: Vec<(usize, usize)>,
    ) -> Result<Self, KakeboError> {
        for (col, row) in &required_fields {
            if *col >= self.data.cols() || *row >= self.data.rows() {
                return Err(KakeboError::InvalidArgument(
                    "Required data field is out of table data bounds".to_string(),
                ));
            }
        }
        self.required_data_fields = required_fields;
        Ok(self)
    }

    pub fn on_save(mut self, on_save: fn(&mut TableData)) -> Self {
        self.on_save = on_save;
        self
    }

    pub fn build(self) -> StatefulTable {
        let first_required = self.required_data_fields.first();
        let pos = if let Some(&(col, row)) = first_required {
            TablePosition::Data { col, row }
        } else {
            TablePosition::Name
        };
        StatefulTable {
            name: self.name,
            data: self.data,
            pos,
            editor: Editor::default(),
            required_data_fields: self.required_data_fields,
            on_save: self.on_save,
        }
    }
}

const TABLE_COLUMN_SPACING: u16 = 2;

#[derive(Debug)]
pub struct StatefulTable {
    name: String,
    data: TableData,
    pos: TablePosition,
    editor: Editor,
    required_data_fields: Vec<(usize, usize)>,
    on_save: fn(&mut TableData),
}

impl StatefulTable {
    pub fn builder() -> StatefulTableBuilder {
        StatefulTableBuilder::default()
    }

    pub fn current(&mut self) -> Option<CurrentReference> {
        match self.pos {
            TablePosition::Name => None,
            TablePosition::Header(col) => {
                Some(CurrentReference::Str(&mut self.data.col_names[col]))
            }
            TablePosition::RowName(row) => {
                Some(CurrentReference::Str(&mut self.data.row_names[row]))
            }
            TablePosition::Data { col, row } => {
                Some(CurrentReference::Data(self.data.get_mut(col, row)?))
            }
        }
    }

    fn style_cell<'a, 'b: 'a>(&'b self, cell: Cell<'a>, cell_pos: TablePosition) -> Cell<'a> {
        let style = Style::default();
        let style = match cell_pos {
            TablePosition::Name => style.fg(Color::White),
            TablePosition::Header(_) => style.fg(Color::White),
            TablePosition::RowName(_) => style.bg(Color::Rgb(40, 40, 40)),
            TablePosition::Data { col, row } => {
                if self.required_data_fields.contains(&(col, row)) {
                    style.bg(Color::Rgb(110, 60, 40))
                } else {
                    style
                }
            }
        };
        if cell_pos == self.pos {
            if self.editor.is_editing() {
                let style = style.fg(Color::DarkGray).bg(Color::Gray);
                let cell = cell.content(self.editor.value());
                return cell.style(style);
            } else {
                return cell.style(style.add_modifier(Modifier::UNDERLINED));
            }
        }
        cell.style(style)
    }

    fn start_editing(&mut self, delete: bool) {
        if let Some(current_value) = self.current() {
            let s = if delete {
                "".to_string()
            } else {
                match current_value {
                    CurrentReference::Str(s) => s.clone(),
                    CurrentReference::Data(i) => format_value(*i),
                }
            };
            self.editor.start_editing(s);
        }
    }

    fn cancel_editing(&mut self) {
        if self.editor.is_editing() {
            self.editor.stop_editing();
        }
    }

    fn stop_editing(&mut self) {
        if self.editor.is_editing() {
            let s = self.editor.stop_editing();
            if let Some(current_value) = self.current() {
                match current_value {
                    CurrentReference::Str(val) => *val = s,
                    CurrentReference::Data(val) => match parse_value(&s) {
                        Ok(new_val) => *val = new_val,
                        Err(KakeboError::Parse(msg)) => {
                            // TODO: show error message
                        }
                        _ => unreachable!(),
                    },
                }
                (self.on_save)(&mut self.data);
            }
        }
    }

    fn above(&self, pos: TablePosition) -> TablePosition {
        match pos {
            TablePosition::Name | TablePosition::Header(_) => pos,
            TablePosition::RowName(row) => {
                if row == 0 {
                    TablePosition::Name
                } else {
                    TablePosition::RowName(row - 1)
                }
            }
            TablePosition::Data { col, row } => {
                if row == 0 {
                    TablePosition::Header(col)
                } else {
                    TablePosition::Data { col, row: row - 1 }
                }
            }
        }
    }

    fn below(&self, pos: TablePosition) -> TablePosition {
        match pos {
            TablePosition::Name => TablePosition::RowName(0),
            TablePosition::Header(col) => TablePosition::Data { col, row: 0 },
            TablePosition::RowName(row) => {
                if row + 1 == self.data.rows() {
                    pos
                } else {
                    TablePosition::RowName(row + 1)
                }
            }
            TablePosition::Data { col, row } => {
                if row + 1 == self.data.rows() {
                    pos
                } else {
                    TablePosition::Data { col, row: row + 1 }
                }
            }
        }
    }

    fn left_of(&self, pos: TablePosition) -> TablePosition {
        match pos {
            TablePosition::Name | TablePosition::RowName(_) => pos,
            TablePosition::Header(col) => {
                if col == 0 {
                    TablePosition::Name
                } else {
                    TablePosition::Header(col - 1)
                }
            }
            TablePosition::Data { col, row } => {
                if col == 0 {
                    TablePosition::RowName(row)
                } else {
                    TablePosition::Data { col: col - 1, row }
                }
            }
        }
    }

    fn right_of(&self, pos: TablePosition) -> TablePosition {
        match pos {
            TablePosition::Name => TablePosition::Header(0),
            TablePosition::Header(col) => {
                if col + 1 == self.data.cols() {
                    pos
                } else {
                    TablePosition::Header(col + 1)
                }
            }
            TablePosition::RowName(row) => TablePosition::Data { col: 0, row },
            TablePosition::Data { col, row } => {
                if col + 1 == self.data.cols() {
                    pos
                } else {
                    TablePosition::Data { col: col + 1, row }
                }
            }
        }
    }

    fn move_to_top(&mut self) {
        match self.pos {
            TablePosition::Name | TablePosition::Header(_) => {}
            TablePosition::RowName(_) => self.pos = TablePosition::Name,
            TablePosition::Data { col, .. } => self.pos = TablePosition::Header(col),
        }
    }

    fn move_to_bottom(&mut self) {
        let row = self.data.rows() - 1;
        match self.pos {
            TablePosition::Name | TablePosition::RowName(_) => {
                self.pos = TablePosition::RowName(row)
            }
            TablePosition::Header(col) | TablePosition::Data { col, .. } => {
                self.pos = TablePosition::Data { col, row }
            }
        }
    }

    fn move_to_start(&mut self) {
        match self.pos {
            TablePosition::Name | TablePosition::RowName(_) => {}
            TablePosition::Header(_) => self.pos = TablePosition::Name,
            TablePosition::Data { row, .. } => self.pos = TablePosition::RowName(row),
        }
    }

    fn move_to_end(&mut self) {
        let col = self.data.cols() - 1;
        match self.pos {
            TablePosition::Name | TablePosition::Header(_) => self.pos = TablePosition::Header(col),
            TablePosition::RowName(row) | TablePosition::Data { row, .. } => {
                self.pos = TablePosition::Data { col, row }
            }
        }
    }

    fn required_index(&self) -> Option<usize> {
        if let TablePosition::Data { col, row } = self.pos {
            self.required_data_fields
                .iter()
                .position(|&(c, r)| c == col && r == row)
        } else {
            None
        }
    }
}

impl TuiWidget for StatefulTable {
    fn handle_events(&mut self) -> Option<TuiAction> {
        if self.editor.is_editing() {
            match widget_editing_action()? {
                EditingAction::InsertChar(c) => self.editor.insert_char(c),
                EditingAction::DeleteLeft => self.editor.delete_left(),
                EditingAction::DeleteRight => self.editor.delete_right(),
                EditingAction::MoveLeft => self.editor.move_left(),
                EditingAction::MoveRight => self.editor.move_right(),
                EditingAction::CancelEditing => self.cancel_editing(),
                EditingAction::StopEditing => self.stop_editing(),
            }
        } else {
            match widget_action()? {
                TuiAction::MoveUp => self.pos = self.above(self.pos),
                TuiAction::MoveDown => self.pos = self.below(self.pos),
                TuiAction::MoveLeft => self.pos = self.left_of(self.pos),
                TuiAction::MoveRight => self.pos = self.right_of(self.pos),
                TuiAction::Edit => self.start_editing(false),
                TuiAction::Replace => self.start_editing(true),
                TuiAction::Delete => match self.current() {
                    Some(CurrentReference::Str(s)) => *s = String::default(),
                    Some(CurrentReference::Data(i)) => *i = 0,
                    None => {}
                },
                TuiAction::Select => {
                    match self.pos {
                        TablePosition::Data { .. }
                        | TablePosition::Header(_)
                        | TablePosition::RowName(_) => {
                            self.start_editing(false);
                        }
                        _ => {
                            todo!(); // TODO: implement selecting buttons
                        }
                    }
                }
                TuiAction::Copy => {
                    let value: CurrentValue = self.current()?.into();
                    self.editor.copy(value);
                }
                TuiAction::Paste => match (self.editor.paste()?, self.current()?) {
                    (CurrentValue::Str(buf), CurrentReference::Str(s)) => *s = buf,
                    (CurrentValue::Data(buf), CurrentReference::Data(s)) => *s = buf,
                    _ => {}
                },
                TuiAction::ToTop => self.move_to_top(),
                TuiAction::ToBottom => self.move_to_bottom(),
                TuiAction::ToStart => self.move_to_start(),
                TuiAction::ToEnd => self.move_to_end(),
                TuiAction::EditStart => {
                    self.move_to_start();
                    self.start_editing(false);
                }
                TuiAction::EditEnd => {
                    self.move_to_end();
                    self.start_editing(false);
                }
                TuiAction::Next => {
                    if !self.required_data_fields.is_empty() {
                        let next_index = if let Some(index) = self.required_index() {
                            (index + 1) % self.required_data_fields.len()
                        } else {
                            0
                        };
                        let (col, row) = self.required_data_fields[next_index];
                        self.pos = TablePosition::Data { col, row };
                    }
                }
                TuiAction::Prev => {
                    if !self.required_data_fields.is_empty() {
                        let prev_index = if let Some(index) = self.required_index() {
                            (index - 1 + self.required_data_fields.len())
                                % self.required_data_fields.len()
                        } else {
                            0
                        };
                        let (col, row) = self.required_data_fields[prev_index];
                        self.pos = TablePosition::Data { col, row };
                    }
                }
                TuiAction::Exit => return Some(TuiAction::Exit),
            }
        }
        None
    }

    fn render(&mut self, frame: &mut Frame) {
        let name_cell = Cell::new(self.name.clone());
        let name_cell = self.style_cell(name_cell, TablePosition::Name);
        let mut first_width = self.name.len() as u16;

        let header_cells = once(name_cell).chain(
            self.data
                .col_names
                .iter()
                .map(|name| Cell::new(name.clone()))
                .enumerate()
                .map(|(col, cell)| self.style_cell(cell, TablePosition::Header(col))),
        );
        let header = Row::new(header_cells).bg(Color::Rgb(80, 80, 80));
        let mut other_widths: Vec<u16> = self
            .data
            .col_names
            .iter()
            .map(|col_name| col_name.len() as u16)
            .collect();

        let rows: Vec<Row> = (0..self.data.rows())
            .map(|row| {
                let row_name_cell = Cell::new(self.data.row_names[row].clone());
                let row_name_cell = self.style_cell(row_name_cell, TablePosition::RowName(row));
                first_width = first_width.max(self.data.row_names[row].len() as u16);

                let cells = once(row_name_cell).chain((0..self.data.cols()).map(|col| {
                    if let Some(val) = self.data.get(col, row) {
                        let text = format_value(val);
                        other_widths[col] = other_widths[col].max(text.len() as u16);
                        let text = Line::from(text).alignment(Alignment::Right);
                        let cell = Cell::new(text);
                        self.style_cell(cell, TablePosition::Data { col, row })
                    } else {
                        unreachable!("Table index out of bounds")
                    }
                }));
                Row::new(cells)
            })
            .collect();

        let widths = once(Constraint::Length(first_width))
            .chain(other_widths.iter().map(|&w| Constraint::Length(w)));
        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.data.name.clone()),
            )
            .column_spacing(TABLE_COLUMN_SPACING);
        let area = frame.size();
        frame.render_widget(table, area);

        let cursor_col = |col: usize| {
            first_width
                + TABLE_COLUMN_SPACING
                + other_widths
                    .into_iter()
                    .map(|w| w + TABLE_COLUMN_SPACING)
                    .take(col)
                    .sum::<u16>()
        };

        let (cell_x, cell_y) = match self.pos {
            TablePosition::Name => (0, 0),
            TablePosition::Header(col) => (cursor_col(col), 0),
            TablePosition::RowName(row) => (0, row as u16 + 1),
            TablePosition::Data { col, row } => (cursor_col(col), row as u16 + 1),
        };
        if self.editor.is_editing() {
            frame.set_cursor(
                area.x + cell_x + self.editor.cursor_position() as u16 + 1,
                area.y + cell_y + 1,
            );
        }
    }
}
