use std::sync::Arc;

use crossterm::event::{Event, KeyCode};
use libmoon::persona::Persona;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Borders, Paragraph, StatefulWidget},
};
use tokio::sync::Mutex;
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{
    AppCommand,
    editor_widget::{EditorResult, EditorState, EditorUnfocused, EditorWidget},
};

pub struct SelectorState {
    personas: Arc<Mutex<Vec<Persona>>>,
    filtered_names: Vec<String>,
    list_state: ListState,

    searching: bool,
    search_bar: EditorState,
}

impl SelectorState {
    pub fn new(personas: Arc<Mutex<Vec<Persona>>>) -> Self {
        let mut list_state = ListState::default();
        let names = match personas.try_lock() {
            Ok(p) => p.iter().map(|p| p.name().to_string()).collect(),
            Err(_) => vec![],
        };
        if !names.is_empty() {
            list_state.selected = Some(0)
        }
        SelectorState {
            filtered_names: names,
            personas,
            list_state,
            searching: false,
            search_bar: EditorState::new(String::new(), true),
        }
    }

    pub fn handle_input(&mut self, event: Event) -> AppCommand {
        match self.searching {
            true => {
                match self.search_bar.input(event) {
                    EditorResult::Ok => self.searching = false,
                    EditorResult::Quit => self.searching = false,
                    _ => (),
                }
                let text = self.search_bar.text().to_lowercase().trim().to_string();
                if let Ok(personas) = self.personas.try_lock() {
                    self.filtered_names = personas
                        .iter()
                        .filter(|s| {
                            s.name().to_lowercase().contains(&text)
                                | s.system_prompt(None).to_lowercase().contains(&text)
                        })
                        .map(|p| p.name().to_string())
                        .collect();
                }
                match self.filtered_names.is_empty() {
                    true => self.list_state.selected = None,
                    false => self.list_state.selected = Some(0),
                }
            }

            false => {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char('i') => {
                            self.searching = true;
                            self.search_bar.insert_mode();
                        }
                        KeyCode::Esc => return AppCommand::ToggleSelection,
                        _ => (),
                    }

                    if let Some(selected) = self.list_state.selected {
                        match key.code {
                            KeyCode::Char('j') => self.list_state.next(),
                            KeyCode::Char('k') => self.list_state.previous(),
                            KeyCode::Enter => {
                                if let Some(p) = self.input_ok(selected) {
                                    return AppCommand::CharSelection(p);
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        };
        AppCommand::None
    }
    fn input_ok(&self, selected: usize) -> Option<Persona> {
        for p in self.personas.blocking_lock().iter() {
            if self.filtered_names[selected] == p.name() {
                return Some(p.clone());
            }
        }
        None
    }
}

#[derive(Default)]
pub struct SelectorWidget {}

impl StatefulWidget for SelectorWidget {
    type State = SelectorState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]);
        let [personas_area, filter_area] = vertical.areas(area);
        let builder = ListBuilder::new(|context| {
            let style = match context.is_selected {
                true => Style::new().fg(ratatui::style::Color::Red),
                false => Style::new(),
            };

            let item = Paragraph::new(state.filtered_names[context.index].clone())
                .block(Block::bordered().borders(Borders::TOP))
                .style(style);
            (item, 3)
        });
        let list = ListView::new(builder, state.filtered_names.len())
            .scroll_axis(tui_widget_list::ScrollAxis::Vertical)
            .block(Block::bordered().title("Character selector"));
        list.render(personas_area, buf, &mut state.list_state);
        match state.searching {
            true => EditorWidget::default().render(filter_area, buf, &mut state.search_bar),
            false => EditorUnfocused::default().render(filter_area, buf, &mut state.search_bar),
        }
    }
}
