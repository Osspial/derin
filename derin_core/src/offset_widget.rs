// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    {LoopFlow, InputState},
    widget::{WidgetDyn, WidgetId, WidgetIdent, WidgetTag},
    event::{InputState as EventInputState, WidgetEventSourced, EventOps},
    render::{RenderFrame, RenderFrameClipped},
};

use derin_common_types::layout::SizeBounds;

use crate::cgmath::{Vector2, EuclideanSpace};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use arrayvec::ArrayVec;

pub(crate) struct OffsetWidget<'a, F: RenderFrame> {
    widget: &'a mut WidgetDyn<F>,
    offset: Vector2<i32>,
    clip: Option<BoundBox<D2, i32>>,
}

pub(crate) struct OffsetWidgetInfo<'a, F: RenderFrame> {
    pub ident: WidgetIdent,
    pub index: usize,
    pub widget: OffsetWidget<'a, F>,
}

impl<'a, F: RenderFrame> OffsetWidget<'a, F> {
    #[inline]
    pub fn new(widget: &'a mut WidgetDyn<F>, offset: Vector2<i32>, clip: Option<BoundBox<D2, i32>>) -> OffsetWidget<'a, F> {
        OffsetWidget {
            widget,
            offset,
            clip,
        }
    }

    #[inline]
    pub fn inner(&self) -> &WidgetDyn<F> {
        self.widget
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut WidgetDyn<F> {
        self.widget
    }

    pub fn clip(&self) -> Option<BoundBox<D2, i32>> {
        self.clip
    }

    pub fn widget_tag(&self) -> &WidgetTag {
        self.widget.widget_tag()
    }
    pub fn widget_id(&self) -> WidgetId {
        self.widget.widget_id()
    }
    pub fn rect(&self) -> BoundBox<D2, i32> {
        self.widget.rect() + self.offset
    }
    pub fn rect_clipped(&self) -> Option<BoundBox<D2, i32>> {
        self.clip.and_then(|clip_rect| clip_rect.intersect_rect(self.rect()))
    }
    pub fn set_rect(&mut self, rect: BoundBox<D2, i32>) {
        *self.widget.rect_mut() = rect - self.offset;
    }
    pub fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        self.widget.render(frame);
    }
    pub fn on_widget_event(
        &mut self,
        event: WidgetEventSourced,
        input_state: &InputState,
    ) -> EventOps
    {
        let InputState {
            mouse_pos,
            mouse_buttons_down,
            keys_down,
            modifiers,
            ..
        } = input_state;
        let offset = self.rect().min().to_vec();
        let mbd_array: ArrayVec<[_; 5]> = mouse_buttons_down.clone().into_iter()
            .map(|down| down.mouse_down)
            .map(|mut down| {
                down.down_pos -= offset;
                down
            }).collect();
        let mbdin_array: ArrayVec<[(); 5]> = ArrayVec::new(); //TODO: GET ACTUAL VALUES

        let input_state = EventInputState {
            mouse_pos: mouse_pos.map(|p| p - offset),
            modifiers: *modifiers,
            mouse_buttons_down: &mbd_array[..],
            mouse_buttons_down_in_widget: &mbd_array[..],
            keys_down
        };
        let ops = self.widget.on_widget_event(
            event.map(|e| e.translate(-offset)),
            input_state,
        );
        ops
    }
    // pub fn subtrait(&self) -> WidgetSubtrait<F>;
    // pub fn subtrait_mut(&mut self) -> WidgetSubtraitMut<F>;

    pub fn size_bounds(&self) -> SizeBounds {
        self.widget.size_bounds()
    }

    // pub fn num_children(&self) -> usize {
    //     self.widget.num_children()
    // }
    pub fn update_layout(&mut self, theme: &F::Theme) {
        self.widget.update_layout(theme);
    }

    // pub fn children<'b, G>(&'b self, mut for_each: G)
    //     where G: FnMut(WidgetSummary<&'b WidgetDyn<F>>) -> LoopFlow
    // {
    //     self.widget.children(&mut |summary_slice| {
    //         for summary in summary_slice {
    //             if LoopFlow::Break == for_each(summary) {
    //                 return LoopFlow::Break;
    //             }
    //         }

    //         LoopFlow::Continue
    //     });
    // }

    pub fn children_mut<'b, G>(&'b mut self, mut for_each: G)
        where G: FnMut(OffsetWidgetInfo<'b, F>) -> LoopFlow
    {
        let child_offset = self.rect().min().to_vec();
        let clip_rect = self.rect_clipped();

        self.widget.children_mut(&mut |widget_slice| {
            for info in widget_slice {
                let widget: OffsetWidget<'b, _> = OffsetWidget::new(info.widget, child_offset, clip_rect);
                let child_offset = OffsetWidgetInfo {
                    ident: info.ident,
                    index: info.index,
                    widget
                };
                if LoopFlow::Break == for_each(child_offset) {
                    return LoopFlow::Break;
                }
            }

            LoopFlow::Continue
        });
    }
}
