// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use container::WidgetContainer;
use widgets::{Contents, ContentsInner};
use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};

use core::LoopFlow;
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, WidgetSummary, Widget, Parent, OnFocus};
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;
use core::render::Theme as CoreTheme;
use dct::layout::{SizeBounds, WidgetPos};

use std::cell::RefCell;

use gl_render::{RelPoint, ThemedPrim, Prim, PrimFrame};
use dle::{GridEngine, UpdateHeapCache, SolveError};
use layout::GridLayout;

/// A radio button widget.
///
/// Generally is only useful alongside other radio buttons, as an individual radio button can only
/// be disabled by clicking a sibling radio button. Other radio buttons can be set as siblings
/// with the [`RadioButtonList`] widget.
///
/// [`RadioButtonList`]: ./struct.RadioButtonList.html
#[derive(Debug, Clone)]
pub struct RadioButton {
    update_tag: UpdateTag,
    rect: BoundBox<Point2<i32>>,
    min_size: DimsBox<Point2<i32>>,

    pressed: bool,
    contents: ContentsInner,
}

/// A set of radio buttons.
///
/// Used to define a set of linked radio buttons which disable eachother when pressed.
#[derive(Debug, Clone)]
pub struct RadioButtonList<C, L>
    where L: GridLayout
{
    update_tag: UpdateTag,
    rect: BoundBox<Point2<i32>>,

    layout_engine: GridEngine,
    buttons: C,
    layout: L
}

impl<C, L> RadioButtonList<C, L>
    where L: GridLayout
{
    /// Takes a collection of radio buttons, as well as the layout in which to place those buttons.
    ///
    /// The passed collection can *only contain radio buttons*, otherwise this will fail to compile.
    pub fn new(buttons: C, layout: L) -> RadioButtonList<C, L> {
        RadioButtonList {
            update_tag: UpdateTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),

            layout_engine: GridEngine::new(),
            buttons, layout
        }
    }

    /// Retrieves the collection of radio buttons stored within this list.
    pub fn buttons(&self) -> &C {
        &self.buttons
    }

    /// Retrieves the collection of radio buttons stored within this list, for mutation.
    pub fn buttons_mut(&mut self) -> &mut C {
        self.update_tag.mark_update_child().mark_update_layout();
        &mut self.buttons
    }
}

impl RadioButton {
    /// Creates a new radio button, with the given default pressed state and contents.
    pub fn new(pressed: bool, contents: Contents) -> RadioButton {
        RadioButton {
            update_tag: UpdateTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            min_size: DimsBox::new2(0, 0),

            pressed,
            contents: contents.to_inner(),
        }
    }

    /// Retrieves the contents of the radio button.
    pub fn contents(&self) -> Contents<&str> {
        self.contents.borrow()
    }

    /// Retrieves the contents of the radio button, for mutation.
    ///
    /// Calling this function forces the radio button to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> Contents<&mut String> {
        self.update_tag.mark_render_self();
        self.contents.borrow_mut()
    }

    /// Retrieves whether or not the radio button is pressed.
    pub fn pressed(&self) -> bool {
        self.pressed
    }

    /// Retrieves whether or not the radio button is pressed, for mutation.
    ///
    /// Calling this function forces the radio button to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn pressed_mut(&mut self) -> &mut bool {
        self.update_tag.mark_render_self();
        &mut self.pressed
    }
}

impl<A, F> Widget<A, F> for RadioButton
    where F: PrimFrame
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
        SizeBounds::new_min(self.min_size)
    }

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        let icon_str = match self.pressed {
            true => "RadioButton::Pressed",
            false => "RadioButton::Empty"
        };
        let icon_min_size = frame.theme().widget_theme(icon_str).image.map(|b| b.size_bounds.min).unwrap_or(DimsBox::new2(0, 0));

        let mut content_rect = BoundBox::new2(0, 0, 0, 0);
        frame.upload_primitives(Some(
            ThemedPrim {
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0),
                ),
                ..self.contents.to_prim("RadioButton", Some(&mut content_rect))
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
                    theme_path: icon_str,
                    rect_px_out: Some(&mut icon_rect)
                },
                false => {
                    ThemedPrim {
                        min: Point2::new(
                            RelPoint::new(-1.0, 0),
                            RelPoint::new(-1.0, content_rect.min.y),
                        ),
                        max: Point2::new(
                            RelPoint::new( 1.0, 0),
                            RelPoint::new(-1.0, content_rect.min.y + icon_min_size.height()),
                        ),
                        prim: Prim::Image,
                        theme_path: icon_str,
                        rect_px_out: Some(&mut icon_rect)
                    }
                }
            }
        ));

        self.min_size = DimsBox::new2(
            content_rect.width() + icon_rect.width(),
            icon_min_size.height().max(self.contents.min_size(frame.theme()).height())
        );
    }

    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
        let mut force_bubble = false;
        match event {
            WidgetEvent::MouseUp{in_widget: true, pressed_in_widget: true, ..} => {
                force_bubble = true;
                self.pressed = true;
                self.update_tag.mark_render_self();
            },
            _ => ()
        };


        EventOps {
            action: None,
            focus: None,
            bubble: force_bubble || event.default_bubble(),
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }
}

