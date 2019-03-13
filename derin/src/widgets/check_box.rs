// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    event::{EventOps, InputState, WidgetEventSourced},
    widget::{WidgetTag, WidgetRender, Widget},
    render::{Renderer, WidgetTheme},
};
use crate::widgets::{
    Contents,
    assistants::toggle_button::{Toggle, ToggleBoxTheme, ToggleOnClickHandler},
};
use cgmath_geometry::{D2, rect::BoundBox};
use derin_common_types::layout::SizeBounds;

#[derive(Debug, Clone, Copy)]
struct CheckBoxTheme;
impl ToggleBoxTheme for CheckBoxTheme {
    const TYPE_NAME: &'static str = "CheckBox::Toggle";
}

impl<H: CheckToggleHandler> ToggleOnClickHandler for H {
    fn on_click(&mut self, checked: &mut bool) {
        *checked = !*checked;
        self.change_state(*checked);
    }
}

/// Determines which action, if any, should be taken in response to a button toggle.
pub trait CheckToggleHandler: 'static {
    fn change_state(&mut self, enabled: bool);
}

impl<A: 'static + Clone> CheckToggleHandler for Option<A> {
    /// Returns the stored action when the toggle is enabled. Otherwise, returns `None`.
    #[inline]
    fn change_state(&mut self, enabled: bool) {
        unimplemented!()
    }
}

impl CheckToggleHandler for () {
    /// Always returns `None`.
    #[inline]
    fn change_state(&mut self, _: bool) {
        unimplemented!()
    }
}


/// A toggleable box that can be either checked or unchecked.
///
/// When toggled, calls the [`change_state`] function in the associated handler passed in through the
/// `new` function.
///
/// [`change_state`]: ./trait.CheckToggleHandler.html
#[derive(Debug, Clone)]
pub struct CheckBox<H: CheckToggleHandler> {
    toggle: Toggle<H, CheckBoxTheme>,
}

impl<H: CheckToggleHandler> CheckBox<H> {
    /// Creates a new `CheckBox` with the given checked state, contents, and [toggle handler].
    ///
    /// [toggle handler]: ./trait.CheckToggleHandler.html
    pub fn new(checked: bool, contents: Contents, handler: H) -> CheckBox<H> {
        CheckBox {
            toggle: Toggle::new(checked, contents, handler),
        }
    }

    /// Retrieves the contents of the checkbox.
    pub fn contents(&self) -> &Contents {
        self.toggle.contents()
    }

    /// Retrieves the contents of the checkbox, for mutation.
    ///
    /// Calling this function forces the checkbox to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> &mut Contents {
        self.toggle.contents_mut()
    }

    /// Retrieves whether or not the checkbox is checked.
    pub fn checked(&self) -> bool {
        self.toggle.selected()
    }

    /// Retrieves whether or not the checkbox is checked, for mutation.
    ///
    /// Calling this function forces the checkbox to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn checked_mut(&mut self) -> &mut bool {
        self.toggle.selected_mut()
    }
}

impl<H> Widget for CheckBox<H>
    where H: CheckToggleHandler
{
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

impl<R, H> WidgetRender<R> for CheckBox<H>
    where R: Renderer,
          H: CheckToggleHandler,
{
    fn render(&mut self, frame: &mut R::SubFrame) {
        WidgetRender::<R>::render(&mut self.toggle, frame)
    }

    fn theme_list(&self) -> &[WidgetTheme] {
        WidgetRender::<R>::theme_list(&self.toggle)
    }

    fn update_layout(&mut self, l: &mut R::Layout) {
        WidgetRender::<R>::update_layout(&mut self.toggle, l)
    }
}
