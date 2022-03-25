use std::{
    cmp, env, fs,
    io::{stdout, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::KeyCode,
    execute, queue, style,
    terminal::{self, ClearType},
};

use self::cursor_controller::CursorController;

pub mod cursor_controller;

static VERSION: &str = "0.1.0";
static TAB_STOP: usize = 8;

struct Row {
    row_content: Box<str>,
    render: String,
}

impl Row {
    fn new(row_content: Box<str>, render: String) -> Self {
        Self {
            row_content,
            render,
        }
    }

    fn len(&self) -> usize {
        self.row_content.len()
    }
}

struct StatusMessage {
    message: Option<String>,
    set_time: Option<Instant>,
}

impl StatusMessage {
    fn new(message: String) -> Self {
        Self {
            message: Some(message),
            set_time: Some(Instant::now()),
        }
    }

    fn set_message(&mut self, message: String) {
        self.message = Some(message);
        self.set_time = Some(Instant::now());
    }

    fn message(&mut self) -> Option<&String> {
        self.set_time.and_then(|time| {
            if time.elapsed() > Duration::from_secs(5) {
                self.message = None;
                self.set_time = None;
                None
            } else {
                self.message.as_ref()
            }
        })
    }
}

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

pub struct EditorRows {
    row_contents: Vec<Row>, // Box<str> and String are same, but Box<str> is more efficient and smaller.
    filename: Option<PathBuf>, //add field
}

impl EditorRows {
    fn new() -> Self {
        match env::args().nth(1) {
            None => Self {
                row_contents: Vec::new(),
                filename: None,
            },
            Some(file) => Self::from_file(file.into()),
        }
    }

    fn from_file(file: PathBuf) -> Self {
        let file_contents = fs::read_to_string(&file).expect("Unable to read file");
        /* modify */
        Self {
            row_contents: file_contents
                .lines()
                .map(|it| {
                    let mut row = Row::new(it.into(), String::new());
                    Self::render_row(&mut row);
                    row
                })
                .collect(),
            filename: Some(file),
        }
        /* end */
    }

    /* add functions*/
    fn get_render(&self, at: usize) -> &String {
        &self.row_contents[at].render
    }

    fn render_row(row: &mut Row) {
        let mut index = 0;
        let capacity = row
            .row_content
            .chars()
            //modify
            .fold(0, |acc, next| acc + if next == '\t' { TAB_STOP } else { 1 });
        row.render = String::with_capacity(capacity);
        row.row_content.chars().for_each(|c| {
            index += 1;
            if c == '\t' {
                row.render.push(' ');
                while index % TAB_STOP != 0 {
                    // modify
                    row.render.push(' ');
                    index += 1
                }
            } else {
                row.render.push(c);
            }
        });
    }

    pub fn number_of_rows(&self) -> usize {
        self.row_contents.len() /* modify */
    }

    fn get_editor_row(&self, at: usize) -> &Row {
        &self.row_contents[at] /* modify */
    }
}

/// This is a consumer.
/// Should be used to write to stdout.
/// 1. Implement Write trait
/// 2. Clear screen
/// 3. Write to stdout
/// 4. Move cursor
pub(crate) struct Output {
    pub win_size: (usize, usize),
    editor_contents: EditorContents, // #TODO 这里可以直接用 Stdout 吗?
    pub cursor_controller: CursorController,
    pub editor_rows: EditorRows,
    status_message: StatusMessage,
}

impl Output {
    pub fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize - 2))
            .unwrap();
        Self {
            win_size,
            editor_contents: EditorContents::new(),
            cursor_controller: CursorController::new(win_size),
            editor_rows: EditorRows::new(),
            status_message: StatusMessage::new("Press Ctrl + Q to quit".into()),
        }
    }

    pub fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    pub fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller
            .move_cursor(direction, &self.editor_rows); // modify
    }

    /* add this function */
    fn draw_rows(&mut self) {
        let screen_rows = self.win_size.1;
        let screen_columns = self.win_size.0;
        for i in 0..screen_rows {
            let file_row = i + self.cursor_controller.row_offset;
            if file_row >= self.editor_rows.number_of_rows() {
                if self.editor_rows.number_of_rows() == 0 && i == screen_rows / 3 {
                    let mut welcome = format!("Pound Editor --- Version {}", VERSION);
                    if welcome.len() > screen_columns {
                        welcome.truncate(screen_columns)
                    }
                    let mut padding = (screen_columns - welcome.len()) / 2;
                    if padding != 0 {
                        self.editor_contents.push('~');
                        padding -= 1
                    }
                    (0..padding).for_each(|_| self.editor_contents.push(' '));
                    self.editor_contents.push_str(&welcome);
                } else {
                    self.editor_contents.push('~');
                }
            } else {
                let row = self.editor_rows.get_render(file_row); // modify
                let column_offset = self.cursor_controller.column_offset;
                let len = cmp::min(row.len().saturating_sub(column_offset), screen_columns);
                let start = if len == 0 { 0 } else { column_offset };
                self.editor_contents.push_str(&row[start..start + len])
            }
            queue!(
                self.editor_contents,
                terminal::Clear(ClearType::UntilNewLine)
            )
            .unwrap();
            // if i < screen_rows - 1 {
            self.editor_contents.push_str("\r\n");
            // }
        }
    }

    fn draw_status_bar(&mut self) {
        self.editor_contents
            .push_str(&style::Attribute::Reverse.to_string());
        let info = format!(
            "{} -- {} lines",
            self.editor_rows
                .filename
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("[No Name]"),
            self.editor_rows.number_of_rows()
        );
        let info_len = cmp::min(info.len(), self.win_size.0);
        /* add the following*/
        let line_info = format!(
            "{}/{}",
            self.cursor_controller.cursor_y + 1,
            self.editor_rows.number_of_rows()
        );
        self.editor_contents.push_str(&info[..info_len]);
        for i in info_len..self.win_size.0 {
            if self.win_size.0 - i == line_info.len() {
                self.editor_contents.push_str(&line_info);
                break;
            } else {
                self.editor_contents.push(' ')
            }
        }
        /* end */
        self.editor_contents
            .push_str(&style::Attribute::Reset.to_string());
        self.editor_contents.push_str("\r\n");
    }

    fn draw_message_bar(&mut self) {
        queue!(
            self.editor_contents,
            terminal::Clear(ClearType::UntilNewLine)
        )
        .unwrap();
        if let Some(message) = self.status_message.message() {
            let len = cmp::min(message.len(), self.win_size.0);
            self.editor_contents.push_str(&message[..len]);
        }
    }

    /// refresh screen
    /// 1. clear screen
    /// 2. move cursor to top-left
    /// 3. draw rows
    /// 4. move cursor to top-left
    pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
        self.cursor_controller.scroll(&self.editor_rows); //modify
        queue!(self.editor_contents, cursor::Hide, cursor::MoveTo(0, 0))?;
        self.draw_rows();
        self.draw_status_bar(); // add line
        self.draw_message_bar();
        let cursor_x = self.cursor_controller.render_x - self.cursor_controller.column_offset; // modify
        let cursor_y = self.cursor_controller.cursor_y - self.cursor_controller.row_offset;
        queue!(
            self.editor_contents,
            cursor::MoveTo(cursor_x as u16, cursor_y as u16),
            cursor::Show
        )?;
        self.editor_contents.flush()
    }
}
