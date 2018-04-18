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

use LoopFlow;
use popup::ChildPopupsMut;
use tree::{OnFocus, Widget, WidgetIdent, WidgetTag, WidgetSummary};
use tree::dyn::ParentDyn;
use event::{InputState, WidgetEvent, EventOps};
use render::RenderFrame;
use timer::TimerRegister;
use mbseq::MouseButtonSequenceTrackPos;

use derin_common_types::buttons::ModifierKeys;
use derin_common_types::layout::SizeBounds;

use cgmath::{Point2, Vector2, EuclideanSpace};
use cgmath_geometry::{BoundBox, GeoBox};

use arrayvec::ArrayVec;

#[derive(Debug)]
pub struct OffsetWidget<'a, W: 'a + ?Sized> {
    widget: &'a mut W,
    offset: Vector2<i32>,
    clip: Option<BoundBox<Point2<i32>>>
}

impl<'a, W: ?Sized> OffsetWidget<'a, W> {
    #[inline]
    pub fn new(widget: &'a mut W, offset: Vector2<i32>, clip: Option<BoundBox<Point2<i32>>>) -> OffsetWidget<'a, W> {
        OffsetWidget {
            widget,
            offset,
            clip
        }
    }

    // #[inline]
    // pub fn inner(&self) -> &W {
    //     self.widget
    // }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut W {
        self.widget
    }
}

pub trait OffsetWidgetTrait<A, F>
    where F: RenderFrame
{
    type Widget: Widget<A, F> + ?Sized;

    fn update_tag(&self) -> &WidgetTag;
    fn rect(&self) -> BoundBox<Point2<i32>>;
    fn rect_clipped(&self) -> Option<BoundBox<Point2<i32>>>;
    fn set_rect(&mut self, rect: BoundBox<Point2<i32>>);
    // fn render(&mut self, frame: &mut FrameRectStack<F>) {
    //     self.widget.render(frame);
    // }
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        mouse_pos: Point2<i32>,
        mouse_buttons_down: &MouseButtonSequenceTrackPos,
        modifiers: ModifierKeys,
        popups: Option<ChildPopupsMut<A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F>;

    // fn subtrait(&self) -> WidgetSubtrait<A, F>;
    // fn subtrait_mut(&mut self) -> WidgetSubtraitMut<A, F>;

    fn size_bounds(&self) -> SizeBounds;
    fn register_timers(&self, register: &mut TimerRegister);
    fn accepts_focus(&self) -> OnFocus;

    fn num_children(&self) -> usize
        where Self::Widget: ParentDyn<A, F>;
    fn update_child_layout(&mut self)
        where Self::Widget: ParentDyn<A, F>;
    fn children_mut<'b, G>(&'b mut self, for_each: G)
        where A: 'b,
              Self::Widget: ParentDyn<A, F>,
              G: FnMut(WidgetSummary<OffsetWidget<'b, Widget<A, F>>>) -> LoopFlow<()>;
}

pub trait OffsetWidgetTraitAs<'a, A, F: RenderFrame> {
    type AsParent: 'a;

    fn as_parent_mut(self) -> Option<Self::AsParent>;
}

impl<'a, A, F, W> OffsetWidgetTrait<A, F> for OffsetWidget<'a, W>
    where A: 'a,
          F: RenderFrame,
          W: Widget<A, F> + ?Sized
{
    type Widget = W;

    fn update_tag(&self) -> &WidgetTag {
        self.widget.update_tag()
    }
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.widget.rect() + self.offset
    }
    fn rect_clipped(&self) -> Option<BoundBox<Point2<i32>>> {
        self.clip.and_then(|clip_rect| clip_rect.intersect_rect(self.rect()))
    }
    fn set_rect(&mut self, rect: BoundBox<Point2<i32>>) {
        *self.widget.rect_mut() = rect - self.offset;
    }
    // fn render(&mut self, frame: &mut FrameRectStack<F>) {
    //     self.widget.render(frame);
    // }
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        mouse_pos: Point2<i32>,
        mouse_buttons_down: &MouseButtonSequenceTrackPos,
        modifiers: ModifierKeys,
        popups: Option<ChildPopupsMut<A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F>
    {
        let update_tag = self.update_tag();
        let offset = self.rect().min().to_vec();
        let mbd_array: ArrayVec<[_; 5]> = mouse_buttons_down.clone().into_iter()
            .map(|mut down| {
                down.down_pos -= offset;
                down
            }).collect();
        let mbdin_array: ArrayVec<[_; 5]> = update_tag.mouse_state.get().mouse_button_sequence()
            .into_iter().filter_map(|b| mouse_buttons_down.contains(b))
            .map(|mut down| {
                down.down_pos -= offset;
                down
            }).collect();

        let input_state = InputState {
            mouse_pos: mouse_pos - offset,
            modifiers: modifiers,
            mouse_buttons_down: &mbd_array,
            mouse_buttons_down_in_widget: &mbdin_array
        };
        let mut ops = self.widget.on_widget_event(
            event.translate(-offset),
            input_state,
            popups,
            source_child
        );
        if let Some((_, ref mut popup_attributes)) = ops.popup {
            popup_attributes.rect = popup_attributes.rect + offset;
        }
        if let Some(ref mut cursor_pos) = ops.cursor_pos {
            *cursor_pos += offset;
        }
        ops
    }
    // fn subtrait(&self) -> WidgetSubtrait<A, F>;
    // fn subtrait_mut(&mut self) -> WidgetSubtraitMut<A, F>;

    fn size_bounds(&self) -> SizeBounds {
        self.widget.size_bounds()
    }
    fn register_timers(&self, register: &mut TimerRegister) {
        self.widget.register_timers(register)
    }
    fn accepts_focus(&self) -> OnFocus {
        self.widget.accepts_focus()
    }

    fn num_children(&self) -> usize
        where W: ParentDyn<A, F>
    {
        self.widget.num_children()
    }
    fn update_child_layout(&mut self)
        where W: ParentDyn<A, F>
    {
        self.widget.update_child_layout();
    }

    fn children_mut<'b, G>(&'b mut self, mut for_each: G)
        where A: 'b,
              Self::Widget: ParentDyn<A, F>,
              G: FnMut(WidgetSummary<OffsetWidget<'b, Widget<A, F>>>) -> LoopFlow<()>
    {
        let child_offset = self.rect().min().to_vec();
        let clip_rect = self.rect_clipped();

        self.widget.children_mut(&mut |summary_slice| {
            for summary in summary_slice {
                let widget: OffsetWidget<'b, _> = OffsetWidget::new(summary.widget, child_offset, clip_rect);
                let summary_offset = WidgetSummary {
                    ident: summary.ident,
                    index: summary.index,
                    widget
                };
                if LoopFlow::Break(()) == for_each(summary_offset) {
                    return LoopFlow::Break(());
                }
            }

            LoopFlow::Continue
        });
    }
}

impl<'a, 'b, A, F, W> OffsetWidgetTraitAs<'b, A, F> for &'b mut OffsetWidget<'a, W>
    where A: 'b,
          F: RenderFrame,
          W: Widget<A, F> + ?Sized
{
    type AsParent = OffsetWidget<'b, ParentDyn<A, F>>;

    fn as_parent_mut(self) -> Option<OffsetWidget<'b, ParentDyn<A, F>>> {
        match self.widget.as_parent_mut() {
            Some(self_as_parent) => Some(OffsetWidget {
                widget: self_as_parent,
                offset: self.offset,
                clip: self.clip
            }),
            None => None
        }
    }
}
