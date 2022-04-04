use crossterm::style::*;
use crossterm::{
    cursor,
    event::KeyCode,
    execute, queue, style,
    terminal::{self, ClearType},
};
use itertools::Itertools;
use itertools::*;
use std::{
    cmp, env, fs,
    io::{self, stdout, ErrorKind, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::prompt;

use self::cursor_controller::CursorController;

pub mod cursor_controller;

static VERSION: &str = "0.1.0";
static TAB_STOP: usize = 8;

/// Describe what type each char should be given in specific syntax rules.
#[derive(Clone, Copy)]
pub enum HighlightType {
    Normal,
    Number,
    SearchMatch,
}

#[derive(Default)]
pub struct Row {
    row_content: String,
    render: String,
    highlight: Vec<HighlightType>, // Save the type of each char in render of this row. So that we can render it in different color.
}

impl Row {
    pub fn new(row_content: String, render: String) -> Self {
        Self {
            row_content,
            render,
            highlight: Vec::new(),
        }
    }

    fn origin_len(&self) -> usize {
        self.row_content.len()
    }

    fn render_len(&self) -> usize {
        self.render.len()
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
pub struct EditorContents {
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
    pub row_contents: Vec<Row>, // Box<str> and String are same, but Box<str> is more efficient and smaller.
    pub filename: Option<PathBuf>, //add field
}

impl EditorRows {
    fn new(syntax_highlight: &mut Option<Box<dyn SyntaxHighlight>>) -> Self {
        match env::args().nth(1) {
            None => Self {
                row_contents: Vec::new(),
                filename: None,
            },
            Some(file) => Self::from_file(file.into(), syntax_highlight),
        }
    }

    fn from_file(file: PathBuf, syntax_highlight: &mut Option<Box<dyn SyntaxHighlight>>) -> Self {
        let file_contents = fs::read_to_string(&file).expect("Unable to read file");

        file.extension()
            .and_then(|ext| ext.to_str()) // 使用 and_then() 而不是 map(), 因为 ext.to_str() 返回的是 Option
            .map(|ext| Output::select_syntax(ext).map(|syntax| syntax_highlight.insert(syntax)));

        let mut content: Vec<Row> = Vec::new();

        file_contents.lines().enumerate().for_each(|(index, line)| {
            let mut row = Row::new(line.to_string(), String::new());
            Self::render_row(&mut row);
            content.push(row);
            if let Some(s) = syntax_highlight {
                s.update_syntax(index, &mut content);
            }
        });

        /* modify */
        Self {
            row_contents: content,
            filename: Some(file),
        }
        /* end */
    }

    /* add functions*/
    fn get_render(&self, at: usize) -> &String {
        &self.row_contents[at].render
    }

    pub fn render_row(row: &mut Row) {
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
    pub syntax_highlight: Option<Box<dyn SyntaxHighlight>>,
    previous_highlight: Option<(usize, Vec<HighlightType>)>,
}

impl Output {
    pub fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize - 2))
            .unwrap();
        let mut syntax_highlight: Option<Box<dyn SyntaxHighlight>> = None;
        Self {
            win_size,
            editor_contents: EditorContents::new(),
            cursor_controller: CursorController::new(win_size),
            editor_rows: EditorRows::new(&mut syntax_highlight),
            status_message: StatusMessage::new(
                "HELP: Ctrl-S = Save | Ctrl-Q = Quit | Ctrl-F = Find ".into(),
            ),
            dirty: 0,
            syntax_highlight,
            previous_highlight: None,
        }
    }

    pub fn select_syntax(extension: &str) -> Option<Box<dyn SyntaxHighlight>> {
        let list: Vec<Box<dyn SyntaxHighlight>> = vec![Box::new(RustHighlight::new())];
        // list.push(other highlight);
        list.into_iter()
            .find(|it| it.extensions().contains(&extension))
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
                let row = self.editor_rows.get_editor_row(file_row);
                let render = &row.render;
                let column_offset = self.cursor_controller.column_offset;
                let len = cmp::min(
                    row.render_len().saturating_sub(column_offset),
                    screen_columns,
                );
                let start = if len == 0 { 0 } else { column_offset };
                // self.editor_contents.push_str(&row[start..start + len]);
                // let _ = &row[start..start + len]
                //     .chars()
                //     .zip((start..start + len).into_iter())
                //     .for_each(|(c, _index)| {
                //         if c.is_ascii_digit() {
                //             queue!(self.editor_contents, SetForegroundColor(Color::Red)).unwrap();
                //             self.editor_contents.push(c);
                //             queue!(self.editor_contents, ResetColor).unwrap();
                //         } else {
                //             self.editor_contents.push(c);
                //         }
                //     });

                /*
                - Method of 'as_ref" is used to avoid borrow checker error.
                - Combine methods of 'map' and 'unwrap_or_else' to realize 'if else' functionality.
                */
                self.syntax_highlight
                    .as_ref()
                    .map(|syntax_highlight| {
                        syntax_highlight.color_row(
                            &render[start..start + len],
                            &row.highlight[start..start + len],
                            &mut self.editor_contents,
                        )
                    })
                    .unwrap_or_else(|| self.editor_contents.push_str(&render[start..start + len]));
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

        // Update syntax highlighting
        if let Some(it) = self.syntax_highlight.as_ref() {
            it.update_syntax(
                self.cursor_controller.cursor_y,
                &mut self.editor_rows.row_contents,
            )
        }

        self.cursor_controller.cursor_x += 1;
        self.dirty += 1;
    }

    pub fn insert_newline(&mut self) {
        /* Insert blank line. */
        if self.cursor_controller.cursor_x == 0 {
            self.editor_rows
                .insert_row(self.cursor_controller.cursor_y, String::new())
        }
        /* Split line */
        else {
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

            // Update highlight for the new row.
            if let Some(it) = self.syntax_highlight.as_ref() {
                it.update_syntax(
                    self.cursor_controller.cursor_y,
                    &mut self.editor_rows.row_contents,
                );
                it.update_syntax(
                    self.cursor_controller.cursor_y + 1,
                    &mut self.editor_rows.row_contents,
                )
            }
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
            self.cursor_controller.cursor_x = previous_row_content.origin_len();
            self.editor_rows
                .join_adjacent_rows(self.cursor_controller.cursor_y);
            self.cursor_controller.cursor_y -= 1;
        }
        if let Some(it) = self.syntax_highlight.as_ref() {
            it.update_syntax(
                self.cursor_controller.cursor_y,
                &mut self.editor_rows.row_contents,
            );
        }
        self.dirty += 1;
    }

    fn find_callback(output: &mut Output, keyword: &str, key_code: KeyCode) {
        // Restore highlight.
        if let Some((row_index, highlight)) = output.previous_highlight.take() {
            output.editor_rows.get_editor_row_mut(row_index).highlight = highlight;
        }
        // Search for keyword.
        match key_code {
            KeyCode::Esc | KeyCode::Enter => {}
            _ => {
                // 默认查找范围是所有行
                let mut line_rng = Either::Left(0..output.editor_rows.number_of_rows());

                let cursor_x = output.cursor_controller.cursor_x;
                let cursor_y = output.cursor_controller.cursor_y;

                /*
                Logic: (Now solution, but not good enough)
                    1. If press Left or Right, search in line of cursor_y.
                    2. If press Up or Down, make range in terms of direction, then search the range.
                Optimization:
                    1. If press Up or Down, find out the line which will be searched, or from first line of file.
                    2. If press Left or Right, make x position in terms of direction, or from head of line.
                    3. Search begin from the x in line.
                */
                match key_code {
                    // 按下左右键, 在行内查找
                    x_dir @ (KeyCode::Left | KeyCode::Right) => {
                        let row = output.editor_rows.get_editor_row_mut(cursor_y);
                        // 确定查找范围: 向左和向右
                        if let KeyCode::Left = x_dir {
                            if let Some(index) = row.render[..cursor_x].rfind(&keyword) {
                                output.previous_highlight = Some((cursor_y, row.highlight.clone())); // backup
                                (index..index + keyword.len()).for_each(|i| {
                                    row.highlight[i] = HighlightType::SearchMatch;
                                });
                                output.cursor_controller.cursor_x = row.get_row_content_x(index);
                            }
                        } else {
                            let start = (cursor_x).min(row.render.len());
                            if let Some(index) = row.render[start..].find(&keyword) {
                                output.previous_highlight = Some((cursor_y, row.highlight.clone())); // backup
                                (start + index..start + index + keyword.len()).for_each(|i| {
                                    row.highlight[i] = HighlightType::SearchMatch;
                                });
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

                        if KeyCode::Up == y_dir {
                            line_rng = Either::Right((0..cursor_y).rev());
                        } else {
                            let start_line =
                                (cursor_y + 1).min(output.editor_rows.number_of_rows() - 1);
                            line_rng =
                                Either::Left(start_line..output.editor_rows.number_of_rows());
                        }
                    }
                    _ => {}
                };

                //
                for i in line_rng {
                    let row = output.editor_rows.get_editor_row_mut(i);

                    if let Some(index) = row.render.find(&keyword) {
                        output.previous_highlight = Some((i, row.highlight.clone())); // backup
                        (index..index + keyword.len()).for_each(|at| {
                            row.highlight[at] = HighlightType::SearchMatch;
                        });

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

fn is_separator(c: char) -> bool {
    c.is_whitespace()
        || [
            ',', '.', '(', ')', '+', '-', '/', '*', '=', '~', '%', '<', '>', '"', '\'', ';',
        ]
        .contains(&c)
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

use crate::syntax_struct;

/// 这个宏的作用是将一些数据转换成一个结构体, 虽然传入宏的参数看起来像结构体, 但并不是结构体.
syntax_struct! {
    struct RustHighlight {
        // 可能有多个扩展名
        extensions: ["rs", "rust"]  // invalid syntax, but it's ok in macro invocation, as we could fix it in macro implementation.
    }
}

/// This is a role who is responsible for highlight.
pub trait SyntaxHighlight {
    // Update the syntax highlighting for the chars in current line.
    fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>);
    // Convert type to color
    fn syntax_color(&self, highlight_type: &HighlightType) -> Color; // add method

    // Write to editor.output.buffer
    fn color_row(&self, render: &str, highlight: &[HighlightType], out: &mut EditorContents) {
        let mut current_color = self.syntax_color(&HighlightType::Normal);

        render.chars().enumerate().for_each(|(i, c)| {
            let color = self.syntax_color(&highlight[i]);

            if current_color != color {
                let _ = queue!(out, SetForegroundColor(color));
            }

            out.push(c);
            current_color = color;
        });

        let _ = queue!(out, ResetColor);
    }

    fn extensions(&self) -> &[&str];
}

#[macro_export]
macro_rules! syntax_struct {
    (
        struct $Name:ident {
            extensions: $ext:expr
        }
    ) => {
        struct $Name {
            extensions: &'static [&'static str],
        }

        impl $Name {
            fn new() -> Self {
                $Name { extensions: &$ext }
            }
        }

        impl SyntaxHighlight for $Name {
            fn syntax_color(&self, highlight_type: &HighlightType) -> Color {
                match highlight_type {
                    HighlightType::Normal => Color::Reset,
                    HighlightType::Number => Color::Cyan,
                    HighlightType::SearchMatch => Color::Blue,
                }
            }

            fn extensions(&self) -> &[&str] {
                self.extensions
            }

            fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>) {
                let current_row = &mut editor_rows[at];
                macro_rules! add {
                    ($h:expr) => {{
                        current_row.highlight.push($h);
                        $h
                    }};
                }

                current_row.highlight = Vec::with_capacity(current_row.render.len());
                let chars = &current_row.render.chars().collect::<Vec<char>>();

                // for c in chars {
                //     if c.is_digit(10) {
                //         add!(HighlightType::Number);
                //     } else {
                //         add!(HighlightType::Normal)
                //     }
                // }

                let mut i = 0;
                let mut previous_separator = true; // Define for loop person.
                let mut previous_highlight_type = HighlightType::Normal; // Define for loop person.
                let mut in_word = false;

                /* WHILE is a queue which maybe infinitive*/
                while i < chars.len() {
                    /*
                    I am a LOOPer in my house, now receive a piece which has been written a number.
                    The number is position of character in render row.
                    Now what I do is recognize all number which consist of character in render row.
                    */
                    let c = chars[i];

                    /* Loop person are watching each item being in loop.*/
                    let highlight_type = if (c.is_digit(10) || c == '.')
                        && (previous_separator
                            || matches!(previous_highlight_type, HighlightType::Number))
                    {
                        add!(HighlightType::Number)
                    } else {
                        add!(HighlightType::Normal)
                    };

                    /* I do these for myself. */
                    // For next loop
                    previous_highlight_type = highlight_type;
                    previous_separator = is_separator(c);
                    i += 1;
                }
                assert_eq!(current_row.render.len(), current_row.highlight.len())
            }
        }
    };
}
