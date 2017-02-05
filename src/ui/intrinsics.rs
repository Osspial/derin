use super::{Node, ActionNode, Control};
use std::ops::{Deref, DerefMut};
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
            state_id: refresh_state_id(0)
        }
    }

    pub fn inner(this: &Self) -> &I {
        &this.inner
    }

    pub fn inner_mut(this: &mut Self) -> &mut I {
        this.state_id = refresh_state_id(this.state_id);

        &mut this.inner
    }

    pub fn unwrap(this: Self) -> I {
        this.inner
    }
}

pub struct TextLabel<S: AsRef<str>> {
    text: S,
    state_id: u16
}

impl<S: AsRef<str>> TextLabel<S> {
    pub fn new(text: S) -> TextLabel<S> {
        TextLabel {
            text: text,
            state_id: refresh_state_id(0)
        }
    }

    pub fn text(this: &Self) -> &S {
        &this.text
    }

    pub fn text_mut(this: &mut Self) -> &mut S {
        this.state_id = refresh_state_id(this.state_id);

        &mut this.text
    }

    pub fn unwrap(this: Self) -> S {
        this.text
    }
}

fn refresh_state_id(state_id: u16) -> u16 {
    state_id ^ thread_rng().gen_range(1, u16::max_value())
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

impl<I> Deref for TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    type Target = I;
    fn deref(&self) -> &I {
        TextButton::inner(self)
    }
}

impl<I> DerefMut for TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    fn deref_mut(&mut self) -> &mut I {
        TextButton::inner_mut(self)
    }
}

impl<I> Node for TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    fn type_name(&self) -> &'static str {
        "TextButton"
    }

    fn state_id(&self) -> u16 {
        self.state_id
    }
}

impl<I> ActionNode for TextButton<I>
        where I: 'static + AsRef<str> + Control
{
    type Action = I::Action;
}

impl<S: AsRef<str>> AsRef<S> for TextLabel<S> {
    fn as_ref(&self) -> &S {
        TextLabel::text(self)
    }
}

impl<S: AsRef<str>> AsMut<S> for TextLabel<S> {
    fn as_mut(&mut self) -> &mut S {
        TextLabel::text_mut(self)
    }
}

impl<S: AsRef<str>> Deref for TextLabel<S> {
    type Target = S;
    fn deref(&self) -> &S {
        TextLabel::text(self)
    }
}

impl<S: AsRef<str>> DerefMut for TextLabel<S> {
    fn deref_mut(&mut self) -> &mut S {
        TextLabel::text_mut(self)
    }
}

impl<S: AsRef<str>> Node for TextLabel<S> {
    fn type_name(&self) -> &'static str {
        "TextLabel"
    }

    fn state_id(&self) -> u16 {
        self.state_id
    }
}
