use crossterm::event::{Event, KeyCode};
use libmoon::persona::{self, Persona};
use ratatui::{
    style::Style,
    widgets::{Block, Borders, Paragraph, StatefulWidget},
};
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::AppCommand;

pub struct SelectorState {
    personas: Vec<Persona>,
    list_state: ListState,
}

impl SelectorState {
    pub fn load() -> Self {
        let personas = persona::loader::load_chars();
        let mut list_state = ListState::default();
        if !personas.is_empty() {
            list_state.selected = Some(0)
        }
        SelectorState {
            personas,
            list_state,
        }
    }

    pub fn handle_input(&mut self, event: Event) -> AppCommand {
        if let Some(selected) = self.list_state.selected
            && let Event::Key(key) = event
        {
            match key.code {
                KeyCode::Char('j') => self.list_state.next(),
                KeyCode::Char('k') => self.list_state.previous(),
                KeyCode::Enter => {
                    return AppCommand::CharSelection(self.personas[selected].clone());
                }
                KeyCode::Esc => return AppCommand::ToggleSelection,
                _ => (),
            }
        }
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
        let builder = ListBuilder::new(|context| {
            let style = match context.is_selected {
                true => Style::new().fg(ratatui::style::Color::Red),
                false => Style::new(),
            };

            let item = Paragraph::new(state.personas[context.index].name())
                .block(Block::bordered().borders(Borders::TOP))
                .style(style);
            (item, 3)
        });
        let list = ListView::new(builder, state.personas.len())
            .scroll_axis(tui_widget_list::ScrollAxis::Vertical)
            .block(Block::bordered().title("Character selector"));
        list.render(area, buf, &mut state.list_state);
    }
}
