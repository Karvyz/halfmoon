use crossterm::event::{Event, KeyCode};
use libmoon::persona::{self, Persona};
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Borders, Paragraph, StatefulWidget},
};
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{
    AppCommand,
    editor_widget::{EditorResult, EditorState, EditorUnfocused, EditorWidget},
};

pub struct SelectorState {
    personas: Vec<Persona>,
    filtered_personas: Vec<Persona>,
    list_state: ListState,

    searching: bool,
    search_bar: EditorState,
}

impl SelectorState {
    pub fn load() -> Self {
        let personas = persona::loader::load_chars();
        let mut list_state = ListState::default();
        if !personas.is_empty() {
            list_state.selected = Some(0)
        }
        SelectorState {
            filtered_personas: personas.clone(),
            personas,
            list_state,
            searching: false,
            search_bar: EditorState::default(),
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
                self.filtered_personas = self
                    .personas
                    .iter()
                    .filter(|s| {
                        s.name().to_lowercase().contains(&text)
                            | s.system_prompt(None).to_lowercase().contains(&text)
                    })
                    .cloned()
                    .collect();
                match self.filtered_personas.is_empty() {
                    true => self.list_state.selected = None,
                    false => self.list_state.selected = Some(0),
                }
            }

            false => {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char('s') => self.searching = true,
                        KeyCode::Esc => return AppCommand::ToggleSelection,
                        _ => (),
                    }

                    if let Some(selected) = self.list_state.selected {
                        match key.code {
                            KeyCode::Char('j') => self.list_state.next(),
                            KeyCode::Char('k') => self.list_state.previous(),
                            KeyCode::Enter => {
                                return AppCommand::CharSelection(
                                    self.filtered_personas[selected].clone(),
                                );
                            }
                            _ => (),
                        }
                    }
                }
            }
        };
        AppCommand::None
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

            let item = Paragraph::new(state.filtered_personas[context.index].name())
                .block(Block::bordered().borders(Borders::TOP))
                .style(style);
            (item, 3)
        });
        let list = ListView::new(builder, state.filtered_personas.len())
            .scroll_axis(tui_widget_list::ScrollAxis::Vertical)
            .block(Block::bordered().title("Character selector"));
        list.render(personas_area, buf, &mut state.list_state);
        match state.searching {
            true => EditorWidget::default().render(filter_area, buf, &mut state.search_bar),
            false => EditorUnfocused::default().render(filter_area, buf, &mut state.search_bar),
        }
    }
}
