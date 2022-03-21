use std::io::{stdout, Write};

use crossterm::{
    cursor,
    event::KeyCode,
    execute, queue,
    terminal::{self, ClearType},
};

struct EditorContents {
    content: String,
}

impl EditorContents {
    pub fn new() -> Self {
        EditorContents {
            content: String::new(),
        }
    }

    pub fn push(&mut self, c: char) {
        self.content.push(c);
    }

    pub fn push_str(&mut self, string: &str) {
        self.content.push_str(string);
    }
}

// Write trait should be implemented on object of sink which are byte-oriented sink
impl Write for EditorContents {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                Ok(s.len())
            }
            Err(_) => Ok(0),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let ret = write!(stdout(), "{}", self.content);
        stdout().flush().expect("Error on stdout flush.");
        self.content.clear();
        ret
    }
}

struct CursorController {
    x: u16, // column
    y: u16, // row
    screen_column: u16,
    screen_row: u16,
}

impl CursorController {
    fn new((screen_column, screen_row): (u16, u16)) -> Self {
        CursorController {
            x: 0,
            y: 0,
            screen_column,
            screen_row,
        }
    }

    fn move_to(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
    }

    fn move_cursor_wsad(&mut self, char: char) -> (u16, u16) {
        match char {
            'w' => {
                self.y = self.y.saturating_sub(1);
            }
            's' => {
                if self.y != self.screen_row - 1 {
                    self.y += 1;
                }
            }
            'a' => {
                if self.x != 0 {
                    self.x -= 1;
                } else {
                    if self.y != 0 {
                        self.y -= 1;
                        self.x = self.screen_column - 1;
                    }
                }
            }
            'd' => {
                if self.x != self.screen_column - 1 {
                    self.x += 1;
                } else {
                    if self.y != self.screen_row - 1 {
                        self.x = 0;
                        self.y += 1;
                    }
                }
            }
            _ => {}
        }
        (self.x, self.y)
    }
    fn move_cursor_arrow(&mut self, arrow_key: KeyCode) -> (u16, u16) {
        match arrow_key {
            KeyCode::Up => {
                self.y = self.y.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.y != self.screen_row - 1 {
                    self.y += 1;
                }
            }
            KeyCode::Left => {
                if self.x != 0 {
                    self.x -= 1;
                } else {
                    if self.y != 0 {
                        self.y -= 1;
                        self.x = self.screen_column - 1;
                    }
                }
            }
            KeyCode::Right => {
                if self.x != self.screen_column - 1 {
                    self.x += 1;
                } else {
                    if self.y != self.screen_row - 1 {
                        self.x = 0;
                        self.y += 1;
                    }
                }
            }
            KeyCode::Home => {
                self.x = 0;
            }
            KeyCode::End => {
                self.x = self.screen_column - 1;
            }
            _ => {}
        }
        (self.x, self.y)
    }
}

struct EditorRows {
    row_contents: Vec<Box<str>>, // Box<str> and String are same, but Box<str> is more efficient and smaller.
}

impl EditorRows {
    fn new() -> Self {
        Self {
            row_contents: vec!["Hello World".into()],
        }
    }

    fn number_of_rows(&self) -> usize {
        1
    }

    fn get_row(&self) -> &str {
        &self.row_contents[0]
    }
}

/// This is a consumer.
/// Should be used to write to stdout.
/// 1. Implement Write trait
/// 2. Clear screen
/// 3. Write to stdout
/// 4. Move cursor
pub struct Output {
    pub win_size: (usize, usize),
    editor_contents: EditorContents, // #TODO 这里可以直接用 Stdout 吗?
    cursor_controller: CursorController,
    editor_rows: EditorRows,
}

impl Output {
    pub fn new() -> Self {
        Self {
            win_size: terminal::size()
                .map(|(x, y)| (x as usize, y as usize))
                .unwrap_or((0, 0)),
            editor_contents: EditorContents::new(),
            cursor_controller: CursorController::new(terminal::size().unwrap()),
            editor_rows: EditorRows::new(),
        }
    }

    pub fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    pub fn move_cursor_wsad(&mut self, char: char) {
        let (_, _) = self.cursor_controller.move_cursor_wsad(char);
    }

    pub fn move_cursor_arrow(&mut self, arrow_key: KeyCode) {
        let (_, _) = self.cursor_controller.move_cursor_arrow(arrow_key);
    }
    /* add this function */
    fn draw_rows(&mut self) {
        let screen_rows = self.win_size.1;
        let screen_columns = self.win_size.0;
        let file_rows = self.editor_rows.number_of_rows();
        let file_row = self.editor_rows.get_row();
        for i in 0..screen_rows {
            // show file content
            if i < file_rows {
                self.editor_contents.push_str(file_row);
            }
            // show '~'
            else {
                // Add name of editor and version
                if i == screen_rows / 3 {
                    let mut welcome = format!("Pound Editor --- Version {}", "0.1.0");
                    // do not need padding
                    if welcome.len() > screen_columns {
                        welcome.truncate(screen_columns)
                    }
                    // need padding
                    else if welcome.len() < screen_columns {
                        let mut padding = (screen_columns - welcome.len()) / 2;
                        if padding > 0 {
                            self.editor_contents.push('~');
                            padding += 1;
                        }
                        let padding_str = " ".repeat(padding);
                        self.editor_contents.push_str(&padding_str);
                    }
                    self.editor_contents.push_str(&welcome);
                } else {
                    self.editor_contents.push('~');
                }
            }

            // erase the rest of the line
            queue!(
                self.editor_contents,
                terminal::Clear(ClearType::UntilNewLine) // why? #TODO erase the line
            )
            .unwrap();

            // add crlf to the end of each line
            if i < screen_rows - 1 {
                self.editor_contents.push_str("\r\n");
            }
        }
    }

    /// refresh screen
    /// 1. clear screen
    /// 2. move cursor to top-left
    /// 3. draw rows
    /// 4. move cursor to top-left
    pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
        // queue is a lazy executor, first argument must be a sink
        queue!(
            self.editor_contents, // sink
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        // self.draw_rows() write something into a sink which as well be used for queue
        self.draw_rows();
        let x = self.cursor_controller.x;
        let y = self.cursor_controller.y;
        queue!(
            self.editor_contents, // sink
            cursor::MoveTo(0, 0),
            cursor::MoveTo(x, y),
            cursor::Show,
        )?;

        // flush sink
        self.editor_contents.flush()
    }
}
