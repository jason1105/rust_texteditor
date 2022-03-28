use std::{
    cmp, env, fs,
    io::{self, stdout, ErrorKind, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::KeyCode,
    execute, queue, style,
    terminal::{self, ClearType},
};

use itertools::Itertools;
use itertools::*;

use crate::prompt;

use self::cursor_controller::CursorController;

pub mod cursor_controller;

static VERSION: &str = "0.1.0";
static TAB_STOP: usize = 8;

#[derive(Default)]
struct Row {
    row_content: String,
    render: String,
}

impl Row {
    fn new(row_content: String, render: String) -> Self {
        Self {
            row_content,
            render,
        }
    }

    fn len(&self) -> usize {
        self.row_content.len()
    }

    fn insert_char(&mut self, at: usize, ch: char) {
        self.row_content.insert(at, ch);
        EditorRows::render_row(self)
    }

    fn delete_char(&mut self, at: usize) {
        self.row_content.remove(at);
        EditorRows::render_row(self)
    }

    /// args
    ///     usize: x position of cursor which is a offset in rendered row.
    /// Returns
    ///     usize: x position of cursor in row_content of not being rendered
    fn get_row_content_x(&self, render_x: usize) -> usize {
        let mut current_render_x = 0;
        for (cursor_x, ch) in self.row_content.chars().enumerate() {
            if ch == '\t' {
                current_render_x += (TAB_STOP - 1) - (current_render_x % TAB_STOP);
            }
            current_render_x += 1;
            if current_render_x > render_x {
                return cursor_x;
            }
        }
        0
    }
}

pub struct StatusMessage {
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

    pub fn set_message(&mut self, message: String) {
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

/// Buffer of editor
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

/// File's content
/// 1. Read file
/// 2. Write file
/// 3. Edit file
/// 4. Render file
pub struct EditorRows {
    row_contents: Vec<Row>, // Box<str> and String are same, but Box<str> is more efficient and smaller.
    pub filename: Option<PathBuf>, //add field
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

    fn insert_row(&mut self, at: usize, contents: String) {
        let mut new_row = Row::new(contents, String::new());
        EditorRows::render_row(&mut new_row);
        self.row_contents.insert(at, new_row);
    }

    fn get_editor_row_mut(&mut self, at: usize) -> &mut Row {
        &mut self.row_contents[at]
    }

    pub fn save(&self) -> io::Result<usize> {
        match &self.filename {
            None => Err(io::Error::new(ErrorKind::Other, "no file name specified")),
            Some(name) => {
                let mut file = fs::OpenOptions::new().write(true).create(true).open(name)?;
                let contents: String = self
                    .row_contents
                    .iter()
                    .map(|it| it.row_content.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n");
                file.set_len(contents.len() as u64)?;
                file.write_all(contents.as_bytes())?;
                Ok(contents.as_bytes().len())
            }
        }
    }

    fn join_adjacent_rows(&mut self, at: usize) {
        let current_row = self.row_contents.remove(at);
        let previous_row = self.get_editor_row_mut(at - 1);
        previous_row.row_content.push_str(&current_row.row_content);
        Self::render_row(previous_row);
    }
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
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
    pub status_message: StatusMessage,
    pub dirty: u64,
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
            status_message: StatusMessage::new(
                "HELP: Ctrl-S = Save | Ctrl-Q = Quit | Ctrl-F = Find ".into(),
            ),
            dirty: 0,
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
            "{} {} -- {} lines",
            self.editor_rows
                .filename
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("[No Name]"),
            if self.dirty > 0 { "(modified)" } else { "" },
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

    pub fn insert_char(&mut self, ch: char) {
        if self.cursor_controller.cursor_y == self.editor_rows.number_of_rows() {
            self.editor_rows
                .insert_row(self.editor_rows.number_of_rows(), String::new());
            self.dirty += 1;
        }
        self.editor_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y)
            .insert_char(self.cursor_controller.cursor_x, ch);
        self.cursor_controller.cursor_x += 1;
        self.dirty += 1;
    }

    pub fn insert_newline(&mut self) {
        if self.cursor_controller.cursor_x == 0 {
            self.editor_rows
                .insert_row(self.cursor_controller.cursor_y, String::new())
        } else {
            let current_row = self
                .editor_rows
                .get_editor_row_mut(self.cursor_controller.cursor_y);
            let new_row_content = current_row.row_content[self.cursor_controller.cursor_x..].into();
            current_row
                .row_content
                .truncate(self.cursor_controller.cursor_x);
            EditorRows::render_row(current_row);
            self.editor_rows
                .insert_row(self.cursor_controller.cursor_y + 1, new_row_content);
        }
        self.cursor_controller.cursor_x = 0;
        self.cursor_controller.cursor_y += 1;
        self.dirty += 1;
    }

    /// 删除光标前一个字符
    pub fn delete_char(&mut self) {
        if self.cursor_controller.cursor_y == self.editor_rows.number_of_rows() {
            return;
        }
        let row = self
            .editor_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y);
        if self.cursor_controller.cursor_x > 0 {
            row.delete_char(self.cursor_controller.cursor_x - 1);
            self.cursor_controller.cursor_x -= 1;
        } else {
            let previous_row_content = self
                .editor_rows
                .get_editor_row(self.cursor_controller.cursor_y - 1);
            self.cursor_controller.cursor_x = previous_row_content.len();
            self.editor_rows
                .join_adjacent_rows(self.cursor_controller.cursor_y);
            self.cursor_controller.cursor_y -= 1;
        }
        self.dirty += 1;
    }

    fn find_callback(output: &mut Output, keyword: &str, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc | KeyCode::Enter => {}
            _ => {
                // 默认查找范围是所有行
                let mut line_rng =
                    Either::Left((0..output.editor_rows.number_of_rows() - 1).into_iter());

                let cursor_x = output.cursor_controller.cursor_x;
                let cursor_y = output.cursor_controller.cursor_y;

                match key_code {
                    // 按下左右键, 在行内查找
                    x_dir @ (KeyCode::Left | KeyCode::Right) => {
                        let mut column_rng: (usize, usize) = (0, 0); // (start, end)
                        let row = output
                            .editor_rows
                            .get_editor_row(output.cursor_controller.cursor_y);
                        // 确定查找范围: 向左和向右
                        if let KeyCode::Left = x_dir {
                            if let Some(index) = row.render[..cursor_x].rfind(&keyword) {
                                output.cursor_controller.cursor_x = row.get_row_content_x(index);
                            }
                        } else {
                            let start = (cursor_x + 1).min(row.render.len());
                            if let Some(index) = row.render[start..].find(&keyword) {
                                output.cursor_controller.cursor_x =
                                    row.get_row_content_x(index + start);
                            }
                        };

                        // 行内查找结束后, 返回
                        return;
                    }
                    // 按下上下键, 在行间查找
                    y_dir @ (KeyCode::Up | KeyCode::Down) => {
                        // search line by line: (start_line, end_line)

                        let (mut start_line, mut end_line) = (cursor_y, cursor_y);

                        if KeyCode::Up == y_dir {
                            start_line = 0;
                            end_line = end_line.saturating_sub(1);
                        } else {
                            start_line =
                                (start_line + 1).min(output.editor_rows.number_of_rows() - 1);
                            end_line = output.editor_rows.number_of_rows() - 1;
                        }

                        line_rng = Either::Left((start_line..end_line).into_iter());
                        if start_line > end_line {
                            line_rng = Either::Right((end_line..start_line).rev());
                        }
                    }
                    _ => {}
                };

                //
                for i in line_rng {
                    let row = output.editor_rows.get_editor_row(i);
                    if let Some(index) = row.render.find(&keyword) {
                        output.cursor_controller.cursor_y = i;
                        output.cursor_controller.cursor_x = row.get_row_content_x(index);
                        output.cursor_controller.row_offset = output.editor_rows.number_of_rows();
                        break;
                    }
                }
            }
        }
    }

    pub fn find(&mut self) -> io::Result<()> {
        let cursor_controller = self.cursor_controller;
        prompt!(
            self,
            "Search: {} (Use ESC / Arrows / Enter)",
            callback = Output::find_callback
        );
        self.cursor_controller = cursor_controller;
        Ok(())
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
