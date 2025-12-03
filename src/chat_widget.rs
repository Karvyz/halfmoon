use edtui::{EditorEventHandler, EditorState, EditorTheme, EditorView};
use libmoon::chat::Chat;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Layout},
    prelude::Widget,
    style::Style,
    text::Line,
    widgets::{Block, Paragraph, StatefulWidget, Wrap},
};
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::AppCommand;

enum InputMode {
    Normal,
    Editing,
}

pub struct ChatState {
    pub chat: Chat,
    input_mode: InputMode,
    list_state: ListState,
    input_state: EditorState,
}

impl ChatState {
    pub fn load() -> Self {
        let chat = Chat::load();
        let mut list_state = ListState::default();
        if !chat.get_history().is_empty() {
            list_state.selected = Some(0);
        }
        ChatState {
            chat,
            input_mode: InputMode::Normal,
            list_state,
            input_state: EditorState::default(),
        }
    }

    pub fn test(&mut self, key: KeyEvent) -> AppCommand {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('i') => {
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Esc => return AppCommand::Quit,
                _ => self.update(&key),
            },
            InputMode::Editing => {
                if self.input_state.mode == edtui::EditorMode::Normal {
                    match key.code {
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        KeyCode::Enter => {
                            let message: String = self
                                .input_state
                                .lines
                                .flatten(&Some('\n'))
                                .into_iter()
                                .collect();
                            self.chat.add_user_message(message);
                            self.input_state = EditorState::default();
                        }
                        _ => (),
                    }
                }

                let mut event_handler = EditorEventHandler::default();
                event_handler.on_key_event(key, &mut self.input_state)
            }
        }
        AppCommand::None
    }

    pub fn update(&mut self, event: &KeyEvent) {
        if let Some(s) = self.list_state.selected {
            match event.code {
                KeyCode::Char('h') => self.chat.previous(s),
                KeyCode::Char('j') => self.list_state.next(),
                KeyCode::Char('k') => self.list_state.previous(),
                KeyCode::Char('l') => self.chat.next(s),
                _ => (),
            }
        }
    }

    fn render_list(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let messages = self.chat.get_history();
        let structure = self.chat.get_history_structure();

        let builder = ListBuilder::new(|context| {
            let style = match context.is_selected {
                true => Style::new(),
                false => Style::new().bg(ratatui::style::Color::Red),
            };

            let title = Line::from(self.chat.owner_name(&messages[context.index]));
            let structure = Line::from(format!(
                "{}/{}",
                structure[context.index].0, structure[context.index].1
            ))
            .right_aligned();
            let item = Paragraph::new(messages[context.index].text.clone())
                .wrap(Wrap { trim: true })
                .block(Block::bordered().title(title).title(structure))
                .style(style);

            let main_axis_size = item.line_count(area.width - 2) as u16;
            (item, main_axis_size)
        });

        let item_count = messages.len();
        let list = ListView::new(builder, item_count)
            .scroll_axis(tui_widget_list::ScrollAxis::Vertical)
            .block(Block::new().title(self.chat.title()));

        list.render(area, buf, &mut self.list_state);
    }

    fn render_input(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let theme = match self.input_mode {
            InputMode::Normal => EditorTheme::default()
                .block(Block::bordered())
                .hide_cursor(),
            InputMode::Editing => EditorTheme::default().block(Block::bordered()),
        };

        EditorView::new(&mut self.input_state)
            .theme(theme)
            .wrap(true)
            .render(area, buf);
    }
}

#[derive(Default)]
pub struct ChatWidget {}

impl StatefulWidget for ChatWidget {
    type State = ChatState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(5)]);
        let [messages_area, input_area] = vertical.areas(area);
        state.render_list(messages_area, buf);
        state.render_input(input_area, buf);
    }
}

impl ChatWidget {}
