use core::LoopFlow;
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, WidgetSummary, Widget, Parent, OnFocus};
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;

use cgmath::{EuclideanSpace, Point2};
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};
use dct::layout::SizeBounds;

use gl_render::PrimFrame;

#[derive(Debug, Clone)]
pub struct Clip<W> {
    update_tag: UpdateTag,
    rect: BoundBox<Point2<i32>>,
    widget: W
}

impl<W> Clip<W> {
    pub fn new(widget: W) -> Clip<W> {
        Clip {
            update_tag: UpdateTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            widget
        }
    }

    pub fn widget(&self) -> &W {
        &self.widget
    }

    pub fn widget_mut(&mut self) -> &mut W {
        self.update_tag.mark_update_child().mark_update_layout();
        &mut self.widget
    }
}

impl<A, F, W> Widget<A, F> for Clip<W>
    where F: PrimFrame,
          W: Widget<A, F>
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
        SizeBounds::default()
    }

    fn render(&mut self, _: &mut FrameRectStack<F>) {}

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
        EventOps {
            action: None,
            focus: None,
            bubble: true,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }

    fn accepts_focus(&self) -> OnFocus {
        OnFocus::FocusChild
    }
}

impl<A, F, W> Parent<A, F> for Clip<W>
    where F: PrimFrame,
          W: Widget<A, F>
{
    fn num_children(&self) -> usize {
        1
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<A, F>>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetSummary {
                widget: &self.widget as &Widget<A, F>,
                ident: WidgetIdent::Num(0),
                rect: self.widget.rect(),
                size_bounds: self.widget.size_bounds(),
                update_tag: self.widget.update_tag().clone(),
                index: 0
            }),
            _ => None
        }
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetSummary {
                ident: WidgetIdent::Num(0),
                rect: self.widget.rect(),
                size_bounds: self.widget.size_bounds(),
                update_tag: self.widget.update_tag().clone(),
                widget: &mut self.widget as &mut Widget<A, F>,
                index: 0
            }),
            _ => None
        }
    }

    fn children<'a, G, R>(&'a self, mut for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a Widget<A, F>>) -> LoopFlow<R>
    {
        let flow = for_each(WidgetSummary {
            widget: &self.widget as &Widget<A, F>,
            ident: WidgetIdent::Num(0),
            rect: self.widget.rect(),
            size_bounds: self.widget.size_bounds(),
            update_tag: self.widget.update_tag().clone(),
            index: 0
        });

        match flow {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a mut Widget<A, F>>) -> LoopFlow<R>
    {
        let flow = for_each(WidgetSummary {
            ident: WidgetIdent::Num(0),
            rect: self.widget.rect(),
            size_bounds: self.widget.size_bounds(),
            update_tag: self.widget.update_tag().clone(),
            widget: &mut self.widget as &mut Widget<A, F>,
            index: 0
        });

        match flow {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<A, F>>> {
        match index {
            0 => Some(WidgetSummary {
                widget: &self.widget as &Widget<A, F>,
                ident: WidgetIdent::Num(0),
                rect: self.widget.rect(),
                size_bounds: self.widget.size_bounds(),
                update_tag: self.widget.update_tag().clone(),
                index: 0
            }),
            _ => None
        }
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        match index {
            0 => Some(WidgetSummary {
                ident: WidgetIdent::Num(0),
                rect: self.widget.rect(),
                size_bounds: self.widget.size_bounds(),
                update_tag: self.widget.update_tag().clone(),
                widget: &mut self.widget as &mut Widget<A, F>,
                index: 0
            }),
            _ => None
        }
    }

    fn update_child_layout(&mut self) {
        let widget_rect = self.widget.rect();
        let size_bounds = self.widget.size_bounds();

        let dims_clipped = size_bounds.bound_rect(DimsBox::new(widget_rect.dims()));
        if dims_clipped.dims() != widget_rect.dims() {
            *self.widget.rect_mut() = BoundBox::from(dims_clipped) + widget_rect.min().to_vec();
        }
    }
}
