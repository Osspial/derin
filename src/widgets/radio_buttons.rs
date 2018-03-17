use widgets::{Contents, ContentsInner};
use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};

use core::LoopFlow;
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, WidgetSummary, Widget, Parent};
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;
use dct::layout::{SizeBounds, WidgetPos};

use std::cell::RefCell;

use gl_render::{RelPoint, ThemedPrim, Prim, PrimFrame};
use dle::{GridEngine, UpdateHeapCache, SolveError};
use layout::GridLayout;

#[derive(Debug, Clone)]
pub struct RadioButton {
    pub pressed: bool,
    contents: ContentsInner,
    rect: BoundBox<Point2<i32>>,
    min_size: DimsBox<Point2<i32>>,
}

#[derive(Debug, Clone)]
pub struct RadioButtonList<L>
    where L: GridLayout
{
    update_tag: UpdateTag,
    rect: BoundBox<Point2<i32>>,

    layout_engine: GridEngine,
    buttons: Vec<RadioButton>,
    layout: L
}

impl<L> RadioButtonList<L>
    where L: GridLayout
{
    pub fn new(buttons: Vec<RadioButton>, layout: L) -> RadioButtonList<L> {
        RadioButtonList {
            update_tag: UpdateTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),

            layout_engine: GridEngine::new(),
            buttons, layout
        }
    }

    pub fn buttons(&self) -> &[RadioButton] {
        &self.buttons
    }

    pub fn buttons_mut(&mut self) -> &mut Vec<RadioButton> {
        self.update_tag.mark_render_self();
        &mut self.buttons
    }
}

impl RadioButton {
    pub fn new(pressed: bool, contents: Contents) -> RadioButton {
        RadioButton {
            pressed,
            contents: contents.to_inner(),
            rect: BoundBox::new2(0, 0, 0, 0),
            min_size: DimsBox::new2(0, 0)
        }
    }
}

impl<A, F, L> Widget<A, F> for RadioButtonList<L>
    where F: PrimFrame,
          L: GridLayout
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.rect
    }

    fn size_bounds(&self) -> SizeBounds {
        self.layout_engine.actual_size_bounds()
    }

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        let mut press_found = false;
        for button in self.buttons.iter_mut() {
            button.pressed &= !press_found;
            let image_str = match button.pressed {
                true => {
                    press_found = true;
                    "RadioButton::Pressed"
                },
                false => "RadioButton::Empty"
            };

            let mut content_rect = BoundBox::new2(0, 0, 0, 0);
            frame.upload_primitives(Some(
                ThemedPrim {
                    min: Point2::new(
                        RelPoint::new(-1.0, button.rect.min.x),
                        RelPoint::new(-1.0, button.rect.min.y),
                    ),
                    max: Point2::new(
                        RelPoint::new(-1.0, button.rect.max.x),
                        RelPoint::new(-1.0, button.rect.max.y),
                    ),
                    ..button.contents.to_prim("RadioButton", Some(&mut content_rect))
                }
            ));

            let mut icon_rect = BoundBox::new2(0, 0, 0, 0);
            frame.upload_primitives(Some(
                match content_rect == BoundBox::new2(0, 0, 0, 0) {
                    true => ThemedPrim {
                        min: Point2::new(
                            RelPoint::new(-1.0, 0),
                            RelPoint::new(-1.0, 0),
                        ),
                        max: Point2::new(
                            RelPoint::new( 1.0, 0),
                            RelPoint::new( 1.0, 0)
                        ),
                        prim: Prim::Image,
                        theme_path: image_str,
                        rect_px_out: Some(&mut icon_rect)
                    },
                    false => ThemedPrim {
                        min: Point2::new(
                            RelPoint::new(-1.0, 0),
                            RelPoint::new(-1.0, content_rect.min().y),
                        ),
                        max: Point2::new(
                            RelPoint::new( 1.0, 0),
                            RelPoint::new(-1.0, content_rect.max().y),
                        ),
                        prim: Prim::Image,
                        theme_path: image_str,
                        rect_px_out: Some(&mut icon_rect)
                    }
                }
            ));

            button.min_size = DimsBox::new2(
                content_rect.width() + icon_rect.width(),
                content_rect.height().max(icon_rect.height())
            );
        }
    }

    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
        match event {
            WidgetEvent::MouseUp{in_widget: true, pressed_in_widget: true, pos, down_pos, ..} => {
                let mut old_press_index = None;
                let mut new_press_index = None;
                for (index, button) in self.buttons.iter_mut().enumerate() {
                    if button.pressed {
                        old_press_index = Some(index);
                    }
                    button.pressed = false;
                    if button.rect.contains(pos) && button.rect.contains(down_pos) {
                        button.pressed = true;
                        new_press_index = Some(index);
                        break;
                    }
                }
                if let Some(new_press_index) = new_press_index {
                    for button in self.buttons[new_press_index + 1..].iter_mut() {
                        button.pressed = false;
                    }
                } else if let Some(old_press_index) = old_press_index {
                    self.buttons[old_press_index].pressed = true;
                }

                if new_press_index != old_press_index {
                    self.update_tag.mark_render_self();
                }
            },
            _ => ()
        };


        EventOps {
            action: None,
            focus: None,
            bubble: event.default_bubble(),
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }
}

