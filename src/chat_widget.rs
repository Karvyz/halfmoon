use crossterm::event::{Event, KeyCode, KeyEvent};
use libmoon::{
    chat::{Chat, ChatUpdate},
    message::Message,
    persona::Persona,
};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Wrap},
};
use tokio::sync::mpsc;
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{
    AppCommand,
    editor_widget::{EditorResult, EditorState, EditorUnfocused, EditorWidget},
};

enum InputMode {
    Normal,
    Editing(Box<EditorState>),
    Inputing,
}

pub struct ChatState {
    chat: Chat,
    input_mode: InputMode,
    list_state: ListState,
    editor_state: EditorState,
    status: Option<ChatUpdate>,
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
            status: None,
        }
    }

    pub fn set_char(&mut self, char: Persona) {
        let user = self.chat.user();
        let settings = self.chat.settings();
        let tx = self.chat.tx.clone();
        self.chat = Chat::with_personas(user, char, settings.clone());
        self.chat.tx = tx;
    }

    pub fn get_rx(&mut self) -> mpsc::Receiver<ChatUpdate> {
        self.chat.get_rx()
    }

    pub fn update_status(&mut self, status: ChatUpdate) {
        self.status = Some(status)
    }

    pub fn input(&mut self, event: Event) -> AppCommand {
        if let Event::Key(key) = event {
            match &mut self.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('i') => self.input_mode = InputMode::Inputing,
                    KeyCode::Char('s') => return AppCommand::ToggleSelection,
                    KeyCode::Esc => return AppCommand::Quit,
                    _ => self.update(&key),
                },
                InputMode::Inputing => match self.editor_state.input(event) {
                    EditorResult::Ok => {
                        self.chat.add_user_message(self.editor_state.text());
                        self.editor_state = EditorState::default();
                        self.input_mode = InputMode::Normal;
                    }
                    EditorResult::Quit => self.input_mode = InputMode::Normal,
                    _ => (),
                },
                InputMode::Editing(editor_state) => match editor_state.input(event) {
                    EditorResult::Ok => {
                        self.chat
                            .add_edit(self.list_state.selected.unwrap_or(0), editor_state.text());
                        self.input_mode = InputMode::Normal;
                    }
                    EditorResult::Quit => self.input_mode = InputMode::Normal,
                    _ => (),
                },
            }
        }
        AppCommand::None
    }

    pub fn update(&mut self, event: &KeyEvent) {
        if let Some(s) = self.list_state.selected {
            match event.code {
                KeyCode::Char('h') | KeyCode::Left => self.chat.previous(s),
                KeyCode::Char('j') | KeyCode::Down => self.list_state.next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.previous(),
                KeyCode::Char('l') | KeyCode::Right => self.chat.next(s),
                KeyCode::Char('d') => self.chat.delete(s),
                KeyCode::Char('e') => {
                    let history = self.chat.get_history();
                    let selected = self.list_state.selected.unwrap_or(0);
                    if history.len() > selected {
                        let edit_text = history[selected].text.clone();
                        self.input_mode = InputMode::Editing(Box::new(EditorState::new(edit_text)))
                    }
                }
                _ => (),
            }
        }
    }

    fn render_list(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let messages = self.chat.get_history();
        let structure = self.chat.get_history_structure();

        let builder = ListBuilder::new(|context| {
            let item = Self::paragraph(
                &messages[context.index],
                self.chat.owner_name(&messages[context.index]).to_string(),
                structure[context.index],
                context.is_selected,
            );
            let main_axis_size = item.line_count(area.width - 2) as u16;
            (item, main_axis_size)
        });

        let status = Line::from(match &self.status {
            Some(status) => match status {
                ChatUpdate::RequestSent => "Waiting",
                ChatUpdate::RequestOk => "OK",
                ChatUpdate::StreamUpdate => "Streaming",
                ChatUpdate::StreamFinished => "Done",
                ChatUpdate::RequestError(_) => "Error",
            },
            None => "",
        })
        .alignment(Alignment::Right);
        let item_count = messages.len();
        let list = ListView::new(builder, item_count)
            .scroll_axis(tui_widget_list::ScrollAxis::Vertical)
            .infinite_scrolling(false)
            .block(Block::bordered().title(self.chat.title()).title(status));

        list.render(area, buf, &mut self.list_state);
    }

    fn paragraph(
        message: &Message,
        owner: String,
        structure: (usize, usize),
        selected: bool,
    ) -> Paragraph<'_> {
        let style = match selected {
            true => Style::new().fg(ratatui::style::Color::Red),
            false => Style::new(),
        };

        let title = Line::from(owner).centered();
        let structure = Line::from(format!("{}/{}", structure.0, structure.1)).right_aligned();
        let mut lines = vec![];
        for l in message.spans() {
            let spans: Vec<Span> = l
                .into_iter()
                .map(|(text, style)| Span::from(text).style(Self::color(style)))
                .collect();
            lines.push(Line::from(spans));
            lines.push(Line::from(""));
        }
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .borders(Borders::TOP)
                .border_style(style)
                .title(title)
                .title(structure),
        )
    }

    fn color(style: libmoon::message::Style) -> Style {
        let color = match style {
            libmoon::message::Style::Normal => Color::White,
            libmoon::message::Style::Strong => Color::Blue,
            libmoon::message::Style::Quote => Color::Red,
            libmoon::message::Style::StrongQuote => Color::Green,
        };
        Style::default().fg(color)
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
        if let InputMode::Editing(e) = &mut state.input_mode {
            EditorWidget::default().render(area, buf, e)
        } else {
            let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(5)]);
            let [messages_area, input_area] = vertical.areas(area);
            state.render_list(messages_area, buf);
            match state.input_mode {
                InputMode::Inputing => {
                    EditorWidget::default().render(input_area, buf, &mut state.editor_state)
                }
                _ => EditorUnfocused::default().render(input_area, buf, &mut state.editor_state),
            };
        }
    }
}
