use crossterm::terminal;
use editor::output::Output;

pub mod editor;

struct Cleaner;

impl Drop for Cleaner {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not turn off raw mode");
        Output::clear_screen().expect("Error"); /* add this line*/
    }
}

fn main() -> crossterm::Result<()> {
    let _cleaner = Cleaner;
    terminal::enable_raw_mode()?; // enable raw mode

    let a: Box<str> = "".into();
    let mut editor = editor::Editor::new();
    while editor.run()? {}

    Ok(())
}
