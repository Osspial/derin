// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    container::WidgetContainer,
    core::{
        LoopFlow,
        event::{EventOps, WidgetEvent, WidgetEventSourced, InputState, MouseHoverChange},
        widget::{WidgetIdent, WidgetRender, WidgetTag, WidgetInfo, WidgetInfoMut, Widget, Parent},
        render::{RenderFrame, RenderFrameClipped},
        render::Theme as CoreTheme,
    },
    gl_render::{RelPoint, ThemedPrim, Prim, PrimFrame},
    layout::GridLayout,
    widgets::assistants::ButtonState,
    widgets::{Contents, ContentsInner},
};

use derin_common_types::layout::{SizeBounds, WidgetPos};

use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};
use std::cell::RefCell;

use derin_layout_engine::{GridEngine, UpdateHeapCache, SolveError};

/// A radio button widget.
///
/// Generally is only useful alongside other radio buttons, as an individual radio button can only
/// be disabled by clicking a sibling radio button. Other radio buttons can be set as siblings
/// with the [`RadioButtonList`] widget.
///
/// [`RadioButtonList`]: ./struct.RadioButtonList.html
#[derive(Debug, Clone)]
pub struct RadioButton {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
    min_size: DimsBox<D2, i32>,

    selected: bool,
    button_state: ButtonState,
    contents: ContentsInner,
}

/// A set of radio buttons.
///
/// Used to define a set of linked radio buttons which disable eachother when selected.
#[derive(Debug, Clone)]
pub struct RadioButtonList<C, L>
    where L: GridLayout
{
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,

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
            widget_tag: WidgetTag::new(),
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
        &mut self.buttons
    }
}

impl RadioButton {
    /// Creates a new radio button, with the given default selected state and contents.
    pub fn new(selected: bool, contents: Contents) -> RadioButton {
        RadioButton {
            widget_tag: WidgetTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            min_size: DimsBox::new2(0, 0),

            selected,
            button_state: ButtonState::Normal,
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
        self.widget_tag.request_redraw();
        self.contents.borrow_mut()
    }

    /// Retrieves whether or not the radio button is selected.
    pub fn selected(&self) -> bool {
        self.selected
    }

    /// Retrieves whether or not the radio button is selected, for mutation.
    ///
    /// Calling this function forces the radio button to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn selected_mut(&mut self) -> &mut bool {
        self.widget_tag.request_redraw();
        &mut self.selected
    }
}

impl Widget for RadioButton {
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        &mut self.rect
    }

    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::new_min(self.min_size)
    }

    fn on_widget_event(&mut self, event: WidgetEventSourced, input_state: InputState) -> EventOps {
        use self::WidgetEvent::*;
        let event = event.unwrap();

        let mut force_bubble = false;
        let (mut new_selected, mut new_state) = (self.selected, self.button_state);
        match event {
            MouseMove{hover_change: Some(ref change), ..} if input_state.mouse_buttons_down_in_widget.is_empty() => {
                match change {
                    MouseHoverChange::Enter => new_state = ButtonState::Hover,
                    MouseHoverChange::Exit => new_state = ButtonState::Normal,
                    _ => ()
                }
            },
            MouseDown{..} => new_state = ButtonState::Pressed,
            MouseUp{in_widget: true, pressed_in_widget: true, ..} => {
                // if !self.selected {
                //     action = self.handler.change_state(!self.selected);
                // }
                force_bubble = true;
                new_selected = true;
                new_state = ButtonState::Hover;
            },
            MouseUp{in_widget: false, ..} => new_state = ButtonState::Normal,
            GainFocus(_, _) => new_state = ButtonState::Hover,
            LoseFocus => new_state = ButtonState::Normal,
            _ => ()
        };

        if new_selected != self.selected || new_state != self.button_state {
            self.widget_tag.request_redraw();
            self.selected = new_selected;
            self.button_state = new_state;
        }



        EventOps {
            focus: None,
            bubble: force_bubble || event.default_bubble(),
        }
    }
}

impl<C, L> Widget for RadioButtonList<C, L>
    where C: WidgetContainer<RadioButton>,
          L: GridLayout
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        self.widget_tag.request_relayout();
        &mut self.rect
    }

    fn size_bounds(&self) -> SizeBounds {
        self.layout_engine.actual_size_bounds()
    }

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEventSourced, _: InputState) -> EventOps {
        // TODO: PASS FOCUS TO CHILD

        let mut bubble = true;
        // Un-select any child radio buttons after a new button was selected.
        if let WidgetEventSourced::Bubble(WidgetEvent::MouseUp{in_widget: true, pressed_in_widget: true, ..}, [child_ident]) = event {
            bubble = false;
            let mut state_changed = false;
            self.buttons.children_mut::<_>(|mut summary| {
                if summary.ident != *child_ident {
                    let radio_button = summary.subtype_mut();
                    if radio_button.selected {
                        radio_button.widget_tag.request_redraw();
                    }
                    state_changed |= radio_button.selected;
                    radio_button.selected = false;
                }
                LoopFlow::Continue
            });
        }

        EventOps {
            focus: None,
            bubble,
        }
    }
}

