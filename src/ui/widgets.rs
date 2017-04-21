use super::{Node, NodeDataRegistry, NodeDataWrapper, Parent, EventActionMap};
use super::buttons::MouseButton;
use self::status::*;

use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use native::NativeWrapperRegistry;

macro_rules! intrinsics {
    () => {};

    (
        pub struct $name:ident$(<$generic:ident>)*((), $content_data_ty:ty)
                $(where $($where_ty:ty: $($constraint:path)|+),+)*;

        impl $name_impl:ident {
            $(type Map = $map_ty_override:ty;)*
            type Event = $event:ty;

            pub fn $content_data:ident(&self) -> &_;
            pub fn $content_data_mut:ident(&mut self) -> &mut _;
        }

        $($rest:tt)*
    ) => {
        intrinsics!{
            pub struct $name$(<$generic>)*((), $content_data_ty)
                    $(where $($where_ty: $($constraint)|+),+)*;

            impl $name_impl {
                $(type Map = $map_ty_override;)*
                type Event = $event;

                pub fn new(content_data: $content_data_ty) -> Self {
                    let event_action_map = if_tokens!{($($map_ty_override)*) {
                        $(<$map_ty_override as Default>::default())*
                    } else {
                        ()
                    }};
                    $name {
                        wrapper: R::NodeDataWrapper::from_node_data(event_action_map, content_data)
                    }
                }

                pub fn $content_data(&self) -> &_;
                pub fn $content_data_mut(&mut self) -> &mut _;
                pub fn borrow(this: &Self) -> &$content_data_ty {this.$content_data()}
                pub fn borrow_mut(this: &mut Self) -> &mut $content_data_ty {this.$content_data_mut()}
            }

            $($rest)*
        }
    };
    (
        pub struct $name:ident$(<$generic:ident>)*($event_action_map:ty, $content_data_ty:ty)
                $(where $($where_ty:ty: $($constraint:path)|+),+)*;

        impl $name_impl:ident {
            $(type Map = $map_ty_override:ty;)*
            type Event = $event:ty;

            pub fn $content_data:ident(&self) -> &_;
            pub fn $content_data_mut:ident(&mut self) -> &mut _;
        }

        $($rest:tt)*
    ) => {
        intrinsics!{
            pub struct $name$(<$generic>)*($event_action_map, $content_data_ty)
                    $(where $($where_ty: $($constraint)|+),+)*;

            impl $name_impl {
                $(type Map = $map_ty_override;)*
                type Event = $event;

                pub fn new(event_action_map: $event_action_map, content_data: $content_data_ty) -> Self {
                    $name {
                        wrapper: R::NodeDataWrapper::from_node_data(event_action_map, content_data)
                    }
                }
                pub fn $content_data(&self) -> &_;
                pub fn $content_data_mut(&mut self) -> &mut _;
                pub fn borrow(this: &Self) -> &$event_action_map {this.wrapper.event_map()}
                pub fn borrow_mut(this: &mut Self) -> &mut $event_action_map {this.wrapper.event_map_mut()}
            }

            $($rest)*
        }
    };

    (
        pub struct $name:ident$(<$generic:ident>)*($event_action_map:ty, $content_data_ty:ty)
                $(where $($where_ty:ty: $($constraint:path)|+),+)*;

        impl $name_impl:ident {
            $(type Map = $map_ty_override:ty;)*
            type Event = $event:ty;

            pub fn new($($params:tt)*) -> Self $new_block:block
            pub fn $content_data:ident(&self) -> &_;
            pub fn $content_data_mut:ident(&mut self) -> &mut _;
            pub fn borrow($borrow_param:ident: &Self) -> &$borrow_ty:ty $borrow_block:block
            pub fn borrow_mut($borrow_param_mut:ident: &mut Self) -> &mut $borrow_ty_mut:ty $borrow_block_mut:block
        }

        $($rest:tt)*
    ) => {
        // Just a shorthand for naming the event_action_map type. Rust supports macro overriding, so
        // there's no concern of this "corrupting" future calls to the intrinsics macro.
        macro_rules! event_action_map {
            () => (if_tokens!{($($map_ty_override)*) {$($map_ty_override)*} else {$event_action_map}});
        }

        pub struct $name<$($generic,)* R = NativeWrapperRegistry>
                where $($($where_ty: $($constraint + )+,)*)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<event_action_map!(), ContentData = $content_data_ty>
        {
            wrapper: R::NodeDataWrapper
        }

        impl<$($generic,)* R> $name<$($generic,)* R>
                where $($($where_ty: $($constraint + )+,)*)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<event_action_map!(), ContentData = $content_data_ty>
        {
            pub fn new($($params)*) -> Self $new_block

            pub fn $content_data(&self) -> &$content_data_ty {
                self.wrapper.content_data()
            }

            pub fn $content_data_mut(&mut self) -> &mut $content_data_ty {
                self.wrapper.content_data_mut()
            }

            pub fn unwrap(this: Self) -> (event_action_map!(), $content_data_ty) {
                this.wrapper.unwrap()
            }
        }

        impl<$($generic,)* R> Borrow<$borrow_ty> for $name_impl<$($generic,)* R>
                where $($($where_ty: $($constraint + )+,)*)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<event_action_map!(), ContentData = $content_data_ty>
        {
            fn borrow(&self) -> &$borrow_ty {
                let $borrow_param = self;
                $borrow_block
            }
        }

        impl<$($generic,)* R> BorrowMut<$borrow_ty_mut> for $name_impl<$($generic,)* R>
                where $($($where_ty: $($constraint + )+,)*)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<event_action_map!(), ContentData = $content_data_ty>
        {
            fn borrow_mut(&mut self) -> &mut $borrow_ty_mut {
                let $borrow_param_mut = self;
                $borrow_block_mut
            }
        }

        impl<$($generic,)* R> Deref for $name_impl<$($generic,)* R>
                where $($($where_ty: $($constraint + )+,)*)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<event_action_map!(), ContentData = $content_data_ty>
        {
            type Target = $borrow_ty;
            fn deref(&self) -> &$borrow_ty {
                let $borrow_param = self;
                $borrow_block
            }
        }

        impl<$($generic,)* R> DerefMut for $name_impl<$($generic,)* R>
                where $($($where_ty: $($constraint + )+,)*)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<event_action_map!(), ContentData = $content_data_ty>
        {
            fn deref_mut(&mut self) -> &mut $borrow_ty_mut {
                let $borrow_param_mut = self;
                $borrow_block_mut
            }
        }

        impl<$($generic,)* R> Node for $name_impl<$($generic,)* R>
                where $($($where_ty: $($constraint + )+,)*)*
                      R: NodeDataRegistry<$name<$($generic,)* R>>,
                      R::NodeDataWrapper: NodeDataWrapper<event_action_map!(), ContentData = $content_data_ty>
        {
            type Wrapper = R::NodeDataWrapper;
            type Map = event_action_map!();
            type Event = $event;

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
    pub struct TextButton<E><S>(E, S)
            where E: EventActionMap<MouseEvent>,
                  S: AsRef<str>;
    impl TextButton {
        type Event = MouseEvent;

        pub fn text(&self) -> &_;
        pub fn text_mut(&mut self) -> &mut _;
    }

    pub struct TextLabel<S>((), S)
            where S: AsRef<str>;
    impl TextLabel {
        type Event = !;
        pub fn text(&self) -> &_;
        pub fn text_mut(&mut self) -> &mut _;
    }

    pub struct WidgetGroup<I>((), I)
            where I: Parent<!>;
    impl WidgetGroup {
        type Map = PhantomData<I::ChildAction>;
        type Event = !;
        pub fn parent(&self) -> &_;
        pub fn parent_mut(&mut self) -> &mut _;
    }

    pub struct ProgressBar((), progbar::Status);
    impl ProgressBar {
        type Event = !;
        pub fn status(&self) -> &_;
        pub fn status_mut(&mut self) -> &mut _;
    }

    pub struct Slider<C>(C, slider::Status)
            where C: EventActionMap<RangeEvent>;
    impl Slider {
        type Event = RangeEvent;
        pub fn status(&self) -> &_;
        pub fn status_mut(&mut self) -> &mut _;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Clicked(MouseButton),
    DoubleClicked(MouseButton)
}

pub enum RangeEvent {
    Move(u32),
    Drop(u32)
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
