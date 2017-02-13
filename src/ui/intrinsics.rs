use super::{Node, NodeProcessorAT, Parent, ActionNode, Control};
use std::ops::{Deref, DerefMut};
use rand::{Rng, thread_rng};

macro_rules! intrinsics {
    () => {};
    (pub struct $name:ident<$inner_ty:ident $(: $($constraint:path)|+ )*, P: NodeProcessorAT> {
        $inner_name:ident: _,
        data: P::$processor_data:ident
    }

    impl $name_impl:ident {
        pub fn $get_inner:ident(this: &Self) -> &_;
        pub fn $get_inner_mut:ident(this: &mut Self) -> &mut _;
    }

    $($rest:tt)*) =>
    {
        pub struct $name<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT
        {
            $inner_name: $inner_ty,
            state_id: u16,
            data: P::$processor_data
        }

        impl<$inner_ty, P> $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT
        {
            pub fn new($inner_name: $inner_ty) -> Self {
                $name {
                    $inner_name: $inner_name,
                    state_id: refresh_state_id(0),
                    data: P::$processor_data::default()
                }
            }

            pub fn $get_inner(this: &Self) -> &$inner_ty {
                &this.$inner_name
            }

            pub fn $get_inner_mut(this: &mut Self) -> &mut $inner_ty {
                this.state_id = refresh_state_id(this.state_id);

                &mut this.$inner_name
            }

            pub fn unwrap(this: Self) -> $inner_ty {
                this.$inner_name
            }
        }

        impl<$inner_ty, P> AsRef<$inner_ty> for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT
        {
            fn as_ref(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$inner_ty, P> AsMut<$inner_ty> for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT
        {
            fn as_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$inner_ty, P> Deref for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT
        {
            type Target = $inner_ty;
            fn deref(&self) -> &$inner_ty {
                $name_impl::$get_inner(self)
            }
        }

        impl<$inner_ty, P> DerefMut for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT
        {
            fn deref_mut(&mut self) -> &mut $inner_ty {
                $name_impl::$get_inner_mut(self)
            }
        }

        impl<$inner_ty, P> Node for $name_impl<$inner_ty, P>
                where $inner_ty $(: $($constraint + )+)*,
                      P: NodeProcessorAT
        {
            type Data = P::$processor_data;

            fn type_name(&self) -> &'static str {
                stringify!($name)
            }

            fn state_id(&self) -> u16 {
                self.state_id
            }

            fn data(&self) -> &P::$processor_data {
                &self.data
            }

            fn data_mut(&mut self) -> &mut P::$processor_data {
                &mut self.data
            }
        }

        intrinsics!{$($rest)*}
    };
}

intrinsics!{
    pub struct TextButton<I: AsRef<str> | Control, P: NodeProcessorAT> {
        inner: _,
        data: P::TextButtonData
    }

    impl TextButton {
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }


    pub struct TextLabel<S: AsRef<str>, P: NodeProcessorAT> {
        inner: _,
        data: P::TextLabelData
    }

    impl TextLabel {
        pub fn text(this: &Self) -> &_;
        pub fn text_mut(this: &mut Self) -> &mut _;
    }


    pub struct WidgetGroup<I: Parent<P>, P: NodeProcessorAT> {
        inner: _,
        data: P::WidgetGroupData
    }

    impl WidgetGroup {
        pub fn inner(this: &Self) -> &_;
        pub fn inner_mut(this: &mut Self) -> &mut _;
    }
}

fn refresh_state_id(state_id: u16) -> u16 {
    state_id ^ thread_rng().gen_range(1, u16::max_value())
}


impl<I, P> ActionNode for TextButton<I, P>
        where I: AsRef<str> + Control,
              P: NodeProcessorAT
{
    type Action = I::Action;
}

impl<I, P> ActionNode for WidgetGroup<I, P>
        where I: Parent<P>,
              P: NodeProcessorAT
{
    type Action = I::ChildAction;
}
