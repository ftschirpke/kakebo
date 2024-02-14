use std::iter::once;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::errors::KakeboError;
use crate::format::format_value;
use crate::parse::parse_value;

use super::actions::{widget_action, widget_editing_action, EditingAction, TuiAction};
use super::editor::{CurrentReference, CurrentValue, Editor};
use super::TuiWidget;

const NUMBER_OF_BUTTONS: usize = 3;
const BUTTON_LABELS: [&str; NUMBER_OF_BUTTONS] = ["Add Row", "Delete Last Row", "Confirm"];
const BUTTON_ACTIONS: [TuiAction; NUMBER_OF_BUTTONS] =
    [TuiAction::AddRow, TuiAction::RemoveRow, TuiAction::Exit];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TablePosition {
    Name,
    RowName(usize),
    Data { col: usize, row: usize },
    TotalRow(usize),
    Button(usize),
}

#[derive(Debug, Default)]
pub struct TableData {
    name: String,
    col_names: Box<[String]>,
    row_names: Vec<String>,
    data: Vec<i32>,
    total_row: Vec<i32>,
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
        let mut total_row = vec![0; col_names.len()];
        for (i, &val) in data.iter().enumerate() {
            total_row[i % col_names.len()] += val;
        }
        Some(Self {
            name,
            col_names: col_names.into(),
            row_names: row_names.into(),
            data: data.into(),
            total_row,
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
        let idx = row * self.cols() + col;
        Some(&mut self.data[idx])
    }

    pub fn cols(&self) -> usize {
        self.col_names.len()
    }

    pub fn rows(&self) -> usize {
        self.row_names.len()
    }

    fn add_row(&mut self) {
        let new_row = vec![0; self.cols()].into_boxed_slice();
        self.row_names.push(String::default());
        self.data = self.data.iter().chain(new_row.iter()).copied().collect();
    }

    fn remove_row(&mut self, row: usize) {
        if self.rows() > 1 && row < self.rows() {
            self.row_names.remove(row);
            let row_index_range = row * self.cols()..(row + 1) * self.cols();
            self.data.drain(row_index_range);
        }
    }
}

#[derive(Debug)]
pub struct StatefulTableBuilder {
    name: String,
    data: TableData,
    fixed_rows_number: usize,
    editable_columns: Vec<usize>,
    editable_total_fields: Vec<usize>,
    on_save: fn(&mut TableData, TablePosition),
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
            fixed_rows_number: 0,
            editable_columns: Vec::new(),
            editable_total_fields: Vec::new(),
            on_save: |_: &mut TableData, _: TablePosition| {},
        }
    }

    pub fn table_data(mut self, data: TableData) -> Self {
        self.data = data;
        self
    }

    pub fn on_save(mut self, on_save: fn(&mut TableData, TablePosition)) -> Self {
        self.on_save = on_save;
        self
    }

    pub fn fixed_rows(mut self, number: usize) -> Self {
        self.fixed_rows_number = number;
        self
    }

    pub fn editable_columns(mut self, columns: impl IntoIterator<Item = usize>) -> Self {
        self.editable_columns.extend(columns);
        self
    }

    pub fn editable_column(mut self, column: usize) -> Self {
        self.editable_columns.push(column);
        self
    }

    pub fn editable_total_fields(mut self, fields: impl IntoIterator<Item = usize>) -> Self {
        self.editable_total_fields.extend(fields);
        self
    }

    pub fn editable_total_field(mut self, field: usize) -> Self {
        self.editable_total_fields.push(field);
        self
    }

    pub fn build(self) -> StatefulTable {
        let pos = TablePosition::Name;
        StatefulTable {
            name: self.name,
            data: self.data,
            pos,
            editor: Editor::default(),
            fixed_rows_number: self.fixed_rows_number,
            editable_columns: self.editable_columns,
            editable_total_fields: self.editable_total_fields,
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
    fixed_rows_number: usize,
    editable_columns: Vec<usize>,
    editable_total_fields: Vec<usize>,
    on_save: fn(&mut TableData, TablePosition),
}

impl StatefulTable {
    pub fn builder() -> StatefulTableBuilder {
        StatefulTableBuilder::default()
    }

    pub fn current(&mut self) -> Option<CurrentReference> {
        match self.pos {
            TablePosition::Name => Some(CurrentReference::Str(&mut self.data.name)),
            TablePosition::RowName(row) => {
                Some(CurrentReference::Str(&mut self.data.row_names[row]))
            }
            TablePosition::Data { col, row } => {
                Some(CurrentReference::Data(self.data.get_mut(col, row)?))
            }
            TablePosition::TotalRow(col) => {
                Some(CurrentReference::Data(self.data.total_row.get_mut(col)?))
            }
            TablePosition::Button(_) => None,
        }
    }

    fn is_editable(&self, pos: &TablePosition) -> bool {
        match *pos {
            TablePosition::Name => true,
            TablePosition::RowName(row) => row >= self.fixed_rows_number,
            TablePosition::Data { col, .. } => self.editable_columns.contains(&col),
            TablePosition::TotalRow(col) => self.editable_total_fields.contains(&col),
            TablePosition::Button(_) => false,
        }
    }

    fn style_span<'a, 'b: 'a>(&'b self, span: Span<'a>, span_pos: TablePosition) -> Span<'a> {
        let mut style = Style::default();
        style = match span_pos {
            TablePosition::RowName(_) => style.bg(Color::Rgb(40, 40, 40)),
            TablePosition::Data { .. } => {
                if self.is_editable(&span_pos) {
                    style.bg(Color::Rgb(110, 60, 40))
                } else {
                    style
                }
            }
            TablePosition::TotalRow(_) => style.bg(Color::Rgb(80, 80, 80)),
            TablePosition::Name => style,
            TablePosition::Button(_) => style.bg(Color::Rgb(80, 0, 0)),
        };
        if span_pos == self.pos {
            style = style.add_modifier(Modifier::UNDERLINED)
        }
        span.style(style)
    }

    fn span_to_line<'a, 'b: 'a>(&'b self, span: Span<'a>, span_pos: TablePosition) -> Line<'a> {
        let style = Style::default();
        let span = self.style_span(span, span_pos);
        if span_pos == self.pos && self.editor.is_editing() {
            let style = style
                .fg(Color::DarkGray)
                .bg(Color::Gray)
                .remove_modifier(Modifier::UNDERLINED);
            let span = span.content(self.editor.value());
            let span = span.style(style);
            let style = span.style;
            return Line::from(span).style(style);
        }

        let style = span.style;
        let line = Line::from(span).style(style);
        if let TablePosition::Data { .. } = span_pos {
            line.right_aligned()
        } else {
            line
        }
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
                (self.on_save)(&mut self.data, self.pos);
            }
        }
    }

    fn above(&self, pos: TablePosition) -> TablePosition {
        match pos {
            TablePosition::Name => pos,
            TablePosition::RowName(row) => {
                if row == 0 {
                    TablePosition::Name
                } else {
                    TablePosition::RowName(row - 1)
                }
            }
            TablePosition::Data { col, row } => {
                if row == 0 {
                    TablePosition::Name
                } else {
                    TablePosition::Data { col, row: row - 1 }
                }
            }
            TablePosition::TotalRow(col) => TablePosition::Data {
                col,
                row: self.data.rows() - 1,
            },
            TablePosition::Button(col) => {
                let new_col = col.min(self.data.cols() - 1);
                TablePosition::TotalRow(new_col)
            }
        }
    }

    fn below(&self, pos: TablePosition) -> TablePosition {
        match pos {
            TablePosition::Name => TablePosition::Data { col: 0, row: 0 },
            TablePosition::RowName(row) => {
                if row + 1 == self.data.rows() {
                    TablePosition::TotalRow(0)
                } else {
                    TablePosition::RowName(row + 1)
                }
            }
            TablePosition::Data { col, row } => {
                if row + 1 == self.data.rows() {
                    TablePosition::TotalRow(col)
                } else {
                    TablePosition::Data { col, row: row + 1 }
                }
            }
            TablePosition::TotalRow(col) => {
                let new_col = col.min(NUMBER_OF_BUTTONS - 1);
                TablePosition::Button(new_col)
            }
            TablePosition::Button(_) => pos,
        }
    }

    fn left_of(&self, pos: TablePosition) -> TablePosition {
        match pos {
            TablePosition::Name | TablePosition::RowName(_) => pos,
            TablePosition::Data { col, row } => {
                if col == 0 {
                    TablePosition::RowName(row)
                } else {
                    TablePosition::Data { col: col - 1, row }
                }
            }
            TablePosition::TotalRow(col) => {
                if col == 0 {
                    pos
                } else {
                    TablePosition::TotalRow(col - 1)
                }
            }
            TablePosition::Button(col) => {
                if col == 0 {
                    pos
                } else {
                    TablePosition::Button(col - 1)
                }
            }
        }
    }

    fn right_of(&self, pos: TablePosition) -> TablePosition {
        match pos {
            TablePosition::Name => pos,
            TablePosition::RowName(row) => TablePosition::Data { col: 0, row },
            TablePosition::Data { col, row } => {
                if col + 1 == self.data.cols() {
                    pos
                } else {
                    TablePosition::Data { col: col + 1, row }
                }
            }
            TablePosition::TotalRow(col) => {
                if col + 1 == self.data.cols() {
                    pos
                } else {
                    TablePosition::TotalRow(col + 1)
                }
            }
            TablePosition::Button(col) => {
                if col + 1 == NUMBER_OF_BUTTONS {
                    pos
                } else {
                    TablePosition::Button(col + 1)
                }
            }
        }
    }

    fn move_to_top(&mut self) {
        self.pos = TablePosition::Name
    }

    fn move_to_bottom(&mut self) {
        let new_col = NUMBER_OF_BUTTONS.min(self.data.cols() - 1);
        self.pos = TablePosition::Button(new_col)
    }

    fn move_to_start(&mut self) {
        match self.pos {
            TablePosition::Name | TablePosition::RowName(_) => {}
            TablePosition::Data { row, .. } => self.pos = TablePosition::RowName(row),
            TablePosition::TotalRow(_) => self.pos = TablePosition::TotalRow(0),
            TablePosition::Button(_) => self.pos = TablePosition::Button(0),
        }
    }

    fn move_to_end(&mut self) {
        let last_col = self.data.cols() - 1;
        match self.pos {
            TablePosition::Name => {}
            TablePosition::RowName(row) | TablePosition::Data { row, .. } => {
                self.pos = TablePosition::Data { col: last_col, row }
            }
            TablePosition::TotalRow(_) => self.pos = TablePosition::TotalRow(last_col),
            TablePosition::Button(_) => self.pos = TablePosition::Button(NUMBER_OF_BUTTONS - 1),
        }
    }
}

