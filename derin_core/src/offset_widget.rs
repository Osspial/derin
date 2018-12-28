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

use crate::{LoopFlow, InputState};
use crate::popup::ChildPopupsMut;
use crate::tree::{Widget, WidgetIdent, WidgetTag, WidgetSummary};
use crate::tree::dynamic::ParentDyn;
use crate::event::{InputState as EventInputState, WidgetEvent, EventOps};
use crate::render::RenderFrame;
use crate::timer::TimerRegister;

use derin_common_types::layout::SizeBounds;

use crate::cgmath::{Vector2, EuclideanSpace};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use arrayvec::ArrayVec;

#[derive(Debug)]
pub(crate) struct OffsetWidget<'a, W: 'a + ?Sized> {
    widget: &'a mut W,
    offset: Vector2<i32>,
    clip: Option<BoundBox<D2, i32>>
}

impl<'a, W: ?Sized> OffsetWidget<'a, W> {
    #[inline]
    pub fn new(widget: &'a mut W, offset: Vector2<i32>, clip: Option<BoundBox<D2, i32>>) -> OffsetWidget<'a, W> {
        OffsetWidget {
            widget,
            offset,
            clip
        }
    }

    #[inline]
    pub fn inner(&self) -> &W {
        self.widget
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut W {
        self.widget
    }
}

pub(crate) trait OffsetWidgetTrait<A, F>
    where F: RenderFrame
{
    type Widget: Widget<A, F> + ?Sized;

    fn widget_tag(&self) -> &WidgetTag;
    fn rect(&self) -> BoundBox<D2, i32>;
    fn rect_clipped(&self) -> Option<BoundBox<D2, i32>>;
    fn set_rect(&mut self, rect: BoundBox<D2, i32>);
    // fn render(&mut self, frame: &mut FrameRectStack<F>) {
    //     self.widget.render(frame);
    // }
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: &InputState,
        popups: Option<ChildPopupsMut<A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F>;

    // fn subtrait(&self) -> WidgetSubtrait<A, F>;
    // fn subtrait_mut(&mut self) -> WidgetSubtraitMut<A, F>;

    fn update_layout(&mut self);
    fn size_bounds(&self) -> SizeBounds;
    fn register_timers(&self, register: &mut TimerRegister);

    fn num_children(&self) -> usize
        where Self::Widget: ParentDyn<A, F>;
    fn children<'b, G>(&'b self, for_each: G)
        where A: 'b,
              Self::Widget: ParentDyn<A, F>,
              G: FnMut(WidgetSummary<&'b Widget<A, F>>) -> LoopFlow<()>;
    fn children_mut<'b, G>(&'b mut self, for_each: G)
        where A: 'b,
              Self::Widget: ParentDyn<A, F>,
              G: FnMut(WidgetSummary<OffsetWidget<'b, Widget<A, F>>>) -> LoopFlow<()>;
}

pub(crate) trait OffsetWidgetTraitAs<'a, A, F: RenderFrame> {
    type AsParent: 'a;

    fn as_parent_mut(self) -> Option<Self::AsParent>;
}

impl<'a, A, F, W> OffsetWidgetTrait<A, F> for OffsetWidget<'a, W>
    where A: 'a,
          F: RenderFrame,
          W: Widget<A, F> + ?Sized
{
    type Widget = W;

    fn widget_tag(&self) -> &WidgetTag {
        self.widget.widget_tag()
    }
    fn rect(&self) -> BoundBox<D2, i32> {
        self.widget.rect() + self.offset
    }
    fn rect_clipped(&self) -> Option<BoundBox<D2, i32>> {
        self.clip.and_then(|clip_rect| clip_rect.intersect_rect(self.rect()))
    }
    fn set_rect(&mut self, rect: BoundBox<D2, i32>) {
        *self.widget.rect_mut() = rect - self.offset;
    }
    // fn render(&mut self, frame: &mut FrameRectStack<F>) {
    //     self.widget.render(frame);
    // }
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: &InputState,
        popups: Option<ChildPopupsMut<A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F>
    {
        let InputState {
            mouse_pos,
            mouse_buttons_down,
            keys_down,
            modifiers,
            ..
        } = input_state;
        let widget_tag = self.widget_tag();
        let offset = self.rect().min().to_vec();
        let mbd_array: ArrayVec<[_; 5]> = mouse_buttons_down.clone().into_iter()
            .map(|down| down.mouse_down)
            .map(|mut down| {
                down.down_pos -= offset;
                down
            }).collect();
        let mbdin_array: ArrayVec<[_; 5]> = widget_tag.mouse_state.get().mouse_button_sequence()
            .into_iter().filter_map(|b| mouse_buttons_down.contains(b))
            .map(|down| down.mouse_down)
            .map(|mut down| {
                down.down_pos -= offset;
                down
            }).collect();

        let input_state = EventInputState {
            mouse_pos: mouse_pos.map(|p| p - offset),
            modifiers: *modifiers,
            mouse_buttons_down: &mbd_array[..],
            mouse_buttons_down_in_widget: &mbdin_array,
            keys_down
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

    fn num_children(&self) -> usize
        where W: ParentDyn<A, F>
    {
        self.widget.num_children()
    }
    fn update_layout(&mut self)
    {
        self.widget.update_layout();
    }

    fn children<'b, G>(&'b self, mut for_each: G)
        where A: 'b,
              Self::Widget: ParentDyn<A, F>,
              G: FnMut(WidgetSummary<&'b Widget<A, F>>) -> LoopFlow<()>
    {
        self.widget.children(&mut |summary_slice| {
            for summary in summary_slice {
                if LoopFlow::Break(()) == for_each(summary) {
                    return LoopFlow::Break(());
                }
            }

            LoopFlow::Continue
        });
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
