use super::{Node, Control};
use rand::{Rng, thread_rng};

pub struct TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    inner: I,
    state_id: u16
}

impl<I> TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    pub fn new(inner: I) -> TextButton<I> {
        TextButton {
            inner: inner,
            state_id: thread_rng().gen_range(1, u16::max_value())
        }
    }

    pub fn inner(this: &Self) -> &I {
        &this.inner
    }

    pub fn inner_mut(this: &mut Self) -> &mut I {
        this.refresh_state_id();

        &mut this.inner
    }

    pub fn unwrap(this: Self) -> I {
        this.inner
    }

    fn refresh_state_id(&mut self) {
        self.state_id ^= thread_rng().gen_range(1, u16::max_value());
    }
}

impl<I> AsRef<I> for TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    fn as_ref(&self) -> &I {
        TextButton::inner(self)
    }
}

impl<I> AsMut<I> for TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    fn as_mut(&mut self) -> &mut I {
        TextButton::inner_mut(self)
    }
}

impl<I> Node for TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    type Action = I::Action;

    fn type_name(&self) -> &'static str {
        "TextButton"
    }

    fn state_id(&self) -> u16 {
        self.state_id
    }
}
