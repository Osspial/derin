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

use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

#[derive(Debug, Clone)]
pub struct SliderAssist {
    pub value: f32,
    pub step: f32,
    pub min: f32,
    pub max: f32,

    pub head_size: i32,
    pub bar_rect: BoundBox<D2, i32>,
    pub head_click_pos: Option<i32>,
    pub horizontal: bool
}

impl SliderAssist {
    #[inline]
    pub fn round_to_step(&mut self) {
        self.value = ((self.value - self.min) / self.step).round() * self.step + self.min;
        self.value = self.value.min(self.max).max(self.min);
    }

    pub fn head_rect(&self) -> BoundBox<D2, i32> {
        let (bar_size, bar_min) = match self.horizontal {
            true => (self.bar_rect.width(), self.bar_rect.min.x),
            false => (self.bar_rect.height(), self.bar_rect.min.y)
        };

        let head_start = (((self.value - self.min) / (self.max - self.min)) * (bar_size - self.head_size) as f32) as i32 + bar_min;

        match self.horizontal {
            true => BoundBox::new2(
                head_start, self.bar_rect.min.y,
                head_start + self.head_size, self.bar_rect.max.y
            ),
            false => BoundBox::new2(
                self.bar_rect.min.x, head_start,
                self.bar_rect.max.x, head_start + self.head_size
            )
        }
    }

    /// Returns if head was clicked. TODO: ADD MORE
    pub fn click_head(&mut self, click_pos: Point2<i32>) -> bool {
        let head_rect = self.head_rect();
        let (click_pos_axis, head_min) = match self.horizontal {
            true => (click_pos.x, head_rect.min.x),
            false => (click_pos.y, head_rect.min.y)
        };

        if head_rect.contains(click_pos) {
            self.head_click_pos = Some(click_pos_axis - head_min);
            true
        } else if self.bar_rect.contains(click_pos) {
            self.head_click_pos = Some(self.head_size / 2);
            self.move_head(click_pos_axis);
            true
        } else {
            false
        }
    }

    pub fn move_head(&mut self, pos_px: i32) {
        if let Some(head_click_pos) = self.head_click_pos {
            let head_offset = self.head_size / 2;
            let (bar_range_min, bar_range_max) = match self.horizontal {
                true => (
                    self.bar_rect.min.x + head_offset,
                    self.bar_rect.max.x - head_offset
                ),
                false => (
                    self.bar_rect.min.y + head_offset,
                    self.bar_rect.max.y - head_offset
                )
            };

            self.value = (pos_px - head_click_pos - (bar_range_min - head_offset)) as f32
                / (bar_range_max - bar_range_min) as f32
                * (self.max - self.min);
            self.round_to_step();
        }
    }
}