impl<C, L> Parent for RadioButtonList<C, L>
    where C: WidgetContainer<RadioButton>,
          L: GridLayout
{
    fn num_children(&self) -> usize {
        self.buttons.num_children()
    }

    fn framed_child<F: RenderFrame>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, F>> {
        self.buttons.framed_child(widget_ident).map(WidgetInfo::erase_subtype)
    }
    fn framed_child_mut<F: RenderFrame>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, F>> {
        self.buttons.framed_child_mut(widget_ident).map(WidgetInfoMut::erase_subtype)
    }

    fn framed_children<'a, F, G>(&'a self, mut for_each: G)
        where F: RenderFrame,
              G: FnMut(WidgetInfo<'a, F>) -> LoopFlow
    {
        self.buttons.framed_children(|summary| for_each(WidgetInfo::erase_subtype(summary)))
    }

    fn framed_children_mut<'a, F, G>(&'a mut self, mut for_each: G)
        where F: RenderFrame,
              G: FnMut(WidgetInfoMut<'a, F>) -> LoopFlow
    {
        self.buttons.framed_children_mut(|summary| for_each(WidgetInfoMut::erase_subtype(summary)))
    }

    fn framed_child_by_index<F: RenderFrame>(&self, index: usize) -> Option<WidgetInfo<'_, F>> {
        self.buttons.framed_child_by_index(index).map(WidgetInfo::erase_subtype)
    }
    fn framed_child_by_index_mut<F: RenderFrame>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, F>> {
        self.buttons.framed_child_by_index_mut(index).map(WidgetInfoMut::erase_subtype)
    }
}

impl<F> WidgetRender<F> for RadioButton
    where F: PrimFrame
{
    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        let image_str = match (self.selected, self.button_state) {
            (true, ButtonState::Normal) => "RadioButton::Selected",
            (true, ButtonState::Hover) => "RadioButton::Selected::Hover",
            (true, ButtonState::Pressed) => "RadioButton::Selected::Pressed",
            (false, ButtonState::Normal) => "RadioButton::Empty",
            (false, ButtonState::Hover) => "RadioButton::Empty::Hover",
            (false, ButtonState::Pressed) => "RadioButton::Empty::Pressed",
        };
        let icon_min_size = frame.theme().widget_theme(image_str).image.map(|b| b.size_bounds.min).unwrap_or(DimsBox::new2(0, 0));

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
                    theme_path: image_str,
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
                        theme_path: image_str,
                        rect_px_out: Some(&mut icon_rect)
                    }
                }
            }
        ));

        // TODO: MOVE TO update_layout
        self.min_size = DimsBox::new2(
            content_rect.width() + icon_rect.width(),
            icon_min_size.height().max(self.contents.min_size(frame.theme()).height())
        );
    }
}

impl<F, C, L> WidgetRender<F> for RadioButtonList<C, L>
    where F: PrimFrame,
          C: WidgetContainer<RadioButton>,
          L: GridLayout
{
    fn render(&mut self, _: &mut RenderFrameClipped<F>) {}

    fn update_layout(&mut self, _: &F::Theme) {
        #[derive(Default)]
        struct HeapCache {
            update_heap_cache: UpdateHeapCache,
            hints_vec: Vec<WidgetPos>,
            rects_vec: Vec<Result<BoundBox<D2, i32>, SolveError>>
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
            self.buttons.children::<_>(|summary| {
                let widget_size_bounds = summary.widget().size_bounds();
                let mut layout_hints = self.layout.positions(summary.ident, summary.index, num_children).unwrap_or(WidgetPos::default());
                layout_hints.size_bounds = SizeBounds {
                    min: layout_hints.size_bounds.bound_rect(widget_size_bounds.min),
                    max: layout_hints.size_bounds.bound_rect(widget_size_bounds.max),
                };
                hints_vec.push(layout_hints);
                rects_vec.push(Ok(BoundBox::new2(0, 0, 0, 0)));
                LoopFlow::Continue
            });

            self.layout_engine.desired_size = self.rect.dims();
            self.layout_engine.set_grid_size(self.layout.grid_size(num_children));
            self.layout_engine.update_engine(hints_vec, rects_vec, update_heap_cache);

            let mut rects_iter = rects_vec.drain(..);
            self.buttons.children_mut::<_>(|mut summary| {
                match rects_iter.next() {
                    Some(rect) => *summary.widget_mut().rect_mut() = rect.unwrap_or(BoundBox::new2(0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF)),
                    None => return LoopFlow::Break
                }

                LoopFlow::Continue
            });

            hints_vec.clear();
        })
    }
}
