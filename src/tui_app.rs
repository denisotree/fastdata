use std::collections::{HashMap, HashSet};
use std::error::Error;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, Row, Table, TableState, ListState,
    },
    Terminal,
};
use crossterm::event::{self, Event, KeyCode};

use crate::virtual_table::VirtualTable;
use crate::data_loader::TableData;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

fn compare_cells(a: &str, b: &str) -> std::cmp::Ordering {
    
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(a_num), Ok(b_num)) => a_num.partial_cmp(&b_num).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.cmp(b),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, EnumIter, PartialOrd, Ord)]
pub enum AggregationFunction {
    Count,
    UniqueCount,
    Sum,
    
}

#[derive(Clone, Copy)]
pub enum ColumnWidth {
    Fixed(u16),
    Content,
}

pub struct TuiApp {
    pub table: VirtualTable,
    pub selected_row: usize,
    pub selected_column: usize,
    pub table_state: TableState,

    pub show_aggregation_popup: bool,
    pub aggregation_state: ListState, 
    pub selected_aggregations: HashMap<usize, Vec<AggregationFunction>>, 

    pub awaiting_g_key: bool,
    pub column_widths: Vec<ColumnWidth>,
    pub horizontal_offset: u16,
    pub table_area_width: u16,
}

impl TuiApp {
    pub fn new(table: VirtualTable) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let mut aggregation_state = ListState::default();
        aggregation_state.select(Some(0));

        let headers_len = table.data.headers.len();

