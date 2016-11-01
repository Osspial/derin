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
    fn add_child(&mut self, name: &'static str, node: &mut N);
}

pub trait Node {
    fn type_name() -> &'static str;
    fn num_updates(&self) -> u64;
}

pub trait ParentNode: Node {
    fn children<'a, N>(&mut self, N) where N: NodeProcessor<'a, Self>;
}

pub trait Control: Node {
    type Action;

    fn on_mouse_event(&mut self, MouseEvent) -> Option<Self::Action> {None}
}
