use super::{Node, NodeDataRegistry, NodeDataWrapper, Parent, Button};
use std::ops::{Deref, DerefMut};
use std::borrow::Borrow;
use native::NativeWrapperRegistry;
use void::Void;

macro_rules! intrinsics {
    () => {};
    (pub struct $name:ident<$inner_ty:ident $(: $($constraint:path)|+ )*, R: NodeDataRegistry>;

    impl $name_impl:ident {
        type Action = $action:ty;
        pub fn $get_inner:ident(this: &Self) -> &_;
        pub fn $get_inner_mut:ident(this: &mut Self) -> &mut _;
    }

    $($rest:tt)*) =>
    {
        pub struct $name<$inner_ty, R = NativeWrapperRegistry>
                where $($inner_ty: $($constraint + )+,)*
                      R: NodeDataRegistry<$name<$inner_ty, R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            wrapper: R::NodeDataWrapper
        }

        impl<$inner_ty, R> $name_impl<$inner_ty, R>
                where $($inner_ty: $($constraint + )+,)*
                      R: NodeDataRegistry<$name<$inner_ty, R>>,
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

        impl<$inner_ty, R> AsRef<$inner_ty> for $name_impl<$inner_ty, R>
                where $($inner_ty: $($constraint + )+,)*
                      R: NodeDataRegistry<$name<$inner_ty, R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn as_ref(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$inner_ty, R> AsMut<$inner_ty> for $name_impl<$inner_ty, R>
                where $($inner_ty: $($constraint + )+,)*
                      R: NodeDataRegistry<$name<$inner_ty, R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn as_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$inner_ty, R> Deref for $name_impl<$inner_ty, R>
                where $($inner_ty: $($constraint + )+,)*
                      R: NodeDataRegistry<$name<$inner_ty, R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            type Target = $inner_ty;
            fn deref(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$inner_ty, R> DerefMut for $name_impl<$inner_ty, R>
                where $($inner_ty: $($constraint + )+,)*
                      R: NodeDataRegistry<$name<$inner_ty, R>>,
                      R::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn deref_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$inner_ty, R> Node for $name_impl<$inner_ty, R>
                where $($inner_ty: $($constraint + )+,)*
                      R: NodeDataRegistry<$name<$inner_ty, R>>,
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
    pub struct TextButton<I: Button | Borrow<str>, R: NodeDataRegistry>;
    impl TextButton {
        type Action = I::Action;
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }

    pub struct TextLabel<S: AsRef<str>, R: NodeDataRegistry>;
    impl TextLabel {
        type Action = Void;
        pub fn text(this: &Self) -> &_;
        pub fn text_mut(this: &mut Self) -> &mut _;
    }

    pub struct WidgetGroup<I: Parent<()>, R: NodeDataRegistry>;
    impl WidgetGroup {
        type Action = I::ChildAction;
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }
}

pub struct ProgressBar<R = NativeWrapperRegistry>
        where R: NodeDataRegistry<ProgressBar<R>>,
              R::NodeDataWrapper: NodeDataWrapper<ProgBarStatus>
{
    wrapper: R::NodeDataWrapper
}

impl<R> ProgressBar<R>
        where R: NodeDataRegistry<ProgressBar<R>>,
              R::NodeDataWrapper: NodeDataWrapper<ProgBarStatus>
{
    pub fn new(status: ProgBarStatus) -> ProgressBar<R> {
        ProgressBar {
            wrapper: R::NodeDataWrapper::from_node_data(status)
        }
    }

    pub fn status(&self) -> ProgBarStatus {
        *self.wrapper.inner()
    }

    pub fn status_mut(&mut self) -> &mut ProgBarStatus {
        self.wrapper.inner_mut()
    }
}

impl<R> Node for ProgressBar<R>
        where R: NodeDataRegistry<ProgressBar<R>>,
              R::NodeDataWrapper: NodeDataWrapper<ProgBarStatus>
{
    type Wrapper = R::NodeDataWrapper;
    type Inner = ProgBarStatus;
    type Action = Void;

    fn type_name(&self) -> &'static str {
        "ProgressBar"
    }

    fn wrapper(&self) -> &R::NodeDataWrapper {
        &self.wrapper
    }

    fn wrapper_mut(&mut self) -> &mut R::NodeDataWrapper {
        &mut self.wrapper
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgBarStatus {
    Frac(f32),
    Working
}