impl<A, F, C, L> Widget<A, F> for RadioButtonList<C, L>
    where A: 'static,
          F: PrimFrame,
          C: WidgetContainer<A, F, Widget=RadioButton>,
          RadioButton: Widget<A, F>,
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
        self.update_tag.mark_update_layout();
        &mut self.rect
    }

    fn size_bounds(&self) -> SizeBounds {
        self.layout_engine.actual_size_bounds()
    }

    fn render(&mut self, _: &mut FrameRectStack<F>) {}

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, bubble_source: &[WidgetIdent]) -> EventOps<A, F> {
        let mut bubble = true;
        // Un-select any child radio buttons after a new button was pressed.
        if let (Some(child_ident), WidgetEvent::MouseUp{in_widget: true, pressed_in_widget: true, ..}) = (bubble_source.get(0), event) {
            bubble = false;
            let mut state_changed = false;
            self.buttons.children_mut::<_, ()>(|summary| {
                if summary.ident != *child_ident {
                    if summary.widget.pressed {
                        summary.widget.update_tag.mark_render_self();
                    }
                    state_changed |= summary.widget.pressed;
                    summary.widget.pressed = false;
                }
                LoopFlow::Continue
            });
            self.update_tag.mark_update_child();
        }

        EventOps {
            action: None,
            focus: None,
            bubble,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }

    fn accepts_focus(&self) -> OnFocus {
        OnFocus::FocusChild
    }
}

impl<A, F, C, L> Parent<A, F> for RadioButtonList<C, L>
    where A: 'static,
          F: PrimFrame,
          C: WidgetContainer<A, F, Widget=RadioButton>,
          RadioButton: Widget<A, F>,
          L: GridLayout
{
    fn num_children(&self) -> usize {
        self.buttons.num_children()
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<A, F>>> {
        self.buttons.child(widget_ident).map(WidgetSummary::to_dyn)
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        self.buttons.child_mut(widget_ident).map(WidgetSummary::to_dyn_mut)
    }

    fn children<'a, G, R>(&'a self, mut for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a Widget<A, F>>) -> LoopFlow<R>
    {
        self.buttons.children(|summary| for_each(WidgetSummary::to_dyn(summary)))
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a mut Widget<A, F>>) -> LoopFlow<R>
    {
        self.buttons.children_mut(|summary| for_each(WidgetSummary::to_dyn_mut(summary)))
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<A, F>>> {
        self.buttons.child_by_index(index).map(WidgetSummary::to_dyn)
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        self.buttons.child_by_index_mut(index).map(WidgetSummary::to_dyn_mut)
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

            let num_children = self.num_children();
            self.buttons.children::<_, ()>(|summary| {
                let mut layout_hints = self.layout.positions(summary.ident, summary.index, num_children).unwrap_or(WidgetPos::default());
                layout_hints.size_bounds = SizeBounds {
                    min: layout_hints.size_bounds.bound_rect(summary.size_bounds.min),
                    max: layout_hints.size_bounds.bound_rect(summary.size_bounds.max),
                };
                hints_vec.push(layout_hints);
                rects_vec.push(Ok(BoundBox::new2(0, 0, 0, 0)));
                LoopFlow::Continue
            });

            self.layout_engine.desired_size = DimsBox::new(self.rect.dims());
            self.layout_engine.set_grid_size(self.layout.grid_size(num_children));
            self.layout_engine.update_engine(hints_vec, rects_vec, update_heap_cache);

            let mut rects_iter = rects_vec.drain(..);
            self.buttons.children_mut::<_, ()>(|summary| {
                match rects_iter.next() {
                    Some(rect) => *summary.widget.rect_mut() = rect.unwrap_or(BoundBox::new2(0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF)),
                    None => return LoopFlow::Break(())
                }

                LoopFlow::Continue
            });

            hints_vec.clear();
        })
    }
}