        TuiApp {
            table,
            selected_row: 0,
            selected_column: 0,
            table_state,

            show_aggregation_popup: false,
            aggregation_state,
            selected_aggregations: HashMap::new(),

            awaiting_g_key: false,
            column_widths: vec![ColumnWidth::Fixed(15); headers_len],

            horizontal_offset: 0,
            table_area_width: 0,
        }
    }

    pub fn main_loop<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<Option<TuiApp>, Box<dyn Error>> {
        loop {
            self.draw_ui(terminal)?;

            if crossterm::event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if self.show_aggregation_popup {
                        
                        match key.code {
                            KeyCode::Up => {
                                let i = match self.aggregation_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            AggregationFunction::iter().count() - 1
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.aggregation_state.select(Some(i));
                            }
                            KeyCode::Down => {
                                let i = match self.aggregation_state.selected() {
                                    Some(i) => {
                                        if i >= AggregationFunction::iter().count() - 1 {
                                            0
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.aggregation_state.select(Some(i));
                            }
                            KeyCode::Char(' ') => {
                                
                                let index = self.aggregation_state.selected().unwrap_or(0);
                                let agg = AggregationFunction::iter().nth(index).unwrap();
                                let entry = self
                                    .selected_aggregations
                                    .entry(self.selected_column)
                                    .or_insert_with(Vec::new);
                                if entry.contains(&agg) {
                                    entry.retain(|&x| x != agg);
                                    if entry.is_empty() {
                                        self.selected_aggregations.remove(&self.selected_column);
                                    }
                                } else {
                                    entry.push(agg);
                                }
                            }
                            KeyCode::Enter | KeyCode::Char('q') => {
                                
                                self.show_aggregation_popup = false;
                            }
                            _ => {}
                        }
                    } else {
                        
                        if self.awaiting_g_key {
                            match key.code {
                                KeyCode::Char('-') => {
                                    
                                    self.selected_aggregations.clear();
                                    self.awaiting_g_key = false;
                                }
                                KeyCode::Char('_') => {
                                    
                                    for width in &mut self.column_widths {
                                        *width = match *width {
                                            ColumnWidth::Fixed(_) => ColumnWidth::Content,
                                            ColumnWidth::Content => ColumnWidth::Fixed(15),
                                        };
                                    }
                                    self.awaiting_g_key = false;
                                }
                                _ => {
                                    
                                    self.awaiting_g_key = false;
                                }
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('g') => {
                                    
                                    self.awaiting_g_key = true;
                                }
                                KeyCode::Char('_') => {
                                    
                                    if let Some(width) = self.column_widths.get_mut(self.selected_column) {
                                        *width = match *width {
                                            ColumnWidth::Fixed(_) => ColumnWidth::Content,
                                            ColumnWidth::Content => ColumnWidth::Fixed(15),
                                        };
                                    }
                                }
                                KeyCode::Up => {
                                    if self.selected_row > 0 {
                                        self.selected_row -= 1;
                                    }
                                    self.table_state.select(Some(self.selected_row));
                                }
                                KeyCode::Down => {
                                    let num_rows = if self.table.data.columns.is_empty() {
                                        0
                                    } else {
                                        self.table.data.columns[0].len()
                                    };
                                    if self.selected_row < num_rows - 1 {
                                        self.selected_row += 1;
                                    }
                                    self.table_state.select(Some(self.selected_row));
                                }
                                KeyCode::Left => {
                                    if self.selected_column > 0 {
                                        self.selected_column -= 1;
                                        self.adjust_horizontal_offset();
                                    }
                                }
                                KeyCode::Right => {
                                    if self.selected_column < self.table.data.headers.len() - 1 {
                                        self.selected_column += 1;
                                        self.adjust_horizontal_offset();
                                    }
                                }
                                KeyCode::Char('[') => {
                                    self.sort_table(true); 
                                }
                                KeyCode::Char(']') => {
                                    self.sort_table(false); 
                                }
                                KeyCode::Char(' ') => {
                                    self.show_aggregation_popup = true;
                                    self.aggregation_state.select(Some(0));
                                }
                                KeyCode::Enter => {
                                    let new_app = self.open_detail_view();
                                    return Ok(Some(new_app));
                                }
                                KeyCode::Char('q') => {
                                    return Ok(None);
                                }
                                _ => {}
                            }
                        }
                    }
                } else {

                    self.awaiting_g_key = false;
                }
            }
        }
    }

    fn adjust_horizontal_offset(&mut self) {

        let mut col_start = 0;
        for i in 0..self.selected_column {
            col_start += self.get_column_width(i) + 1;
        }
    

        let selected_col_width = self.get_column_width(self.selected_column);
    

        let visible_width = self.table_area_width.saturating_sub(2);
    

        if col_start < self.horizontal_offset {
            self.horizontal_offset = col_start;
        }

        else if col_start + selected_col_width > self.horizontal_offset + visible_width {
            self.horizontal_offset = col_start + selected_col_width - visible_width;
        }
    }

    fn get_column_width(&self, index: usize) -> u16 {
        match self.column_widths[index] {
            ColumnWidth::Fixed(w) => w,
            ColumnWidth::Content => {
                let max_content_width = self.table.data.columns[index]
                    .iter()
                    .map(|cell| cell.len() as u16)
                    .max()
                    .unwrap_or(10)
                    + 2;
                max_content_width
            }
        }
    }


    fn draw_ui<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Box<dyn Error>> {
        terminal.draw(|f| {
            let size = f.area();

            f.render_widget(Block::default(), size);


            let show_aggregation_block = !self.selected_aggregations.is_empty();
            let agg_results = if show_aggregation_block {
                Some(self.calculate_aggregations())
            } else {
                None
            };


            let agg_table_height = if let Some(agg_results) = &agg_results {

                let num_rows = agg_results.len() as u16;
                3 + num_rows
            } else {
                0
            };

            let constraints = if show_aggregation_block {
                vec![Constraint::Min(0), Constraint::Length(agg_table_height)]
            } else {
                vec![Constraint::Percentage(100)]
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(size);

            self.table_area_width = chunks[0].width;


            {
                let header_cells = self.table.data.headers.iter().enumerate().map(|(i, h)| {
                    let style = if i == self.selected_column {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                            .bg(Color::Blue)
                    } else {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                    };
                    Cell::from(h.clone()).style(style)
                });

                let header = Row::new(header_cells).height(1).bottom_margin(0);

                let num_rows = if self.table.data.columns.is_empty() {
                    0
                } else {
                    self.table.data.columns[0].len()
                };

                let rows = (0..num_rows).map(|row_idx| {
                    let cells = self.table.data.columns.iter().enumerate().map(|(col_idx, col)| {
                        let mut cell = Cell::from(col[row_idx].clone());
                        if row_idx == self.selected_row && col_idx == self.selected_column {
                            cell = cell.style(Style::default().bg(Color::LightBlue));
                        }
                        cell
                    });
                    Row::new(cells).height(1).bottom_margin(0)
                });

                let widths = self
                    .column_widths
                    .iter()
                    .enumerate()
                    .map(|(i, width)| match width {
                        ColumnWidth::Fixed(w) => Constraint::Length(*w),
                        ColumnWidth::Content => {
                            let max_content_width = self.table.data.columns[i]
                                .iter()
                                .map(|cell| cell.len() as u16)
                                .max()
                                .unwrap_or(10)
                                + 2;
                            Constraint::Length(max_content_width)
                        }
                    })
                    .collect::<Vec<_>>();

                let table = Table::new(rows, &widths)
                    .header(header)
                    .block(Block::default().borders(Borders::ALL).title("Table"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("-> ")
                    .column_spacing(2);

                f.render_stateful_widget(table, chunks[0], &mut self.table_state);
            }

            if let Some(agg_results) = &agg_results {
                
                let mut all_aggs = HashSet::new();
                for aggs in self.selected_aggregations.values() {
                    for &agg in aggs {
                        all_aggs.insert(agg);
                    }
                }
                let mut all_aggs_vec: Vec<_> = all_aggs.into_iter().collect();
                all_aggs_vec.sort();

                let mut header_cells = vec![
                    Cell::from("Column").style(Style::default().add_modifier(Modifier::BOLD)),
                ];
                for agg in &all_aggs_vec {
                    header_cells.push(
                        Cell::from(format!("{:?}", agg))
                            .style(Style::default().add_modifier(Modifier::BOLD)),
                    );
                }
                let header = Row::new(header_cells).height(1).bottom_margin(0);

                let mut rows = Vec::new();

                let mut col_indices: Vec<_> = agg_results.keys().cloned().collect();
                col_indices.sort();

                for &col_idx in &col_indices {
                    let col_aggs = &agg_results[&col_idx];
                    let mut cells = vec![Cell::from(self.table.data.headers[col_idx].clone())];
                    for agg in &all_aggs_vec {
                        if let Some(result_option) = col_aggs.get(agg) {
                            if let Some(result) = result_option {
                                cells.push(Cell::from(result.clone()));
                            } else {
                                cells.push(Cell::from("-"));
                            }
                        } else {
                            
                            cells.push(Cell::from("-"));
                        }
                    }
                    rows.push(Row::new(cells).height(1).bottom_margin(0));
                }

                if !rows.is_empty() {
                    
                    let mut widths = Vec::new();
                    widths.push(Constraint::Length(15)); 
                    for _ in &all_aggs_vec {
                        widths.push(Constraint::Length(15));
                    }

                    
                    let agg_table = Table::new(rows, &widths)
                        .header(header)
                        .block(Block::default().borders(Borders::ALL).title("Aggregations"))
                        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                        .column_spacing(1);

                    
                    f.render_widget(agg_table, chunks[1]);
                }
            }

            if self.show_aggregation_popup {
                
                let popup_area = Self::centered_rect(60, 40, size);

                
                let block = Block::default()
                    .title("Select aggregation functions (q to quit)")
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::Black));

                
                let inner_area = block.inner(popup_area);

                
                f.render_widget(Clear, popup_area);

                
                f.render_widget(block, popup_area);

                
                let list_height = AggregationFunction::iter().count() as u16;

                
                let available_height = inner_area.height;

                
                let top_padding = (available_height.saturating_sub(list_height)) / 2;

                
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(top_padding),
                        Constraint::Length(list_height),
                        Constraint::Min(0),
                    ])
                    .split(inner_area);

                
                let items: Vec<ListItem> = AggregationFunction::iter()
                    .map(|agg| {
                        let is_selected = self
                            .selected_aggregations
                            .get(&self.selected_column)
                            .map_or(false, |v| v.contains(&agg));
                        let checkbox = if is_selected { "[x]" } else { "[ ]" };
                        let content = format!("{} {:?}", checkbox, agg);
                        ListItem::new(content)
                    })
                    .collect();

                let list = List::new(items)
                    .highlight_style(Style::default().fg(Color::Yellow).bg(Color::Blue))
                    .highlight_symbol(">> ");

                
                f.render_stateful_widget(list, layout[1], &mut self.aggregation_state);
            }
        })?;
        Ok(())
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Percentage(percent_y),
                    Constraint::Percentage((100 - percent_y) / 2),
                ]
                .as_ref(),
            )
            .split(r);
        let vertical_chunk = popup_layout[1];
        let horizontal_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                .as_ref(),
            )
            .split(vertical_chunk);
        horizontal_layout[1]
    }

    fn calculate_aggregations(
        &self,
    ) -> HashMap<usize, HashMap<AggregationFunction, Option<String>>> {
        let mut results = HashMap::new();

        for (&col_idx, aggs) in &self.selected_aggregations {
            let column_data = &self.table.data.columns[col_idx];
            let mut agg_results = HashMap::new();

            for &agg in aggs {
                let result = match agg {
                    AggregationFunction::Sum => {
                        
                        let parsed_data: Vec<f64> = column_data
                            .iter()
                            .filter_map(|v| v.parse::<f64>().ok())
                            .collect();

                        if parsed_data.len() == column_data.len() && !parsed_data.is_empty() {
                            
                            let sum: f64 = parsed_data.iter().sum();
                            Some(sum.to_string())
                        } else {
                            
                            None
                        }
                    }
                    AggregationFunction::Count => {
                        Some(column_data.len().to_string())
                    }
                    AggregationFunction::UniqueCount => {
                        let unique_count = column_data.iter().collect::<HashSet<_>>().len();
                        Some(unique_count.to_string())
                    }
                    
                };
                agg_results.insert(agg, result);
            }

            if !agg_results.is_empty() {
                results.insert(col_idx, agg_results);
            }
        }

        results
    }

    fn open_detail_view(&self) -> TuiApp {
        let selected_row = self.selected_row;
        let field_column = self.table.data.headers.clone();
        let value_column: Vec<String> = self
            .table
            .data
            .columns
            .iter()
            .map(|col| col[selected_row].clone())
            .collect();

        let detail_data = TableData::new(
            vec!["Field".to_string(), "Value".to_string()],
            vec![field_column, value_column],
        );

        let detail_table = VirtualTable::new(detail_data);
        TuiApp::new(detail_table)
    }

    fn sort_table(&mut self, ascending: bool) {
        let col_idx = self.selected_column;
        let num_rows = if self.table.data.columns.is_empty() {
            0
        } else {
            self.table.data.columns[0].len()
        };

        let mut indices: Vec<usize> = (0..num_rows).collect();

        indices.sort_by(|&i, &j| {
            let a = &self.table.data.columns[col_idx][i];
            let b = &self.table.data.columns[col_idx][j];
            let ord = compare_cells(a, b);
            if ascending {
                ord
            } else {
                ord.reverse()
            }
        });

        
        for col in self.table.data.columns.iter_mut() {
            let reordered_col: Vec<String> = indices.iter().map(|&i| col[i].clone()).collect();
            *col = reordered_col;
        }

        
        self.selected_row = 0;
        self.table_state.select(Some(self.selected_row));
    }
}