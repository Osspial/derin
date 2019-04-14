// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    event::{EventOps, InputState, WidgetEventSourced},
    widget::{WidgetTag, WidgetRenderable, Widget},
    render::{DisplayEngine},
};
use crate::widgets::{
    Content,
    assistants::toggle_button::{Toggle, ToggleOnClickHandler},
};
use cgmath_geometry::{D2, rect::BoundBox};
use derin_common_types::layout::SizeBounds;

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

#[derive(Default, Debug, Clone, Copy)]
pub struct CheckBoxTheme(());

/// Determines which action, if any, should be taken in response to a button toggle.
pub trait CheckToggleHandler: 'static {
    fn change_state(&mut self, enabled: bool);
}


impl<H: CheckToggleHandler> CheckBox<H> {
    /// Creates a new `CheckBox` with the given checked state, contents, and [toggle handler].
    ///
    /// [toggle handler]: ./trait.CheckToggleHandler.html
    pub fn new(checked: bool, contents: Content, handler: H) -> CheckBox<H> {
        CheckBox {
            toggle: Toggle::new(checked, contents, handler, CheckBoxTheme(())),
        }
    }

    /// Retrieves the contents of the checkbox.
    pub fn contents(&self) -> &Content {
        self.toggle.contents()
    }

    /// Retrieves the contents of the checkbox, for mutation.
    ///
    /// Calling this function forces the checkbox to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> &mut Content {
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

impl<R, H> WidgetRenderable<R> for CheckBox<H>
    where R: Renderer,
          H: CheckToggleHandler,
{
    type Theme = CheckBoxTheme;

    fn theme(&self) -> CheckBoxTheme {
        WidgetRenderable::<R>::theme(&self.toggle)
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        WidgetRenderable::<R>::render(&mut self.toggle, frame)
    }

    fn update_layout(&mut self, l: &mut R::Layout) {
        WidgetRenderable::<R>::update_layout(&mut self.toggle, l)
    }
}

impl WidgetTheme for CheckBoxTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {None}
}

impl<H: CheckToggleHandler> ToggleOnClickHandler for H {
    fn on_click(&mut self, checked: &mut bool) {
        *checked = !*checked;
        self.change_state(*checked);
    }
}
