use super::Node;
use rand::{Rng, thread_rng};

pub struct TextButton<S: AsRef<str> = String> {
    text: S,
    state_id: u64
}

impl<S: AsRef<str>> TextButton<S> {
    pub fn new(text: S) -> TextButton<S> {
        TextButton {
            text: text,
            state_id: 0
        }
    }

    pub fn unwrap(self) -> S {
        self.text
    }
}

impl<S: AsRef<str>> AsRef<str> for TextButton<S> {
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<S: AsRef<str>> AsMut<str> for TextButton<S>
        where S: AsMut<str> {
    fn as_mut(&mut self) -> &mut str {
        self.state_id = thread_rng().next_u64();

        self.text.as_mut()
    }
}

impl AsRef<String> for TextButton<String> {
    fn as_ref(&self) -> &String {
        &self.text
    }
}

impl AsMut<String> for TextButton<String> {
    fn as_mut(&mut self) -> &mut String {
        self.state_id = thread_rng().next_u64();

        &mut self.text
    }
}

impl<S: AsRef<str>> Node for TextButton<S> {
    fn type_name() -> &'static str {
        "TextButton"
    }

    fn state_id(&self) -> u64 {
        self.state_id
    }
}
