use arboard::Clipboard;
use crossterm::event::{Event, KeyCode, KeyEvent};
use libmoon::{
    chat::{Chat, ChatUpdate},
    gateway::GatewayUpdate,
    message::Message,
    moon::MoonUpdate,
};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{
    AppCommand,
    editor_widget::{EditorResult, EditorState, EditorUnfocused, EditorWidget},
};

enum Mode {
    Normal,
    Editing(Box<EditorState>),
    Inputing,
    Quiting,
}

pub struct ChatState {
    title: String,
    history: Vec<Message>,
    structure: Vec<(usize, usize)>,
    input_mode: Mode,
    list_state: ListState,
    editor_state: EditorState,
    status: Option<&'static str>,
    borders: bool,
}

impl ChatState {
    pub fn new(title: String, history: Vec<Message>, structure: Vec<(usize, usize)>) -> Self {
        let mut list_state = ListState::default();
        if !history.is_empty() {
            list_state.selected = Some(0);
        }
        ChatState {
            title,
            history,
            structure,
            input_mode: Mode::Normal,
            list_state,
            editor_state: EditorState::default(),
            status: None,
            borders: true,
        }
    }

    pub fn update_status(&mut self, status: MoonUpdate, chat: &Chat) {
        self.status = Some(match status {
            MoonUpdate::CU(cu) => match cu {
                ChatUpdate::RequestSent => "Waiting",
                ChatUpdate::RequestOk => "OK",
                ChatUpdate::StreamUpdate => "Streaming",
                ChatUpdate::StreamFinished => "Done",
                ChatUpdate::RequestError(_) => "API Error",
            },
            MoonUpdate::GU(gu) => match gu {
                GatewayUpdate::Char => "Char loaded",
                GatewayUpdate::User => "User loaded",
            },
            MoonUpdate::Error(_) => "Moon Error",
        });
        self.history = chat.get_history();
        self.structure = chat.get_history_structure();
    }

    pub fn input(&mut self, event: Event, chat: &mut Chat) -> AppCommand {
        self.status = None;
        if let Event::Key(key) = event {
            match &mut self.input_mode {
                Mode::Normal => match key.code {
                    KeyCode::Char('i') => self.input_mode = Mode::Inputing,
                    KeyCode::Char('b') => self.borders = !self.borders,
                    KeyCode::Char('s') => return AppCommand::ToggleSelection,
                    KeyCode::Esc => self.input_mode = Mode::Quiting,
                    _ => self.update(&key, chat),
                },
                Mode::Inputing => match self.editor_state.input(event) {
                    EditorResult::Ok => self.chat_push(chat),
                    EditorResult::Quit => self.input_mode = Mode::Normal,
                    _ => (),
                },
                Mode::Editing(editor_state) => match editor_state.input(event) {
                    EditorResult::Ok => {
                        chat.add_edit(self.list_state.selected.unwrap_or(0), editor_state.text());
                        self.input_mode = Mode::Normal;
                        self.update_history(chat);
                        self.selected_to_last();
                    }
                    EditorResult::Quit => self.input_mode = Mode::Normal,
                    _ => (),
                },
                Mode::Quiting => match key.code {
                    KeyCode::Enter => return AppCommand::Quit,
                    _ => self.input_mode = Mode::Normal,
                },
            }
        }
        AppCommand::None
    }

    pub fn update(&mut self, event: &KeyEvent, chat: &mut Chat) {
        if let Some(depth) = self.list_state.selected {
            match event.code {
                KeyCode::Char('h') | KeyCode::Left => self.chat_prev(chat, depth),
                KeyCode::Char('j') | KeyCode::Down => self.list_state.next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.previous(),
                KeyCode::Char('l') | KeyCode::Right => self.chat_next(chat, depth),
                KeyCode::Char('y') => self.message_to_clipboard(chat, depth),
                KeyCode::Char('d') => self.chat_delete(chat, depth),
                KeyCode::Char('e') => {
                    let selected = self.list_state.selected.unwrap_or(0);
                    if self.history.len() > selected {
                        let edit_text = self.history[selected].text.clone();
                        self.input_mode =
                            Mode::Editing(Box::new(EditorState::new(edit_text, false)))
                    }
                }
                _ => (),
            }
        }
    }

    fn chat_push(&mut self, chat: &mut Chat) {
        chat.add_user_message(self.editor_state.text());
        self.editor_state = EditorState::default();
        self.input_mode = Mode::Normal;
        self.update_history(chat);
        self.selected_to_last();
    }

    fn chat_next(&mut self, chat: &mut Chat, depth: usize) {
        chat.next(depth);
        self.update_history(chat);
    }

    fn chat_prev(&mut self, chat: &mut Chat, depth: usize) {
        chat.previous(depth);
        self.update_history(chat);
    }

    fn chat_delete(&mut self, chat: &mut Chat, depth: usize) {
        chat.delete(depth);
        self.update_history(chat);
        self.selected_to_last();
    }

    fn update_history(&mut self, chat: &Chat) {
        self.history = chat.get_history();
        self.structure = chat.get_history_structure();
    }

    fn selected_to_last(&mut self) {
        self.list_state.selected = match self.history.len() {
            0 => None,
            x => Some(x - 1),
        }
    }

    fn message_to_clipboard(&mut self, chat: &Chat, depth: usize) {
        if let Ok(mut clipboard) = Clipboard::new() {
            let history = chat.get_history();
            match clipboard.set_text(history[depth].clean()) {
                Ok(_) => self.status = Some("Yanked"),
                Err(_) => self.status = Some("Copy error"),
            }
        }
    }

    fn render_list(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let builder = ListBuilder::new(|context| {
            let item = Self::paragraph(
                &self.history[context.index],
                self.structure[context.index],
                context.is_selected,
            );
            let main_axis_size = item.line_count(area.width - 2) as u16;
            (item, main_axis_size)
        });

        let status = Line::from(match &self.status {
            Some(status) => status,
            None => "",
        })
        .alignment(Alignment::Right);
        let item_count = self.history.len();
        let borders = match self.borders {
            true => Borders::all(),
            false => Borders::TOP,
        };
        let list = ListView::new(builder, item_count)
            .scroll_axis(tui_widget_list::ScrollAxis::Vertical)
            .infinite_scrolling(false)
            .block(
                Block::bordered()
                    .borders(borders)
                    .title(self.title.clone())
                    .title(status),
            );

        list.render(area, buf, &mut self.list_state);
    }

    fn paragraph(message: &Message, structure: (usize, usize), selected: bool) -> Paragraph<'_> {
        let style = match selected {
            true => Style::new().fg(ratatui::style::Color::Red),
            false => Style::new(),
        };

        let title = Line::from(message.owner_name.clone()).centered();
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
        match &mut state.input_mode {
            Mode::Editing(editor_state) => EditorWidget::default().render(area, buf, editor_state),
            Mode::Quiting => {
                Line::from("Quit? (Enter)")
                    .alignment(Alignment::Center)
                    .render(area, buf);
            }
            _ => {
                let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(5)]);
                let [messages_area, input_area] = vertical.areas(area);
                state.render_list(messages_area, buf);
                match state.input_mode {
                    Mode::Inputing => {
                        EditorWidget::default().render(input_area, buf, &mut state.editor_state)
                    }
                    _ => {
                        EditorUnfocused::default().render(input_area, buf, &mut state.editor_state)
                    }
                };
            }
        };
    }
}
