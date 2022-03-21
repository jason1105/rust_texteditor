use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use self::{output::Output, reader::Reader};

pub mod output;
pub mod reader;

pub(crate) struct Editor {
    reader: Reader,
    output: Output,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
        }
    }

    /// This is a processor
    fn process_key(&mut self) -> crossterm::Result<bool> {
        // get key
        if let Ok(key_event) = self.reader.read_key() {
            match key_event {
                KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::CONTROL,
                } => return Ok(false),
                KeyEvent {
                    code:
                        arrow_key
                        @
                        (KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Home
                        | KeyCode::End),
                    modifiers: KeyModifiers::NONE,
                } => {
                    self.output.move_cursor_arrow(arrow_key);
                    return Ok(true);
                }
                KeyEvent {
                    code: key @ (KeyCode::PageUp | KeyCode::PageDown),
                    modifiers: KeyModifiers::NONE,
                } => {
                    (0..self.output.win_size.1).for_each(|_| {
                        self.output
                            .move_cursor_arrow(if matches!(key, KeyCode::PageUp) {
                                KeyCode::Up
                            } else {
                                KeyCode::Down
                            })
                    });

                    return Ok(true);
                }

                _ => return Ok(true),
            };
        }

        Ok(true)
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.process_key()
    }
}
