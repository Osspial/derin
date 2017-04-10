use super::{Node, NodeDataRegistry, NodeDataWrapper, Parent};
use super::buttons::MouseButton;
use self::status::*;

use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, BorrowMut};
use native::NativeWrapperRegistry;


macro_rules! intrinsics {
    () => {};
    (
        pub struct $name:ident$(<$generic:ident>),*($inner_ty:ty)
                $(where $($where_ty:ty: $($constraint:path)|+),+)*;

        impl $name_impl:ident {
            type Action = $action:ty;
            pub fn $get_inner:ident(this: &Self) -> &_;
            pub fn $get_inner_mut:ident(this: &mut Self) -> &mut _;
        }

        $($rest:tt)*
    ) => {
        pub struct $name<$($generic,)* R = NativeWrapperRegistry>
                where $($($generic: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            wrapper: R::NodeDataWrapper
        }

        impl<$($generic,)* R> $name_impl<$($generic,)* R>
                where $($($generic: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            pub fn new(widget_data: $inner_ty) -> Self {
                $name {
                    wrapper: R::NodeDataWrapper::from_node_data(widget_data)
                }
            }

            pub fn $get_inner(this: &Self) -> &$inner_ty {
                this.wrapper.inner()
            }

            pub fn $get_inner_mut(this: &mut Self) -> &mut $inner_ty {
                this.wrapper.inner_mut()
            }

            pub fn unwrap(this: Self) -> $inner_ty {
                this.wrapper.unwrap()
            }
        }

        impl<$($generic,)* R> Borrow<$inner_ty> for $name_impl<$($generic,)* R>
                where $($($generic: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn borrow(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$($generic,)* R> BorrowMut<$inner_ty> for $name_impl<$($generic,)* R>
                where $($($generic: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn borrow_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$($generic,)* R> Deref for $name_impl<$($generic,)* R>
                where $($($generic: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            type Target = $inner_ty;
            fn deref(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$($generic,)* R> DerefMut for $name_impl<$($generic,)* R>
                where $($($generic: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn deref_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$($generic,)* R> Node for $name_impl<$($generic,)* R>
                where $($($generic: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            type Wrapper = R::NodeDataWrapper;
            type Inner = $inner_ty;
            type Action = $action;

            fn type_name(&self) -> &'static str {
                stringify!($name)
            }

            fn wrapper(&self) -> &R::NodeDataWrapper {
                &self.wrapper
            }

            fn wrapper_mut(&mut self) -> &mut R::NodeDataWrapper {
                &mut self.wrapper
            }
        }

        intrinsics!{$($rest)*}
    };
}

intrinsics!{
    pub struct TextButton<I>(I)
            where I: ButtonControl | Borrow<str>;
    impl TextButton {
        type Action = I::Action;
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }

    pub struct TextLabel<S>(S)
            where S: AsRef<str>;
    impl TextLabel {
        type Action = !;
        pub fn text(this: &Self) -> &_;
        pub fn text_mut(this: &mut Self) -> &mut _;
    }

    pub struct WidgetGroup<I>(I)
            where I: Parent<!>;
    impl WidgetGroup {
        type Action = I::ChildAction;
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }

    pub struct ProgressBar(progbar::Status);
    impl ProgressBar {
        type Action = !;
        pub fn status(this: &Self) -> &_;
        pub fn status_mut(this: &mut Self) -> &mut _;
    }

    pub struct Slider<I>(I)
            where I: SliderControl;
    impl Slider {
        type Action = I::Action;
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Clicked(MouseButton),
    DoubleClicked(MouseButton)
}

pub struct RangeEvent {
    pub moved_to: u32
}

pub trait ButtonControl {
    type Action;

    fn on_mouse_event(&self, MouseEvent) -> Option<Self::Action>;
}

pub trait SliderControl {
    type Action;

    fn status(&self) -> slider::Status;
    fn status_mut(&mut self) -> &mut slider::Status;
    fn on_range_event(&self, RangeEvent) -> Option<Self::Action>;
}

pub mod status {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Orientation {
        Horizontal,
        Vertical
    }

    impl Default for Orientation {
        #[inline]
        fn default() -> Orientation {
            Orientation::Horizontal
        }
    }

    pub mod progbar {
        use super::Orientation;

        #[derive(Default, Debug, Clone, Copy, PartialEq)]
        pub struct Status {
            pub completion: Completion,
            pub orientation: Orientation
        }

        impl Status {
            #[inline]
            pub fn new(completion: Completion, orientation: Orientation) -> Status {
                Status {
                    completion,
                    orientation
                }
            }

            pub fn new_completion(completion: Completion) -> Status {
                Status {
                    completion,
                    orientation: Orientation::default()
                }
            }

            pub fn new_orientation(orientation: Orientation) -> Status {
                Status {
                    completion: Completion::default(),
                    orientation
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum Completion {
            Frac(f32),
            Working
        }

        impl Default for Completion {
            #[inline]
            fn default() -> Completion {
                Completion::Frac(0.0)
            }
        }
    }

    pub mod slider {
        use std::ops::Range;
        use super::Orientation;

        #[derive(Debug, Clone, PartialEq)]
        pub struct Status {
            pub position: u32,
            pub range: Range<u32>,
            pub tick_interval: u32,
            pub orientation: Orientation,
            pub tick_position: TickPosition
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum TickPosition {
            BottomRight,
            TopLeft,
            Both,
            None
        }

        impl Default for Status {
            fn default() -> Status {
                Status {
                    position: 0,
                    range: 0..128,
                    tick_interval: 0,
                    orientation: Orientation::default(),
                    tick_position: TickPosition::default()
                }
            }
        }

        impl Default for TickPosition {
            fn default() -> TickPosition {
                TickPosition::BottomRight
            }
        }
    }
}
