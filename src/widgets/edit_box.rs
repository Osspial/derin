use core::event::{EventOps, NodeEvent, InputState, FocusChange};
use core::tree::{NodeIdent, UpdateTag, NodeSubtrait, NodeSubtraitMut, Node};
use core::render::{FrameRectStack, Theme};
use core::popup::ChildPopupsMut;
use core::timer::TimerRegister;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox, Segment};
use dct::layout::SizeBounds;
use dct::cursor::CursorIcon;
use dct::buttons::{Key, ModifierKeys};

use gl_render::{ThemedPrim, PrimFrame, RenderString, EditString, RelPoint, Prim};

use std::cell::Cell;
use std::time::Duration;

use clipboard::{ClipboardContext, ClipboardProvider};

#[derive(Debug, Clone)]
pub struct EditBox {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    string: EditString,
    size_bounds: Cell<SizeBounds>
}

impl EditBox {
    pub fn new(string: String) -> EditBox {
        EditBox {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            string: EditString::new(RenderString::new(string)),
            size_bounds: Cell::new(SizeBounds::default())
        }
    }

    pub fn string(&self) -> &str {
        self.string.render_string.string()
    }

    pub fn string_mut(&mut self) -> &mut String {
        self.update_tag.mark_render_self();
        self.string.render_string.string_mut()
    }
}

impl<A, F> Node<A, F> for EditBox
    where F: PrimFrame
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

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds.get()
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives([
            ThemedPrim {
                theme_path: "EditBox",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image
            },
            ThemedPrim {
                theme_path: "EditBox",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::EditString(&self.string)
            }
        ].iter().cloned());

        let mut size_bounds = self.size_bounds.get();
        size_bounds.min = frame.theme().node_theme("EditBox").icon.map(|i| i.min_size()).unwrap_or(DimsBox::new2(0, 0));
        let render_string_min = self.string.render_string.min_size();
        size_bounds.min.dims.y += render_string_min.height();
        self.size_bounds.set(size_bounds);
    }

    fn on_node_event(&mut self, event: NodeEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[NodeIdent]) -> EventOps<A, F> {
        use self::NodeEvent::*;
        use dct::buttons::MouseButton;

        let allow_char = |c| match c {
            '\t' |
            '\r' |
            '\n' => true,
            _ => !c.is_control()
        };
        let mut focus = None;
        let mut cursor_icon = None;
        match event {
            KeyDown(key, modifiers) => loop {
                let jump_to_word_boundaries = modifiers.contains(ModifierKeys::CTRL);
                match (key, modifiers) {
                    (Key::LArrow, _) => self.string.move_cursor_horizontal(
                        -1,
                        jump_to_word_boundaries,
                        modifiers.contains(ModifierKeys::SHIFT)
                    ),
                    (Key::RArrow, _) => self.string.move_cursor_horizontal(
                        1,
                        jump_to_word_boundaries,
                        modifiers.contains(ModifierKeys::SHIFT)
                    ),
                    (Key::UArrow, _) => self.string.move_cursor_vertical(-1, modifiers.contains(ModifierKeys::SHIFT)),
                    (Key::DArrow, _) => self.string.move_cursor_vertical(1, modifiers.contains(ModifierKeys::SHIFT)),
                    (Key::A, ModifierKeys::CTRL) => self.string.select_all(),
                    (Key::C, ModifierKeys::CTRL) => {
                        if let Ok(mut clipboard) = ClipboardContext::new() {
                            let select_range = self.string.highlight_range();
                            clipboard.set_contents(self.string.render_string.string()[select_range].to_string()).ok();
                        }
                    },
                    (Key::V, ModifierKeys::CTRL) => {
                        if let Ok(clipboard_conents) = ClipboardContext::new().and_then(|mut c| c.get_contents()) {
                            self.string.insert_str(&clipboard_conents);
                        }
                    },
                    (Key::X, ModifierKeys::CTRL) => {
                        if let Ok(mut clipboard) = ClipboardContext::new() {
                            let highlight_range = self.string.highlight_range();
                            clipboard.set_contents(self.string.render_string.string()[highlight_range.clone()].to_string()).ok();
                            if highlight_range.len() > 0 {
                                self.string.delete_chars(1, false);
                            }
                        }
                    },
                    (Key::Back, _) => self.string.delete_chars(-1, jump_to_word_boundaries),
                    (Key::Delete, _) => self.string.delete_chars(1, jump_to_word_boundaries),
                    _ => break
                }
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
                break;
            },
            Char(c) if allow_char(c) => {
                self.string.insert_char(c);
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
            }
            MouseDown{in_node: true, button, pos} => {
                focus = Some(FocusChange::Take);
                if button == MouseButton::Left {
                    self.string.select_on_line(Segment::new(pos, pos));
                    self.update_tag
                        .mark_render_self()
                        .mark_update_timer();
                }
            },
            MouseUp{button: MouseButton::Left, ..} => {
                self.update_tag.mark_render_self();
            }
            MouseDown{in_node: false, ..} => {
                focus = Some(FocusChange::Remove);
                self.string.draw_cursor = false;
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
            },
            MouseMove{new_pos, buttons_down_in_node, ..} => {
                if let Some(down) = buttons_down_in_node.iter().find(|d| d.button == MouseButton::Left) {
                    self.string.select_on_line(Segment::new(down.down_pos, new_pos));
                    self.update_tag.mark_render_self();
                }
            },
            MouseEnter{..} => cursor_icon = Some(CursorIcon::Text),
            MouseExit{..} => cursor_icon = Some(CursorIcon::default()),
            GainFocus  |
            LoseFocus => {
                self.string.deselect_all();
                self.update_tag.mark_update_timer();
            },
            Timer{name: "cursor_flash", times_triggered, ..} => {
                self.string.draw_cursor = times_triggered % 2 == 0;
                self.update_tag.mark_render_self();
            },
            _ => ()
        };
        EventOps {
            action: None,
            focus,
            bubble: true,
            cursor_pos: None,
            cursor_icon,
            popup: None
        }
    }

    fn register_timers(&self, register: &mut TimerRegister) {
        if self.update_tag.has_keyboard_focus() {
            register.add_timer("cursor_flash", Duration::new(1, 0)/2, true);
        }
    }

    #[inline]
    fn subtrait(&self) -> NodeSubtrait<A, F> {
        NodeSubtrait::Node(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> NodeSubtraitMut<A, F> {
        NodeSubtraitMut::Node(self)
    }
}
