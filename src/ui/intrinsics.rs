use super::{Node, WrapperNodeProcessor, NodeDataWrapper, NodeProcessorAT, Parent, ActionNode, Control};
use std::ops::{Deref, DerefMut};
use rand::{Rng, thread_rng};

macro_rules! intrinsics {
    () => {};
    (pub struct $name:ident<$inner_ty:ident $(: $($constraint:path)|+ )*, P: NodeProcessorAT>;

    impl $name_impl:ident {
        pub fn $get_inner:ident(this: &Self) -> &_;
        pub fn $get_inner_mut:ident(this: &mut Self) -> &mut _;
    }

    $($rest:tt)*) =>
    {
        pub struct $name<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT + WrapperNodeProcessor<$name<$inner_ty, P>>,
                      P::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            data: P::NodeDataWrapper,
            state_id: u16
        }

        impl<$inner_ty, P> $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT + WrapperNodeProcessor<$name<$inner_ty, P>>,
                      P::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            pub fn new(widget_data: $inner_ty) -> Self {
                $name {
                    data: P::NodeDataWrapper::from_node_data(widget_data),
                    state_id: refresh_state_id(0),
                }
            }

            pub fn $get_inner(this: &Self) -> &$inner_ty {
                this.data.inner()
            }

            pub fn $get_inner_mut(this: &mut Self) -> &mut $inner_ty {
                this.state_id = refresh_state_id(this.state_id);

                this.data.inner_mut()
            }

            pub fn unwrap(this: Self) -> $inner_ty {
                this.data.unwrap()
            }
        }

        impl<$inner_ty, P> AsRef<$inner_ty> for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT + WrapperNodeProcessor<$name<$inner_ty, P>>,
                      P::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn as_ref(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$inner_ty, P> AsMut<$inner_ty> for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT + WrapperNodeProcessor<$name<$inner_ty, P>>,
                      P::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn as_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$inner_ty, P> Deref for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT + WrapperNodeProcessor<$name<$inner_ty, P>>,
                      P::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            type Target = $inner_ty;
            fn deref(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$inner_ty, P> DerefMut for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT + WrapperNodeProcessor<$name<$inner_ty, P>>,
                      P::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            fn deref_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$inner_ty, P> Node<P::NodeDataWrapper> for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT + WrapperNodeProcessor<$name<$inner_ty, P>>,
                      P::NodeDataWrapper: NodeDataWrapper<$inner_ty>
        {
            type Inner = $inner_ty;

            fn type_name(&self) -> &'static str {
                stringify!($name)
            }

            fn state_id(&self) -> u16 {
                self.state_id
            }

            fn data(&self) -> &P::NodeDataWrapper {
                &self.data
            }

            fn data_mut(&mut self) -> &mut P::NodeDataWrapper {
                &mut self.data
            }
        }

        intrinsics!{$($rest)*}
    };
}

intrinsics!{
    pub struct TextButton<I: AsRef<str> | Control, P: NodeProcessorAT>;

    impl TextButton {
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }


    pub struct TextLabel<S: AsRef<str>, P: NodeProcessorAT>;

    impl TextLabel {
        pub fn text(this: &Self) -> &_;
        pub fn text_mut(this: &mut Self) -> &mut _;
    }


    pub struct WidgetGroup<I: Parent<P>, P: NodeProcessorAT>;

    impl WidgetGroup {
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }
}

fn refresh_state_id(state_id: u16) -> u16 {
    state_id ^ thread_rng().gen_range(1, u16::max_value())
}


impl<I, P> ActionNode<P::NodeDataWrapper> for TextButton<I, P>
        where I: AsRef<str> + Control,
              P: NodeProcessorAT + WrapperNodeProcessor<TextButton<I, P>>,
              P::NodeDataWrapper: NodeDataWrapper<I>
{
    type Action = I::Action;
}

impl<I, P> ActionNode<P::NodeDataWrapper> for WidgetGroup<I, P>
        where I: Parent<P>,
              P: NodeProcessorAT + WrapperNodeProcessor<WidgetGroup<I, P>>,
              P::NodeDataWrapper: NodeDataWrapper<I>
{
    type Action = I::ChildAction;
}
