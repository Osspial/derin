use derin_core::{
    LoopFlow,
    event::{EventOps, InputState, WidgetEvent, WidgetEventSourced, MouseHoverChange},
    widget::{Parent, Widget, WidgetInfo, WidgetInfoMut, WidgetIdent, WidgetTag, WidgetRenderable},
    render::{DisplayEngine, RendererLayout, SubFrame},
};
use crate::widgets::{
    Content, Label,
    assistants::ButtonState,
};
use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox, OffsetBox}};
use derin_common_types::layout::SizeBounds;

#[derive(Debug, Clone)]
pub struct Toggle<H, T>
    where H: ToggleOnClickHandler,
          T: WidgetTheme + Clone,
{
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,

    tbox: ToggleBox,
    label: Label,
    handler: H,
    theme: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleBoxTheme {
    pub selected: bool,
    pub button_state: ButtonState,
}

pub trait ToggleOnClickHandler: 'static {
    fn on_click(&mut self, selected: &mut bool);
}

/// Toggle-box rendering assistant. Automatically bubbles all events to the parent.
#[derive(Debug, Clone)]
struct ToggleBox {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
    size_bounds: SizeBounds,

    selected: bool,
    button_state: ButtonState,
}

impl<H, T> Toggle<H, T>
    where H: ToggleOnClickHandler,
          T: WidgetTheme + Clone,
{
    /// Creates a new `Toggle` with the given selected state, contents, and [toggle handler].
    ///
    /// [toggle handler]: ./trait.ToggleOnClickHandler.html
    pub fn new(selected: bool, contents: Content, handler: H, theme: T) -> Toggle<H, T> {
        Toggle {
            widget_tag: WidgetTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),

            tbox: ToggleBox {
                widget_tag: WidgetTag::new(),
                rect: BoundBox::new2(0, 0, 0, 0),
                size_bounds: SizeBounds::default(),

                selected,
                button_state: ButtonState::Normal,
            },
            label: Label::new(contents),
            handler,
            theme,
        }
    }

    /// Retrieves the contents of the toggle.
    pub fn contents(&self) -> &Content {
        self.label.contents()
    }

    /// Retrieves the contents of the toggle, for mutation.
    ///
    /// Calling this function forces the toggle to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> &mut Content {
        self.label.contents_mut()
    }

    /// Retrieves whether or not the toggle is selected.
    pub fn selected(&self) -> bool {
        self.tbox.selected
    }

    /// Retrieves whether or not the toggle is selected, for mutation.
    ///
    /// Calling this function forces the toggle to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn selected_mut(&mut self) -> &mut bool {
        self.tbox.widget_tag
            .request_redraw()
            .request_relayout();

        &mut self.tbox.selected
    }
}

impl<H, T> Widget for Toggle<H, T>
    where H: ToggleOnClickHandler,
          T: WidgetTheme + Clone,
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
        &mut self.rect
    }

    fn size_bounds(&self) -> SizeBounds {
        let check_sb = self.tbox.size_bounds;
        let label_sb = self.label.size_bounds();

        SizeBounds {
            min: DimsBox::new2(
                check_sb.min.width() + label_sb.min.width(),
                i32::max(check_sb.min.height(), label_sb.min.height())
            ),
            max: DimsBox::new2(
                check_sb.max.width() + label_sb.max.width(),
                i32::min(check_sb.max.height(), label_sb.max.height())
            ),
        }
    }

    fn on_widget_event(&mut self, event: WidgetEventSourced, _: InputState) -> EventOps {
        use self::WidgetEvent::*;
        // TODO: FIX BUBBLING AND CLICK-DRAGGING OFF OF SUBWIDGET NOT WORKING
        let event = event.unwrap();

        let (mut new_selected, mut new_state) = (self.tbox.selected, self.tbox.button_state);
        match event {
            MouseMove{hover_change: Some(ref change), ..} => match change {
                MouseHoverChange::Enter => new_state = ButtonState::Hover,
                MouseHoverChange::Exit => new_state = ButtonState::Normal,
                _ => ()
            },
            MouseDown{..} => new_state = ButtonState::Pressed,
            MouseUp{in_widget: true, pressed_in_widget: true, ..} => {
                self.handler.on_click(&mut new_selected);
                new_state = ButtonState::Hover;
            },
            MouseUp{in_widget: false, ..} => new_state = ButtonState::Normal,
            GainFocus(_, _) => new_state = ButtonState::Hover,
            LoseFocus => new_state = ButtonState::Normal,
            _ => ()
        };

        if new_selected != self.tbox.selected || new_state != self.tbox.button_state {
            self.tbox.widget_tag.request_redraw();
            self.tbox.selected = new_selected;
            self.tbox.button_state = new_state;
        }


        EventOps {
            focus: None,
            bubble: event.default_bubble(),
        }
    }
}

impl<H, T> Parent for Toggle<H, T>
    where H: ToggleOnClickHandler,
          T: WidgetTheme + Clone,
{
    fn num_children(&self) -> usize {
        1
    }

    fn framed_child<R: Renderer>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, R>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.tbox)),
            _ => None
        }
    }
    fn framed_child_mut<R: Renderer>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, R>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.tbox)),
            _ => None
        }
    }

    fn framed_children<'a, R, G>(&'a self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfo<'a, R>) -> LoopFlow
    {
        let _ = for_each(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.tbox));
    }

    fn framed_children_mut<'a, R, G>(&'a mut self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfoMut<'a, R>) -> LoopFlow
    {
        let _ = for_each(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.tbox));
    }

    fn framed_child_by_index<R: Renderer>(&self, index: usize) -> Option<WidgetInfo<'_, R>> {
        match index {
            0 => Some(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.tbox)),
            _ => None
        }
    }
    fn framed_child_by_index_mut<R: Renderer>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, R>> {
        match index {
            0 => Some(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.tbox)),
            _ => None
        }
    }
}

impl WidgetTheme for ToggleBoxTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {None}
}

impl Widget for ToggleBox {
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

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds
    }

    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl<R, H, T> WidgetRenderable<R> for Toggle<H, T>
    where R: Renderer,
          H: ToggleOnClickHandler,
          T: WidgetTheme + Clone,
{
    type Theme = T;
    fn theme(&self) -> T {
        self.theme.clone()
    }

    fn render(&mut self, _: &mut R::SubFrame) { }

    fn update_layout(&mut self, _: &mut R::Layout) {
        let mut tbox_rect_origin = OffsetBox::from(self.tbox.rect);
        tbox_rect_origin.origin = Point2::new(0, 0);
        self.tbox.rect = BoundBox::from(tbox_rect_origin);

        let mut label_rect_origin = OffsetBox::from(self.label.rect());
        label_rect_origin.origin = Point2::new(self.tbox.rect.max.x, 0);
        let label_rect = BoundBox::from(label_rect_origin);
        if self.label.rect() != label_rect {
            *self.label.rect_mut() = label_rect;
        }
    }
}

impl<R> WidgetRenderable<R> for ToggleBox
    where R: Renderer,
{
    type Theme = ToggleBoxTheme;

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn theme(&self) -> ToggleBoxTheme {
        ToggleBoxTheme {
            selected: self.selected,
            button_state: self.button_state,
        }
    }

    fn update_layout(&mut self, layout: &mut R::Layout) {
        let result = layout.finish();
        self.size_bounds = result.size_bounds;
    }
}
