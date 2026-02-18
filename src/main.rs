use std::io;

use crossterm::event::EventStream;
use futures::StreamExt;
use libmoon::{moon::Moon, persona::Persona};
use ratatui::{DefaultTerminal, Frame, crossterm::event::Event};
use tokio::select;

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
    moon: Moon,
    chat_state: ChatState,
    selector_state: Option<SelectorState>,
    event_stream: EventStream,
    exit: bool,
}

impl App {
    fn new() -> Self {
        let moon = Moon::new();
        let chat_state = ChatState::new(
            moon.chat.title(),
            moon.chat.get_history(),
            moon.chat.get_history_structure(),
        );
        Self {
            moon,
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
                mu = self.moon.recv() => self.chat_state.update_status(mu, &self.moon.chat),
            };

            if self.exit {
                return Ok(());
            }
        }
    }

    fn input(&mut self, event: Event) {
        let command = match &mut self.selector_state {
            Some(selector_state) => selector_state.handle_input(event),
            None => self.chat_state.input(event, &mut self.moon.chat),
        };

        match command {
            AppCommand::ToggleSelection => {
                self.selector_state = match self.selector_state {
                    Some(_) => None,
                    None => Some(SelectorState::new(self.moon.gateway.chars.clone())),
                }
            }
            AppCommand::CharSelection(persona) => {
                self.moon.set_chars(persona);
                self.chat_state.update_list(&self.moon.chat);
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