/// This widget doesn't actually have any children - this is just used for updating the layout.
impl<A, F, L> Parent<A, F> for RadioButtonList<L>
    where F: PrimFrame,
          L: GridLayout
{
    fn num_children(&self) -> usize {
        0
    }

    fn child(&self, _: WidgetIdent) -> Option<WidgetSummary<&Widget<A, F>>> {
        None
    }
    fn child_mut(&mut self, _: WidgetIdent) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        None
    }

    fn children<'a, G, R>(&'a self, _: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a Widget<A, F>>) -> LoopFlow<R>
    {
        None
    }

    fn children_mut<'a, G, R>(&'a mut self, _: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a mut Widget<A, F>>) -> LoopFlow<R>
    {
        None
    }

    fn child_by_index(&self, _: usize) -> Option<WidgetSummary<&Widget<A, F>>> {
        None
    }
    fn child_by_index_mut(&mut self, _: usize) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        None
    }

    fn update_child_layout(&mut self) {
        #[derive(Default)]
        struct HeapCache {
            update_heap_cache: UpdateHeapCache,
            hints_vec: Vec<WidgetPos>,
            rects_vec: Vec<Result<BoundBox<Point2<i32>>, SolveError>>
        }
        thread_local! {
            static HEAP_CACHE: RefCell<HeapCache> = RefCell::new(HeapCache::default());
        }

        HEAP_CACHE.with(|hc| {
            let mut hc = hc.borrow_mut();

            let HeapCache {
                ref mut update_heap_cache,
                ref mut hints_vec,
                ref mut rects_vec
            } = *hc;

            for (index, button) in self.buttons.iter().enumerate() {
                let mut layout_hints = self.layout.positions(WidgetIdent::Num(index as u32), index, self.buttons.len()).unwrap_or(WidgetPos::default());
                layout_hints.size_bounds = SizeBounds {
                    min: button.min_size,
                    ..SizeBounds::default()
                };

                hints_vec.push(layout_hints);
                rects_vec.push(Ok(BoundBox::new2(0, 0, 0, 0)));
            }

            self.layout_engine.desired_size = DimsBox::new(self.rect.dims());
            self.layout_engine.set_grid_size(self.layout.grid_size(self.buttons.len()));
            self.layout_engine.update_engine(hints_vec, rects_vec, update_heap_cache);

            for (button, rect) in self.buttons.iter_mut().zip(rects_vec.drain(..)) {
                button.rect = rect.unwrap_or(BoundBox::new2(0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF));
            }

            hints_vec.clear();
        })
    }
}
