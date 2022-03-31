use std::cmp;

use crossterm::event::KeyCode;

use super::{EditorRows, Row};

static TAB_STOP: usize = 8;

#[derive(Copy, Clone)] // 我们想保存状态，所以需要 Copy 和 Clone
pub(crate) struct CursorController {
    pub cursor_x: usize, // column
    pub cursor_y: usize, // row, max value is equal to number of rows of file.
    screen_columns: usize,
    screen_rows: usize,
    pub row_offset: usize,
    pub column_offset: usize,
    pub render_x: usize,
}

impl CursorController {
    pub(crate) fn new((screen_column, screen_row): (usize, usize)) -> Self {
        CursorController {
            cursor_x: 0,
            cursor_y: 0,
            screen_columns: screen_column,
            screen_rows: screen_row,
            row_offset: 0,
            column_offset: 0,
            render_x: 0,
        }
    }

    /// KeyCode: KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End
    /// usize: row offset in file content
    pub(crate) fn move_cursor(
        &mut self,
        arrow_key: KeyCode,
        editor_rows: &EditorRows,
    ) -> (usize, usize) {
        let number_of_rows = editor_rows.number_of_rows();

        match arrow_key {
            KeyCode::Up => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.cursor_y < number_of_rows {
                    //modify
                    self.cursor_y += 1;
                }
            }
            KeyCode::Left => {
                if self.cursor_x != 0 {
                    self.cursor_x -= 1;
                } else {
                    if self.cursor_y != 0 {
                        self.cursor_y -= 1;
                        self.cursor_x = editor_rows.get_editor_row(self.cursor_y).origin_len();
                        self.column_offset = editor_rows
                            .get_editor_row(self.cursor_y)
                            .origin_len()
                            .saturating_sub(self.screen_columns);
                    }
                }
            }
            KeyCode::Right => {
                // 如果光标没有超出文件的最大行数
                if self.cursor_y < number_of_rows {
                    // 如果光标没有超出该行的最大列数, 则光标右移
                    if self.cursor_x < editor_rows.get_editor_row(self.cursor_y).origin_len() {
                        self.cursor_x += 1;
                    }
                    // 如果光标超出该行的最大列数
                    else {
                        // 如果光标还没有到达最后一行, 则把光标移动到下一行的第一列
                        if self.cursor_y < number_of_rows - 1 {
                            self.cursor_x = 0;
                            self.cursor_y += 1;
                        }
                    }
                }
            }
            KeyCode::Home => {
                if self.cursor_y < number_of_rows {
                    self.cursor_x = 0;
                }
            }
            KeyCode::End => {
                if self.cursor_y < number_of_rows {
                    self.cursor_x = editor_rows.get_editor_row(self.cursor_y).render.len();
                }
            }
            _ => {}
        }

        // start 考虑光标x坐标是不是落在了空白处
        let row_len = if self.cursor_y < number_of_rows {
            editor_rows.get_editor_row(self.cursor_y).origin_len()
        } else {
            0
        };
        self.cursor_x = cmp::min(self.cursor_x, row_len);
        // end

        (self.cursor_x, self.cursor_y)
    }
    pub(crate) fn scroll(&mut self, editor_rows: &EditorRows) {
        //
        self.render_x = 0;
        if self.cursor_y < editor_rows.number_of_rows() {
            // 取得实际的光标的位置
            self.render_x = self.get_render_x(editor_rows.get_editor_row(self.cursor_y))
        }
        /*
        // 光标超过了屏幕上边界, 则向上滚动一行
        if self.cursor_y < self.row_offset {
            self.row_offset = self.cursor_y;
        }
        // 光标超过了屏幕下边界, 则向下滚动一行
        else {
            if self.cursor_y >= self.row_offset + self.screen_rows {
                self.row_offset = self.cursor_y - self.screen_rows + 1;
            }
        }
        */
        self.row_offset = cmp::min(self.row_offset, self.cursor_y);
        if self.cursor_y >= self.row_offset + self.screen_rows {
            self.row_offset = self.cursor_y - self.screen_rows + 1;
        }

        /*
        // 光标超过了屏幕左边界, 则向左滚动一列
        if self.cursor_x < self.column_offset {
            self.column_offset = self.cursor_x;
        }
        // 光标超过了屏幕右边界, 则向右滚动一列
        else {
            if self.cursor_x >= self.column_offset + self.screen_columns {
                self.column_offset = self.cursor_x - self.screen_columns + 1;
            }
        }
        */
        self.column_offset = cmp::min(self.column_offset, self.render_x); //modify
        if self.render_x >= self.column_offset + self.screen_columns {
            //modify
            self.column_offset = self.render_x - self.screen_columns + 1; //modify
        }
    }
    fn get_render_x(&self, row: &Row) -> usize {
        row.row_content[..self.cursor_x]
            .chars()
            .fold(0, |render_x, c| {
                if c == '\t' {
                    render_x + (TAB_STOP - 1) - (render_x % TAB_STOP) + 1
                } else {
                    render_x + 1
                }
            })
    }
}
