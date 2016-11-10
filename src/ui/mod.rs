pub mod intrinsics;

pub enum MouseEvent {
    Click(MouseButton),
    Scroll(f32, f32)
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8)
}

/// The trait implemented for a type that processes nodes. This trait definition might seem a bit
/// strange to you on first glance, as `N` is in the trait definition and not the `add_child`
/// function. The reason for this is specialization. By having `N` outside of `add_child`,
/// implementors of NodeProcessor have a much easier time implementing intrinsics (such as images
/// and text), can define custom intrinsics, extend UI functionality with additional traits, and
/// add type-specific optimizations, all due to specialization. Of course, at this point
/// specialization isn't stable, limiting this library to nightly, but the increases in
/// implementation ergonomics and flexability are well worth it.
pub trait NodeProcessor<'a, N: Node + ?Sized> {
    type Error;
    fn add_child(&'a mut self, name: &'static str, node: &mut N) -> Result<(), Self::Error>;
}

pub trait Node {
    fn type_name() -> &'static str;
    /// An identifier for the current state. Calling this function provides only one guarantee: that
    /// if `node_a != node_b`, `state_id(node_a) != state_id(node_b)`.
    fn state_id(&self) -> u16;
}

/// A node that can have other children as nodes. Unless you have a **VERY** good reason to, this
/// should be `derive`d and not manually implemented, as the current system is set up to allow us
/// to emulate higher-kinded types within the current limitations of the type system.
pub trait ParentNode<NP, E>: Node {
    fn children(&mut self, NP) -> Result<(), E>;
}

pub trait Control: Node {
    type Action;

    fn on_mouse_event(&mut self, MouseEvent) -> Option<Self::Action> {None}
}
