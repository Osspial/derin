// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    LoopFlow,
    event::{EventOps, WidgetEventSourced, InputState},
    widget::{WidgetIdent, WidgetRenderable, WidgetTag, WidgetInfo, WidgetInfoMut, WidgetId, Widget, Parent},
    render::{DisplayEngine, SubFrame},
};
use crate::{
    container::WidgetContainer,
    layout::GridLayout,
    widgets::{
        Content,
        assistants::toggle_button::{Toggle, ToggleOnClickHandler},
    },
};

use derin_common_types::layout::{SizeBounds, WidgetPos};

use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};
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
    toggle: Toggle<RadioButtonToggleHandler, RadioButtonTheme>,
}

#[derive(Default, Debug, Clone)]
pub struct RadioButtonTheme(());

#[derive(Default, Debug, Clone)]
pub struct RadioButtonListTheme(());

#[derive(Debug, Clone, Copy)]
struct RadioButtonToggleHandler;
impl ToggleOnClickHandler for RadioButtonToggleHandler {
    fn on_click(&mut self, checked: &mut bool) {
        *checked = true;
        panic!("CALL https://github.com/Osspial/derin/blob/6a92c83b9cfa73024b4216c5d5e1aee74ee513b1/derin/src/widgets/radio_buttons.rs#L184-L187");
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RadioButtonSelected(WidgetId);

impl<C, L> RadioButtonList<C, L>
    where C: WidgetContainer<RadioButton>,
          L: GridLayout
{
    /// Takes a collection of radio buttons, as well as the layout in which to place those buttons.
    ///
    /// The passed collection can *only contain radio buttons*, otherwise this will fail to compile.
    pub fn new(buttons: C, layout: L) -> RadioButtonList<C, L> {
        let mut widget_tag = WidgetTag::new();
        widget_tag.register_message(Self::on_child_selected);
        RadioButtonList {
            widget_tag,
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

    fn on_child_selected(&mut self, child_selected: &RadioButtonSelected) {
        self.buttons.children_mut(|mut child_info| {
            let child_radio_button = child_info.subtype_mut();
            if child_radio_button.widget_id() != child_selected.0 && child_radio_button.selected() {
                *child_radio_button.selected_mut() = false;
            }

            LoopFlow::Continue
        });
    }
}

impl RadioButton {
    /// Creates a new radio button, with the given default selected state and contents.
    pub fn new(selected: bool, contents: Content) -> RadioButton {
        RadioButton {
            toggle: Toggle::new(selected, contents, RadioButtonToggleHandler, RadioButtonTheme(())),
        }
    }


    /// Retrieves the contents of the radio button.
    pub fn contents(&self) -> &Content {
        self.toggle.contents()
    }

    /// Retrieves the contents of the radio button, for mutation.
    ///
    /// Calling this function forces the radio button to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> &mut Content {
        self.toggle.contents_mut()
    }

    /// Retrieves whether or not the radio button is checked.
    pub fn selected(&self) -> bool {
        self.toggle.selected()
    }

    /// Retrieves whether or not the radio button is selected, for mutation.
    ///
    /// Calling this function forces the radio button to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn selected_mut(&mut self) -> &mut bool {
        self.toggle.selected_mut()
    }
}

impl Widget for RadioButton {
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        self.toggle.widget_tag()
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.toggle.rect()
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        self.toggle.rect_mut()
    }

    fn size_bounds(&self) -> SizeBounds {
        self.toggle.size_bounds()
    }

    fn on_widget_event(&mut self, event: WidgetEventSourced, state: InputState) -> EventOps {
        self.toggle.on_widget_event(event, state)
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
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        // TODO: PASS FOCUS TO CHILD

        EventOps {
            focus: None,
            bubble: true,
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

    fn framed_child<R: Renderer>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, R>> {
        self.buttons.framed_child(widget_ident).map(WidgetInfo::erase_subtype)
    }
    fn framed_child_mut<R: Renderer>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, R>> {
        self.buttons.framed_child_mut(widget_ident).map(WidgetInfoMut::erase_subtype)
    }

    fn framed_children<'a, R, G>(&'a self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfo<'a, R>) -> LoopFlow
    {
        self.buttons.framed_children(|summary| for_each(WidgetInfo::erase_subtype(summary)))
    }

    fn framed_children_mut<'a, R, G>(&'a mut self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfoMut<'a, R>) -> LoopFlow
    {
        self.buttons.framed_children_mut(|summary| for_each(WidgetInfoMut::erase_subtype(summary)))
    }

    fn framed_child_by_index<R: Renderer>(&self, index: usize) -> Option<WidgetInfo<'_, R>> {
        self.buttons.framed_child_by_index(index).map(WidgetInfo::erase_subtype)
    }
    fn framed_child_by_index_mut<R: Renderer>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, R>> {
        self.buttons.framed_child_by_index_mut(index).map(WidgetInfoMut::erase_subtype)
    }
}

impl<R> WidgetRenderable<R> for RadioButton
    where R: Renderer,
{
    type Theme = RadioButtonTheme;

    fn theme(&self) -> RadioButtonTheme {
        WidgetRenderable::<R>::theme(&self.toggle)
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        WidgetRenderable::<R>::render(&mut self.toggle, frame)
    }

    fn update_layout(&mut self, l: &mut R::Layout) {
        WidgetRenderable::<R>::update_layout(&mut self.toggle, l)
    }
}

impl<R, C, L> WidgetRenderable<R> for RadioButtonList<C, L>
    where R: Renderer,
          C: WidgetContainer<RadioButton>,
          L: GridLayout
{
    type Theme = RadioButtonListTheme;

    fn theme(&self) -> RadioButtonListTheme {
        RadioButtonListTheme(())
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, _: &mut R::Layout) {
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

impl WidgetTheme for RadioButtonTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {None}
}

impl WidgetTheme for RadioButtonListTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {None}
}
