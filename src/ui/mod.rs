pub mod intrinsics;
pub mod layout;

pub use dle::geometry;

use self::layout::GridLayout;

pub enum MouseEvent {
    Clicked(MouseButton),
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
pub trait NodeProcessor<N: Node>: Sized + NodeProcessorAT {
    /// Add a child to the node processor.
    ///
    /// Unsafe, because it cannot guarantee that `node` is truely immutable (due to `Cell`, `RefCell`,
    /// etc.). Derin does not support interior mutability, and mutating a node through interior
    /// mutability while it is being processed for events is undefined behavior.
    unsafe fn add_child(&mut self, name: &'static str, node: &N) -> Result<(), Self::Error>;
}

pub trait NodeProcessorAT: Sized {
    type Error;
}

pub trait Node {
    type Action;

    fn type_name(&self) -> &'static str;
    /// An identifier for the current state. Calling this function provides only one guarantee: that
    /// if `node_a != node_b`, `state_id(node_a) != state_id(node_b)`.
    fn state_id(&self) -> u16;
}

/// A node that can have other children as nodes. Unless you have a **VERY** good reason to, this
/// should be `derive`d and not manually implemented, as the current system is set up to allow us
/// to emulate higher-kinded types within the current limitations of the type system.
pub trait ParentNode<NP>: Node
        where NP: NodeProcessorAT {
    type Layout: GridLayout;

    fn children(&self, NP) -> Result<(), NP::Error>;
    fn child_layout(&self) -> Self::Layout;
}

pub trait Control {
    type Action;

    fn on_mouse_event(&self, MouseEvent) -> Option<Self::Action> {None}
}
