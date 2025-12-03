use std::{io, time::Duration};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event},
};

use crate::chat_widget::{ChatState, ChatWidget};

mod chat_widget;

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}

enum AppCommand {
    None,
    Quit,
}

/// App holds the state of the application
struct App {
    chat_state: ChatState,
}

impl App {
    fn new() -> Self {
        Self {
            chat_state: ChatState::load(),
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            if event::poll(Duration::from_millis(16))?
                && let Event::Key(key) = event::read()?
            {
                match self.chat_state.test(key) {
                    AppCommand::Quit => return Ok(()),
                    AppCommand::None => (),
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let chat_widget = ChatWidget::default();
        frame.render_stateful_widget(chat_widget, frame.area(), &mut self.chat_state);
    }
}