impl TuiWidget for StatefulTable {
    fn handle_events(&mut self) -> Option<TuiAction> {
        if self.editor.is_editing() {
            let editing_action = widget_editing_action()?;
            self.perform_editing_action(editing_action);
            None
        } else {
            let tui_action = widget_action()?;
            self.perform_tui_action(tui_action)
        }
    }

    fn perform_editing_action(&mut self, action: EditingAction) {
        match action {
            EditingAction::InsertChar(c) => self.editor.insert_char(c),
            EditingAction::DeleteLeft => self.editor.delete_left(),
            EditingAction::DeleteRight => self.editor.delete_right(),
            EditingAction::MoveLeft => self.editor.move_left(),
            EditingAction::MoveRight => self.editor.move_right(),
            EditingAction::CancelEditing => self.cancel_editing(),
            EditingAction::StopEditing => self.stop_editing(),
        }
    }

    fn perform_tui_action(&mut self, action: TuiAction) -> Option<TuiAction> {
        match action {
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
                if self.is_editable(&self.pos) {
                    self.start_editing(true);
                } else if let TablePosition::Button(button_num) = self.pos {
                    let action = BUTTON_ACTIONS.get(button_num)?;
                    if let TuiAction::RemoveRow = action {
                        self.data.remove_row(self.data.rows() - 1);
                    } else {
                        return self.perform_tui_action(*action);
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
                todo!()
            }
            TuiAction::Prev => {
                todo!()
            }
            TuiAction::Exit => return Some(TuiAction::Exit),
            TuiAction::AddRow => self.data.add_row(),
            TuiAction::RemoveRow => match self.pos {
                TablePosition::Data { row, col } => {
                    self.data.remove_row(row);
                    if row >= self.data.rows() {
                        self.pos = TablePosition::Data { col, row: row - 1 }
                    }
                }
                TablePosition::RowName(row) => {
                    self.data.remove_row(row);
                    if row >= self.data.rows() {
                        self.pos = TablePosition::RowName(row - 1)
                    }
                }
                TablePosition::Name | TablePosition::TotalRow(_) | TablePosition::Button(_) => {}
            },
        }
        None
    }

    fn render(&mut self, frame: &mut Frame) {
        let name_span = Span::from(self.data.name.clone());
        let name_line = self.span_to_line(name_span, TablePosition::Name);

        let table_name_cell = Cell::new(self.name.clone());
        let name_cell = table_name_cell.fg(Color::White);
        let mut first_width = self.name.len() as u16;

        let header_cells = once(name_cell).chain(
            self.data
                .col_names
                .iter()
                .map(|name| Cell::new(name.clone()).fg(Color::White)),
        );
        let header = Row::new(header_cells).bg(Color::Rgb(80, 80, 80));
        let mut other_widths: Vec<u16> = self
            .data
            .col_names
            .iter()
            .map(|col_name| col_name.len() as u16)
            .collect();

        let mut rows: Vec<Row> = (0..self.data.rows())
            .map(|row| {
                let row_name_span = Span::from(self.data.row_names[row].clone());
                let row_name_line = self.span_to_line(row_name_span, TablePosition::RowName(row));
                first_width = first_width.max(self.data.row_names[row].len() as u16);

                let lines = once(row_name_line).chain((0..self.data.cols()).map(|col| {
                    if let Some(val) = self.data.get(col, row) {
                        let text = format_value(val);
                        other_widths[col] = other_widths[col].max(text.len() as u16);
                        let span = Span::from(text);
                        self.span_to_line(span, TablePosition::Data { col, row })
                    } else {
                        Line::from("")
                    }
                }));
                Row::new(lines)
            })
            .collect();
        let last_row = once(Line::from("Total").style(Style::default().fg(Color::White))).chain(
            self.data.total_row.iter().enumerate().map(|(col, &val)| {
                let text = format_value(val);
                other_widths[col] = other_widths[col].max(text.len() as u16);
                let span = Span::from(text);
                let span = self.style_span(span, TablePosition::TotalRow(col));
                Line::from(span).right_aligned()
            }),
        );
        let last_row = Row::new(last_row).bg(Color::Rgb(80, 80, 80));
        rows.push(last_row);

        let number_of_rows = rows.len() as u16;

        if self.editor.is_editing() {
            match self.pos {
                TablePosition::Data { col, .. } | TablePosition::TotalRow(col) => {
                    other_widths[col] = other_widths[col].max(self.editor.value().len() as u16 + 2)
                }
                TablePosition::RowName(_) => {
                    first_width = first_width.max(self.editor.value().len() as u16 + 2)
                }
                TablePosition::Name | TablePosition::Button(_) => {}
            }
        }

        let widths = once(Constraint::Length(first_width))
            .chain(other_widths.iter().map(|&w| Constraint::Length(w)));
        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(TABLE_COLUMN_SPACING);

        let button_row: Vec<_> = BUTTON_LABELS
            .iter()
            .map(|s| Span::from(s.to_string()).fg(Color::White))
            .enumerate()
            .map(|(button_num, s)| self.style_span(s, TablePosition::Button(button_num)))
            .flat_map(|s| {
                once(s).chain(once(Span::from(" ".repeat(TABLE_COLUMN_SPACING as usize))))
            })
            .collect();
        let button_row = Line::from(button_row);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.data.name.clone());
        let area = block.inner(frame.size());
        let rects = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(1),
                Constraint::Length(number_of_rows + 1),
                Constraint::Length(2),
            ],
        )
        .split(area);

        frame.render_widget(name_line, rects[0]);
        frame.render_widget(table, rects[1]);
        frame.render_widget(button_row, rects[2]);

        let cursor_col = |col: usize| {
            first_width
                + TABLE_COLUMN_SPACING
                + other_widths
                    .into_iter()
                    .map(|w| w + TABLE_COLUMN_SPACING)
                    .take(col)
                    .sum::<u16>()
        };
        let cursor_row = |row: usize| row as u16 + 2;

        let (cell_x, cell_y) = match self.pos {
            TablePosition::Name => (0, 0),
            TablePosition::RowName(row) => (0, cursor_row(row)),
            TablePosition::Data { col, row } => (cursor_col(col), cursor_row(row)),
            TablePosition::TotalRow(col) => (cursor_col(col), cursor_row(self.data.rows())),
            TablePosition::Button(button_num) => (
                BUTTON_LABELS
                    .iter()
                    .take(button_num)
                    .map(|s| s.len() as u16 + 1)
                    .sum(),
                cursor_row(self.data.rows() + 1),
            ),
        };
        if self.editor.is_editing() {
            frame.set_cursor(
                area.x + cell_x + self.editor.cursor_position() as u16,
                area.y + cell_y,
            );
        }
    }
}
