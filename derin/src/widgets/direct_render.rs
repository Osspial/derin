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

use crate::core::event::{EventOps, WidgetEvent, WidgetEventSourced, InputState};
use crate::core::tree::{WidgetIdent, WidgetTag, Widget, };
use crate::core::render::RenderFrameClipped;

use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::BoundBox};

use crate::gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

use std::mem;
use std::time::Duration;

pub struct DirectRender<R> {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    render_state: R,
}

pub trait DirectRenderState<A> {
    type RenderType;

    fn render(&mut self, _: &mut Self::RenderType);
    fn on_widget_event(
        &mut self,
        _event: WidgetEvent,
        _input_state: InputState,
    ) -> EventOps<A> {
        EventOps {
            action: None,
            focus: None,
            bubble: true,
        }
    }
}

impl<R> DirectRender<R> {
    pub fn new(render_state: R) -> DirectRender<R> {
        DirectRender {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            render_state,
        }
    }

    pub fn render_state(&self) -> &R {
        &self.render_state
    }

    pub fn render_state_mut(&mut self) -> &mut R {
        self.widget_tag.request_redraw();
        &mut self.render_state
    }

    pub fn mark_redraw(&mut self) {
        self.widget_tag.request_redraw();
    }
}

impl<A, F, R> Widget<A, F> for DirectRender<R>
    where R: DirectRenderState<A> + 'static,
          F: PrimFrame<DirectRender = R::RenderType>
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        &mut self.bounds
    }

    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        let mut draw_fn = |render_type: &mut R::RenderType| self.render_state.render(render_type);
        frame.upload_primitives(Some(ThemedPrim {
            theme_path: "DirectRender",
            min: Point2::new(
                RelPoint::new(-1.0, 0),
                RelPoint::new(-1.0, 0),
            ),
            max: Point2::new(
                RelPoint::new( 1.0, 0),
                RelPoint::new( 1.0, 0)
            ),
            prim: unsafe{ Prim::DirectRender(mem::transmute((&mut draw_fn) as &mut FnMut(&mut R::RenderType))) },
            rect_px_out: None
        }).into_iter());
    }

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEventSourced, input_state: InputState) -> EventOps<A> {
        let event = event.unwrap();

        let ops = self.render_state.on_widget_event(event, input_state);

        ops
    }
}
