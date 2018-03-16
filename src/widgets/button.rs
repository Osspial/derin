use widgets::{Contents, ContentsInner};
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, Widget};
use core::render::{FrameRectStack, Theme};
use core::popup::ChildPopupsMut;
use core::timer::TimerRegister;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};
use dct::layout::SizeBounds;

use gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

use std::time::Duration;

use arrayvec::ArrayVec;

pub trait ButtonHandler {
    type Action: 'static;

    fn on_click(&mut self) -> Option<Self::Action>;
}

impl<A: 'static + Clone> ButtonHandler for Option<A> {
    type Action = A;

    fn on_click(&mut self) -> Option<Self::Action> {
        self.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    Normal,
    Hover,
    Clicked,
    // Disabled,
    // Defaulted
}

#[derive(Debug, Clone)]
pub struct Button<H: ButtonHandler> {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    state: ButtonState,
    handler: H,
    contents: ContentsInner,
    waiting_for_mouseover: bool,
    size_bounds: SizeBounds
}

impl<H: ButtonHandler> Button<H> {
    pub fn new(contents: Contents<String>, handler: H) -> Button<H> {
        Button {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            state: ButtonState::Normal,
            handler,
            contents: contents.to_inner(),
            waiting_for_mouseover: false,
            size_bounds: SizeBounds::default()
        }
    }

    pub fn contents(&self) -> Contents<&str> {
        self.contents.borrow()
    }

    pub fn contents_mut(&mut self) -> Contents<&mut String> {
        self.update_tag.mark_render_self();
        self.contents.borrow_mut()
    }
}

impl<F, H> Widget<H::Action, F> for Button<H>
    where F: PrimFrame,
          H: ButtonHandler
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.bounds
    }

    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds
    }

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        let image_str = match self.state {
            ButtonState::Normal    => "Button::Normal",
            ButtonState::Hover     => "Button::Hover",
            ButtonState::Clicked   => "Button::Clicked",
            // ButtonState::Disabled  => "Button::Disabled",
            // ButtonState::Defaulted => "Button::Defaulted"
        };

        frame.upload_primitives(ArrayVec::from([
            ThemedPrim {
                theme_path: image_str,
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image,
                rect_px_out: None
            },
            self.contents.to_prim(image_str, None)
        ]).into_iter());

        self.size_bounds.min = frame.theme().widget_theme(image_str).image.map(|i| i.min_size()).unwrap_or(DimsBox::new2(0, 0));
        let render_string_min = self.contents.min_size(frame.theme());
        self.size_bounds.min.dims.x += render_string_min.width();
        self.size_bounds.min.dims.y += render_string_min.height();
    }

    fn register_timers(&self, register: &mut TimerRegister) {
        if self.waiting_for_mouseover {
            register.add_timer("mouseover_text", Duration::new(1, 0)/2, true);
        }
    }

    fn on_widget_event(&mut self, event: WidgetEvent, input_state: InputState, popups_opt: Option<ChildPopupsMut<H::Action, F>>, bubble_source: &[WidgetIdent]) -> EventOps<H::Action, F> {
        use self::WidgetEvent::*;

        let (mut action, focus) = (None, None);
        let mut popup = None;

        if bubble_source.len() == 0 {
            if let Some(mut popups) = popups_opt {
                // Remove mouseover text, if it exists
                match event {
                    MouseEnter{..} |
                    MouseExit{..} |
                    MouseMove{..} |
                    MouseDown{..} => {
                        popups.remove(WidgetIdent::Str("mouseover_text"));
                    },
                    _ => ()
                }
            }

            let new_state = match event {
                MouseEnter{..} |
                MouseExit{..} => {
                    self.waiting_for_mouseover = false;
                    self.update_tag.mark_update_timer();

                    match (input_state.mouse_buttons_down_in_widget.is_empty(), event) {
                        (true, MouseEnter{..}) => ButtonState::Hover,
                        (true, MouseExit{..}) => ButtonState::Normal,
                        (false, _) => self.state,
                        _ => unreachable!()
                    }
                },
                MouseMove{..} => {
                    self.waiting_for_mouseover = true;
                    self.update_tag.mark_update_timer();
                    self.state
                },
                MouseDown{..} => {
                    self.update_tag.mark_update_timer();
                    ButtonState::Clicked
                },
                MouseUp{in_widget: true, pressed_in_widget, ..} => {
                    match pressed_in_widget {
                        true => {
                            action = self.handler.on_click();
                            ButtonState::Hover
                        },
                        false => self.state
                    }
                },
                MouseUp{in_widget: false, ..} => ButtonState::Normal,
                MouseEnterChild{..} |
                MouseExitChild{..} => unreachable!(),
                GainFocus => ButtonState::Hover,
                LoseFocus => ButtonState::Normal,
                Timer{name: "mouseover_text", times_triggered: 1, ..} => {
                    self.waiting_for_mouseover = false;
                    self.update_tag.mark_update_timer();
                    // popup = Some((
                    //     Box::new(Group::new(SingleContainer::new(Label::new("Hello Popup!".to_string())), LayoutHorizontal::default())) as Box<Widget<_, F>>,
                    //     ::core::popup::PopupAttributes {
                    //         rect: BoundBox::new2(1, 1, 129, 129) + input_state.mouse_pos.to_vec(),
                    //         title: "".to_string(),
                    //         decorations: false,
                    //         tool_window: true,
                    //         focusable: false,
                    //         ident: WidgetIdent::Str("mouseover_text")
                    //     }
                    // ));
                    self.state
                },
                _ => self.state
            };

            if new_state != self.state {
                self.update_tag.mark_render_self();
                self.state = new_state;
            }
        }


        EventOps {
            action, focus,
            bubble: event.default_bubble(),
            cursor_pos: None,
            cursor_icon: None,
            popup
        }
    }
}
