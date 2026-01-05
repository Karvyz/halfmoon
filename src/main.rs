use std::io;

use crossterm::event::EventStream;
use futures::StreamExt;
use libmoon::{chat::ChatUpdate, persona::Persona};
use ratatui::{DefaultTerminal, Frame, crossterm::event::Event};
use tokio::{select, sync::mpsc};

use crate::{
    chat_widget::{ChatState, ChatWidget},
    selector_widget::{SelectorState, SelectorWidget},
};

mod chat_widget;
mod editor_widget;
mod selector_widget;

#[tokio::main]
async fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal).await;
    ratatui::restore();
    app_result
}

enum AppCommand {
    ToggleSelection,
    CharSelection(Persona),
    None,
    Quit,
}

struct App {
    chat_state: ChatState,
    selector_state: Option<SelectorState>,
    chat_rx: mpsc::Receiver<ChatUpdate>,
    event_stream: EventStream,
    exit: bool,
}

impl App {
    fn new() -> Self {
        let mut chat_state = ChatState::load();
        Self {
            chat_rx: chat_state.get_rx(),
            chat_state,
            selector_state: None,
            event_stream: EventStream::new(),
            exit: false,
        }
    }

    async fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            select! {
                Some(event) = self.event_stream.next() => self.input(event?),
                Some(s) = self.chat_rx.recv() => self.chat_state.update_status(s)
            };

            if self.exit {
                return Ok(());
            }
        }
    }

    fn input(&mut self, event: Event) {
        let command = match &mut self.selector_state {
            Some(selector_state) => selector_state.handle_input(event),
            None => self.chat_state.input(event),
        };

        match command {
            AppCommand::ToggleSelection => {
                self.selector_state = match self.selector_state {
                    Some(_) => None,
                    None => Some(SelectorState::load()),
                }
            }
            AppCommand::CharSelection(persona) => {
                self.chat_state.set_char(persona);
                self.selector_state = None;
            }
            AppCommand::Quit => self.exit = true,
            AppCommand::None => (),
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        match &mut self.selector_state {
            Some(selector_state) => frame.render_stateful_widget(
                SelectorWidget::default(),
                frame.area(),
                selector_state,
            ),
            None => frame.render_stateful_widget(
                ChatWidget::default(),
                frame.area(),
                &mut self.chat_state,
            ),
        }
    }
}
