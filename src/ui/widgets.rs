use super::{Node, NodeDataRegistry, NodeDataWrapper, Parent, EventActionMap};
use super::buttons::MouseButton;
use self::content::*;

use std::marker::PhantomData;


macro_rules! intrinsics {
    () => {};

    (
        pub struct $name:ident$(<$generic:ident>)*($map_ty:ty, $content_data_ty:ty)
                $(where $($where_ty:ty: $($constraint:path)|+),+)*;

        impl $name_impl:ident {
            $(type Map = $map_ty_override:ty;)*
            type Event = $event:ty;

            pub fn new($($params:tt)*) -> Self $new_block:block

            $(
                pub
                    $(map fn $map:ident(&self) -> _;)*
                    $(map_mut fn $map_mut:ident(&mut self) -> _;)*
                    $(content fn $content_data:ident(&self) -> _;)*
                    $(content_mut fn $content_data_mut:ident(&mut self) -> _;)*
                    $(fn $borrow:ident($($borrow_params:tt)*) -> $borrow_ty:ty $borrow_block:block)*
            )*
        }

        $($rest:tt)*
    ) => {
        // Just a shorthand for naming the event_action_map type. Rust supports macro overriding, so
        // there's no concern of this "corrupting" future calls to the intrinsics macro.
        macro_rules! event_action_map {
            () => (if_tokens!{($($map_ty_override)*) {$($map_ty_override)*} else {$map_ty}});
        }

        pub struct $name<$($generic,)* R>
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

            $(
                $(
                    pub fn $map(&self) -> &$map_ty {
                        self.wrapper.event_map()
                    }
                )*
                $(
                    pub fn $map_mut(&mut self) -> &mut $map_ty {
                        self.wrapper.event_map_mut()
                    }
                )*
                $(
                    pub fn $content_data(&self) -> &$content_data_ty {
                        self.wrapper.content_data()
                    }
                )*
                $(
                    pub fn $content_data_mut(&mut self) -> &mut $content_data_ty {
                        self.wrapper.content_data_mut()
                    }
                )*
                $(
                    pub fn $borrow($($borrow_params)*) -> $borrow_ty $borrow_block
                )*
            )*

            pub fn unwrap(this: Self) -> (event_action_map!(), $content_data_ty) {
                this.wrapper.unwrap()
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

    (
        pub struct $name:ident$(<$generic:ident>)*((), $content_data_ty:ty)
                $(where $($where_ty:ty: $($constraint:path)|+),+)*;

        impl $name_impl:ident {
            $(type Map = $map_ty_override:ty;)*
            type Event = $event:ty;

            $(
                pub
                    $(map fn $map:ident(&self) -> _;)*
                    $(map_mut fn $map_mut:ident(&mut self) -> _;)*
                    $(content fn $content_data:ident(&self) -> _;)*
                    $(content_mut fn $content_data_mut:ident(&mut self) -> _;)*
                    $(fn $borrow:ident($($borrow_params:tt)*) -> $borrow_ty:ty $borrow_block:block)*
            )*
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

                $(
                    pub
                        $(map fn $map(&self) -> _;)*
                        $(map_mut fn $map_mut(&mut self) -> _;)*
                        $(content fn $content_data(&self) -> _;)*
                        $(content_mut fn $content_data_mut(&mut self) -> _;)*
                        $(fn $borrow($($borrow_params)*) -> $borrow_ty $borrow_block)*
                )*
            }

            $($rest)*
        }
    };
    (
        pub struct $name:ident$(<$generic:ident>)*($map_ty:ty, $content_data_ty:ty)
                $(where $($where_ty:ty: $($constraint:path)|+),+)*;

        impl $name_impl:ident {
            $(type Map = $map_ty_override:ty;)*
            type Event = $event:ty;

            $(
                pub
                    $(map fn $map:ident(&self) -> _;)*
                    $(map_mut fn $map_mut:ident(&mut self) -> _;)*
                    $(content fn $content_data:ident(&self) -> _;)*
                    $(content_mut fn $content_data_mut:ident(&mut self) -> _;)*
                    $(fn $borrow:ident($($borrow_params:tt)*) -> $borrow_ty:ty $borrow_block:block)*
            )*
        }

        $($rest:tt)*
    ) => {
        intrinsics!{
            pub struct $name$(<$generic>)*($map_ty, $content_data_ty)
                    $(where $($where_ty: $($constraint)|+),+)*;

            impl $name_impl {
                $(type Map = $map_ty_override;)*
                type Event = $event;

                pub fn new(event_action_map: $map_ty, content_data: $content_data_ty) -> Self {
                    $name {
                        wrapper: R::NodeDataWrapper::from_node_data(event_action_map, content_data)
                    }
                }
                $(
                    pub
                        $(map fn $map(&self) -> _;)*
                        $(map_mut fn $map_mut(&mut self) -> _;)*
                        $(content fn $content_data(&self) -> _;)*
                        $(content_mut fn $content_data_mut(&mut self) -> _;)*
                        $(fn $borrow($($borrow_params)*) -> $borrow_ty $borrow_block)*
                )*
            }

            $($rest)*
        }
    };
}

intrinsics!{
    pub struct TextButton<E><S>(E, S)
            where E: EventActionMap<MouseEvent>,
                  S: AsRef<str>;
    impl TextButton {
        type Event = MouseEvent;

        pub content fn text(&self) -> _;
        pub content_mut fn text_mut(&mut self) -> _;
        pub map fn map(&self) -> _;
        pub map_mut fn map_mut(&mut self) -> _;
    }

    pub struct TextLabel<S>((), S)
            where S: AsRef<str>;
    impl TextLabel {
        type Event = !;
        pub content fn text(&self) -> _;
        pub content_mut fn text_mut(&mut self) -> _;
    }

    pub struct Group<I>((), I)
            where I: Parent<!>;
    impl Group {
        type Map = PhantomData<I::ChildAction>;
        type Event = !;
        pub content fn children(&self) -> _;
        pub content_mut fn children_mut(&mut self) -> _;
    }

    pub struct GroupBox<S><I>((), GroupBoxContents<S, I>)
            where S: AsRef<str>,
                  I: Parent<!>;
    impl GroupBox {
        type Map = PhantomData<I::ChildAction>;
        type Event = !;

        pub fn new(text: S, children: I) -> Self {
            GroupBox {
                wrapper: R::NodeDataWrapper::from_node_data(PhantomData, GroupBoxContents{text, children})
            }
        }

        pub content fn content(&self) -> _;
        pub content_mut fn content_mut(&mut self) -> _;
        pub fn text(&self) -> &S {
            &self.wrapper.content_data().text
        }
        pub fn text_mut(&mut self) -> &mut S {
            &mut self.wrapper.content_data_mut().text
        }
        pub fn children(&self) -> &I {
            &self.wrapper.content_data().children
        }
        pub fn children_mut(&mut self) -> &mut I {
            &mut self.wrapper.content_data_mut().children
        }
    }

    pub struct Progbar((), ProgbarStatus);
    impl Progbar {
        type Event = !;
        pub content fn status(&self) -> _;
        pub content_mut fn status_mut(&mut self) -> _;
    }

    pub struct Slider<C>(C, SliderStatus)
            where C: EventActionMap<RangeEvent>;
    impl Slider {
        type Event = RangeEvent;
        pub content fn status(&self) -> _;
        pub content_mut fn status_mut(&mut self) -> _;
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

pub mod content {
    use std::ops::Range;
    use std::borrow::{Borrow, BorrowMut};
    use ui::Parent;

    pub struct GroupBoxContents<S, I>
            where S: AsRef<str>,
                  I: Parent<!>
    {
        pub text: S,
        pub children: I
    }

    impl<S, I> Borrow<I> for GroupBoxContents<S, I>
            where S: AsRef<str>,
                  I: Parent<!>
    {
        fn borrow(&self) -> &I {
            &self.children
        }
    }

    impl<S, I> BorrowMut<I> for GroupBoxContents<S, I>
            where S: AsRef<str>,
                  I: Parent<!>
    {
        fn borrow_mut(&mut self) -> &mut I {
            &mut self.children
        }
    }

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

    #[derive(Default, Debug, Clone, Copy, PartialEq)]
    pub struct ProgbarStatus {
        pub completion: Completion,
        pub orientation: Orientation
    }

    impl ProgbarStatus {
        #[inline]
        pub fn new(completion: Completion, orientation: Orientation) -> ProgbarStatus {
            ProgbarStatus {
                completion,
                orientation
            }
        }

        pub fn new_completion(completion: Completion) -> ProgbarStatus {
            ProgbarStatus {
                completion,
                orientation: Orientation::default()
            }
        }

        pub fn new_orientation(orientation: Orientation) -> ProgbarStatus {
            ProgbarStatus {
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

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderStatus {
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

    impl Default for SliderStatus {
        fn default() -> SliderStatus {
            SliderStatus {
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
