use super::Node;

pub struct TextButton<S: AsRef<str> = String> {
    text: S,
    num_updates: u64
}

impl<S: AsRef<str>> TextButton<S> {
    pub fn new(text: S) -> TextButton<S> {
        TextButton {
            text: text,
            num_updates: 0
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
        self.num_updates += 1;

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
        self.num_updates += 1;

        &mut self.text
    }
}

impl<S: AsRef<str>> Node for TextButton<S> {
    fn type_name() -> &'static str {
        "TextButton"
    }

    fn num_updates(&self) -> u64 {
        self.num_updates
    }
}
