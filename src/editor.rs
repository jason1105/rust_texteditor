use std::cmp;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use self::{output::Output, reader::Reader};

pub mod output;
pub mod reader;

static QUIT_TIMES: u8 = 3;

pub(crate) struct Editor {
    reader: Reader,
    output: Output,
    quit_times: u8,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
            quit_times: QUIT_TIMES,
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
                } => {
                    /* add following */
                    if self.output.dirty > 0 && self.quit_times > 0 {
                        self.output.status_message.set_message(format!(
                        "WARNING!!! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                        self.quit_times
                    ));
                        self.quit_times -= 1;
                        return Ok(true);
                    }
                    /* end */
                    return Ok(false);
                }
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
                } => self.output.move_cursor(arrow_key),
                KeyEvent {
                    code: val @ (KeyCode::PageUp | KeyCode::PageDown),
                    modifiers: KeyModifiers::NONE,
                } => {
                    /* add the following */
                    if matches!(val, KeyCode::PageUp) {
                        self.output.cursor_controller.cursor_y =
                            self.output.cursor_controller.row_offset
                    } else {
                        self.output.cursor_controller.cursor_y = cmp::min(
                            self.output.win_size.1 + self.output.cursor_controller.row_offset - 1,
                            self.output.editor_rows.number_of_rows(),
                        );
                    }
                    /* end */
                    (0..self.output.win_size.1).for_each(|_| {
                        self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
                            KeyCode::Up
                        } else {
                            KeyCode::Down
                        })
                    });
                }
                KeyEvent {
                    code: KeyCode::Char('s'),
                    modifiers: KeyModifiers::CONTROL,
                } => self.output.editor_rows.save().map(|size| {
                    self.output
                        .status_message
                        .set_message(format!("{} bytes written", size));
                    self.output.dirty = 0;
                })?,
                KeyEvent {
                    code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                    modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                } => {
                    self.output.insert_char(match code {
                        KeyCode::Char(c) => c,
                        KeyCode::Tab => '\t',
                        _ => unreachable!(),
                    });
                }
                _ => return Ok(true),
            };
        }
        self.quit_times = QUIT_TIMES;
        Ok(true)
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.process_key()
    }
}
