use super::{Node, NodeDataRegistry, NodeDataWrapper, Parent};
use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, BorrowMut};
use native::NativeWrapperRegistry;

use super::events::MouseEvent;

macro_rules! intrinsics {
    () => {};
    (
        pub struct $name:ident$(<$inner_ty_gen:ident $(: $($constraint:path)|+ )*>)*($inner_ty:ty);

        impl $name_impl:ident {
            type Action = $action:ty;
            pub fn $get_inner:ident(this: &Self) -> &_;
            pub fn $get_inner_mut:ident(this: &mut Self) -> &mut _;
        }

        $($rest:tt)*
    ) => {
        pub struct $name<$($inner_ty_gen,)* R = NativeWrapperRegistry>
                where $($($inner_ty_gen: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($inner_ty_gen,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            wrapper: R::NodeDataWrapper
        }

        impl<$($inner_ty_gen,)* R> $name_impl<$($inner_ty_gen,)* R>
                where $($($inner_ty_gen: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($inner_ty_gen,)* R>>,
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

        impl<$($inner_ty_gen,)* R> Borrow<$inner_ty> for $name_impl<$($inner_ty_gen,)* R>
                where $($($inner_ty_gen: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($inner_ty_gen,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn borrow(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$($inner_ty_gen,)* R> BorrowMut<$inner_ty> for $name_impl<$($inner_ty_gen,)* R>
                where $($($inner_ty_gen: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($inner_ty_gen,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn borrow_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$($inner_ty_gen,)* R> Deref for $name_impl<$($inner_ty_gen,)* R>
                where $($($inner_ty_gen: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($inner_ty_gen,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            type Target = $inner_ty;
            fn deref(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$($inner_ty_gen,)* R> DerefMut for $name_impl<$($inner_ty_gen,)* R>
                where $($($inner_ty_gen: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($inner_ty_gen,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn deref_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$($inner_ty_gen,)* R> Node for $name_impl<$($inner_ty_gen,)* R>
                where $($($inner_ty_gen: $($constraint + )+)*,)*
                      R: NodeDataRegistry<$name<$($inner_ty_gen,)* R>>,
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
    pub struct TextButton<I: Button | Borrow<str>>(I);
    impl TextButton {
        type Action = I::Action;
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }

    pub struct TextLabel<S: AsRef<str>>(S);
    impl TextLabel {
        type Action = !;
        pub fn text(this: &Self) -> &_;
        pub fn text_mut(this: &mut Self) -> &mut _;
    }

    pub struct WidgetGroup<I: Parent<!>>(I);
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
}

pub trait Button {
    type Action;
    fn on_mouse_event(&self, MouseEvent) -> Option<Self::Action>;
}

pub mod progbar {
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Orientation {
        Horizontal,
        Vertical
    }

    impl Default for Completion {
        fn default() -> Completion {
            Completion::Frac(0.0)
        }
    }

    impl Default for Orientation {
        fn default() -> Orientation {
            Orientation::Horizontal
        }
    }
}
