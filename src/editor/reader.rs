use std::time::Duration;

use crossterm::event::{self, poll, Event, KeyEvent};

/// This is a producer.
pub struct Reader;

impl Reader {
    pub fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            if poll(Duration::from_millis(500))? {
                if let Ok(Event::Key(event)) = event::read() {
                    return Ok(event);
                }
            }
        }
    }
}
