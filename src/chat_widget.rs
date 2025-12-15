use crossterm::event::{Event, KeyCode, KeyEvent};
use libmoon::{
    chat::{Chat, ChatUpdate},
    persona::Persona,
};
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Wrap},
};
use tokio::sync::mpsc;
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{
    AppCommand,
    editor_widget::{EditorState, EditorWidget},
};

enum InputMode {
    Normal,
    Editing,
}

pub struct ChatState {
    chat: Chat,
    input_mode: InputMode,
    list_state: ListState,
    editor_state: EditorState,
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
            editor_state: EditorState::default(),
        }
    }

    pub fn set_char(&mut self, char: Persona) {
        let user = self.chat.user();
        let settings = self.chat.settings();
        self.chat = Chat::with_personas(user, char, settings.clone())
    }

    pub fn get_rx(&mut self) -> mpsc::Receiver<ChatUpdate> {
        self.chat.get_rx()
    }

    pub fn input(&mut self, event: Event) -> AppCommand {
        if let Event::Key(key) = event {
            match self.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Enter => {
                        self.chat.add_user_message(self.editor_state.text());
                        self.editor_state = EditorState::default();
                    }
                    KeyCode::Char('i') => self.input_mode = InputMode::Editing,
                    KeyCode::Char('s') => return AppCommand::ToggleSelection,
                    KeyCode::Esc => return AppCommand::Quit,
                    _ => self.update(&key),
                },
                InputMode::Editing => {
                    if self.editor_state.input(event) {
                        self.input_mode = InputMode::Normal;
                    }
                }
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
                true => Style::new().fg(ratatui::style::Color::Red),
                false => Style::new(),
            };

            let title = Line::from(self.chat.owner_name(&messages[context.index])).centered();
            let structure = Line::from(format!(
                "{}/{}",
                structure[context.index].0, structure[context.index].1
            ))
            .right_aligned();
            let item = Paragraph::new(messages[context.index].text.clone())
                .wrap(Wrap { trim: true })
                .block(
                    Block::bordered()
                        .borders(Borders::TOP)
                        .border_style(style)
                        .title(title)
                        .title(structure),
                );

            let main_axis_size = item.line_count(area.width - 2) as u16;
            (item, main_axis_size)
        });

        let item_count = messages.len();
        let list = ListView::new(builder, item_count)
            .scroll_axis(tui_widget_list::ScrollAxis::Vertical)
            .block(Block::new().title(self.chat.title()));

        list.render(area, buf, &mut self.list_state);
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
        EditorWidget::default().render(input_area, buf, &mut state.editor_state);
    }
}
