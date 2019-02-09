// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod shape_glyphs;

use shape_glyphs::RenderGlyph;
use crate::gl_render::GLVertex;
use crate::gl_render::atlas::Atlas;
use crate::gl_render::translate::image::ImageToVertices;
use crate::theme::{ThemeText, RescaleRules, LineWrap};

use crate::cgmath::{EuclideanSpace, ElementWise, Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, OffsetBox, GeoBox}, line::Segment};

use gullery::image_format::Rgba;

use glyphydog::{ShapedBuffer, Face, FaceSize, DPI, LoadFlags, RenderMode};
use derin_common_types::layout::Align;

use unicode_segmentation::UnicodeSegmentation;

use std::cmp;
use std::cmp::Ordering;
use std::ops::Range;
use std::any::Any;


pub(in crate::gl_render) struct TextToVertices<'a> {
    glyph_draw: GlyphDraw<'a>,

    glyph_slice_index: usize,
    glyph_slice: &'a [RenderGlyph],
    highlight_range: Range<usize>,
    cursor_pos: Option<usize>,
    offset: Vector2<i32>,

    font_ascender: i32,
    font_descender: i32,

    highlight_vertex_iter: Option<ImageToVertices>,
    glyph_vertex_iter: Option<ImageToVertices>,
    cursor_vertex_iter: Option<ImageToVertices>
}

#[derive(Debug, Clone)]
pub struct RenderString {
    pub offset: Vector2<i32>,
    string: String,
    min_size: DimsBox<D2, i32>,
    draw_data: Option<StringDrawData>,
    pub draw_cursor: bool,
    cursor_pos: usize,
    highlight_range: Range<usize>,
    cursor_target_x_px: Option<i32>,
}

#[derive(Debug, Clone)]
struct StringDrawData {
    shaped_glyphs: Vec<RenderGlyph>,
    text_style: ThemeText,
    dpi: DPI,
    draw_rect: BoundBox<D2, i32>,
    text_rect: Option<BoundBox<D2, i32>>
}

struct GlyphDraw<'a> {
    rect: BoundBox<D2, i32>,
    clip_rect: BoundBox<D2, i32>,
    face: &'a mut Face<Any>,
    atlas: &'a mut Atlas,
    text_style: ThemeText,
    dpi: DPI
}

impl<'a> TextToVertices<'a> {
    pub fn new<'b, F>(
        mut rect: BoundBox<D2, i32>,
        clip_rect: BoundBox<D2, i32>,
        text_style: ThemeText,
        face: &'a mut Face<Any>,
        dpi: DPI,
        atlas: &'a mut Atlas,
        shape_text: F,
        render_string: &'a mut RenderString
    ) -> TextToVertices<'a>
        where F: FnOnce(&str, &mut Face<Any>) -> &'b ShapedBuffer
    {
        let face_size = FaceSize::new(text_style.face_size, text_style.face_size);
        let font_metrics = face.metrics_sized(face_size, dpi).unwrap();
        let (ascender, descender) = ((font_metrics.ascender / 64) as i32, (font_metrics.descender / 64) as i32);

        rect.min.x += text_style.margins.left as i32;
        rect.max.x -= text_style.margins.right as i32;
        rect.min.y += text_style.margins.top as i32;
        rect.max.y -= text_style.margins.bottom as i32;

        TextToVertices {
            glyph_slice_index: 0,
            highlight_range: render_string.highlight_range.clone(),
            cursor_pos: Some(render_string.cursor_pos).filter(|_| render_string.draw_cursor()),
            offset: render_string.offset,

            font_ascender: ascender,
            font_descender: descender,

            glyph_slice: render_string.reshape_glyphs(rect, shape_text, &text_style, face, dpi),
            glyph_draw: GlyphDraw {
                face, atlas, text_style, dpi, rect,
                clip_rect: clip_rect.intersect_rect(rect).unwrap_or(BoundBox::new2(0, 0, 0, 0))
            },

            highlight_vertex_iter: None,
            glyph_vertex_iter: None,
            cursor_vertex_iter: None
        }
    }
}

impl<'a> Iterator for TextToVertices<'a> {
    type Item = GLVertex;

    fn next(&mut self) -> Option<GLVertex> {
        loop {
            fn next_in_iter(i: Option<impl Iterator<Item=GLVertex>>) -> Option<GLVertex> {i.map(|mut v| v.next()).unwrap_or(None)}
            let next_vertex =
                next_in_iter(self.highlight_vertex_iter.as_mut())
                    .or_else(|| next_in_iter(self.glyph_vertex_iter.as_mut()))
                    .or_else(|| next_in_iter(self.cursor_vertex_iter.as_mut()));
            match next_vertex {
                Some(vert) => return Some(vert),
                None => {
                    let TextToVertices {
                        ref glyph_slice,
                        ref mut glyph_slice_index,
                        ref highlight_range,
                        ref mut cursor_pos,
                        ref mut glyph_draw,
                        offset,
                        font_ascender,
                        font_descender,
                        ref mut glyph_vertex_iter,
                        ref mut highlight_vertex_iter,
                        ref mut cursor_vertex_iter,
                    } = *self;
                    macro_rules! get_glyph_slice {
                        (range $i:expr) => {{glyph_slice.get($i).iter().flat_map(|g| g.iter()).cloned().map(|g| g.offset(offset))}};
                        ($i:expr) => {{glyph_slice.get($i).cloned().map(|g| g.offset(offset))}};
                    }
                    let next_glyph_opt = get_glyph_slice!(*glyph_slice_index);

                    *cursor_vertex_iter = cursor_pos.and_then(|pos| {
                        // The position of the top-left corner of the cursor.
                        let base_pos = match next_glyph_opt {
                            Some(next_glyph) => {
                                let highlight_rect = next_glyph.highlight_rect + glyph_draw.rect.min().to_vec();

                                if pos == next_glyph.str_index {
                                    Some(highlight_rect.min())
                                } else if pos == next_glyph.str_index + next_glyph.grapheme_len {
                                    Some(Point2::new(highlight_rect.max().x, highlight_rect.min().y))
                                } else {
                                    None
                                }
                            },
                            None if pos == 0 => Some(
                                Point2 {
                                    x: match glyph_draw.text_style.justify.x {
                                        Align::Start |
                                        Align::Stretch => 0,
                                        Align::Center => glyph_draw.rect.width() as i32 / 2,
                                        Align::End => glyph_draw.rect.width()
                                    },
                                    y: match glyph_draw.text_style.justify.y {
                                        Align::Start => -font_descender,
                                        Align::Stretch => glyph_draw.rect.height() as i32 / 2 - font_ascender,
                                        Align::Center => (glyph_draw.rect.height() as i32 - font_ascender - font_descender) / 2,
                                        Align::End => glyph_draw.rect.height() as i32 - font_ascender,
                                    }
                                } + glyph_draw.rect.min().to_vec()
                            ),
                            None => None
                        };

                        base_pos.map(|pos| {
                            *cursor_pos = None;
                            ImageToVertices::new(
                                BoundBox::new(pos, pos + Vector2::new(1, font_ascender - font_descender)),
                                glyph_draw.clip_rect,
                                glyph_draw.atlas.white().cast().unwrap_or(OffsetBox::new2(0, 0, 0, 0)),
                                glyph_draw.text_style.color,
                                RescaleRules::StretchOnPixelCenter
                            )
                        })
                    });

                    let next_glyph = match (next_glyph_opt, self.cursor_vertex_iter.is_some()) {
                        (Some(next_glyph), _) => next_glyph,
                        (None, false) => return None,
                        (None, true) => continue
                    };
                    *glyph_slice_index += 1;

                    let is_highlighted = highlight_range.contains(&next_glyph.str_index);
                    *glyph_vertex_iter = next_glyph.glyph_index.map(|glyph_index|
                        glyph_draw.glyph_atlas_image(
                            next_glyph.pos,
                            glyph_index,
                            is_highlighted,
                            glyph_draw.rect
                        )
                    );

                    let starts_highlight_rect =
                        (
                            highlight_range.start == next_glyph.str_index &&
                            highlight_range.len() > 0
                        ) ||
                        (
                            is_highlighted &&
                            Some(next_glyph.pos.y) != get_glyph_slice!(*glyph_slice_index - 2).map(|g| g.pos.y)
                        );
                    *highlight_vertex_iter = match starts_highlight_rect {
                        true => {
                            let highlight_rect_end = get_glyph_slice!(range *glyph_slice_index..)
                                .take_while(|g| g.pos.y == next_glyph.pos.y)
                                .take_while(|g| g.str_index < highlight_range.end)
                                .last().unwrap_or(next_glyph).highlight_rect.max().x;

                            let mut highlight_rect = next_glyph.highlight_rect;
                            highlight_rect.max.x = highlight_rect_end;
                            highlight_rect = highlight_rect + glyph_draw.rect.min().to_vec();

                            Some(ImageToVertices::new(
                                highlight_rect,
                                glyph_draw.clip_rect,
                                glyph_draw.atlas.white().cast().unwrap_or(OffsetBox::new2(0, 0, 0, 0)),
                                glyph_draw.text_style.highlight_bg_color,
                                RescaleRules::StretchOnPixelCenter
                            ))
                        },
                        false => None
                    };

                    continue;
                }
            }
        }
    }
}

impl<'a> GlyphDraw<'a> {
    fn glyph_atlas_image(&mut self, mut glyph_pos: Point2<i32>, glyph_index: u32, is_highlighted: bool, rect: BoundBox<D2, i32>) -> ImageToVertices {
        let GlyphDraw {
            ref mut face,
            ref mut atlas,
            ref text_style,
            dpi,
            ..
        } = *self;

        let face_size = FaceSize::new(text_style.face_size, text_style.face_size);

        let render_mode = RenderMode::Normal;
        let (atlas_rect, glyph_bearing) = atlas.glyph_rect(
            text_style.face.clone(),
            text_style.face_size,
            glyph_index,
            || {
                let glyph_res = face.load_glyph(
                    glyph_index,
                    face_size,
                    dpi,
                    LoadFlags::empty(),
                    render_mode
                ).and_then(|mut glyph_slot| Ok((
                    glyph_slot.render_glyph(render_mode)?,
                    glyph_slot.metrics()
                )));

                match glyph_res {
                    Ok((bitmap, glyph_metrics)) => {
                        assert!(bitmap.pitch >= 0);
                        let (bytes, pitch, dims) = match bitmap.pitch {
                            0 => (&[][..], 1, DimsBox::new2(0, 0)),
                            _ => (bitmap.buffer, bitmap.pitch as usize, bitmap.dims)
                        };
                        (
                            bytes.chunks(pitch)
                                .map(move |b|
                                    b[..dims.width() as usize]
                                        // We upload white glyphs to the atlas, which are colored by
                                        // vertex colors.
                                        .into_iter().map(|t| Rgba::new(255, 255, 255, *t))
                                ),
                            bitmap.dims,
                            glyph_metrics.hori_bearing / 64
                        )
                    },
                    Err(_) => {
                        // TODO: LOG
                        unimplemented!()
                        // (&[], DimsBox::new2(0, 0), Vector2::new(0, 0))
                    }
                }

            }
        );

        glyph_pos +=
            // rect top-left
            rect.min().to_vec() +
            // Advance the cursor down the line. Pos is with TLO, so vertical flip
            Vector2::new(1, -1).mul_element_wise(glyph_bearing);
        let glyph_rect = BoundBox::new2(
            glyph_pos.x,
            glyph_pos.y,
            glyph_pos.x + atlas_rect.width() as i32,
            glyph_pos.y + atlas_rect.height() as i32
        );

        ImageToVertices::new(
            glyph_rect,
            self.clip_rect,
            atlas_rect.cast::<u16>().unwrap_or(OffsetBox::new2(0, 0, 0, 0)),
            match is_highlighted {
                false => text_style.color,
                true => text_style.highlight_text_color
            },
            RescaleRules::Stretch
        )
    }
}

impl Default for RenderString {
    #[inline]
    fn default() -> RenderString {
        RenderString::new(String::new())
    }
}

impl RenderString {
    pub fn new(string: String) -> RenderString {
        RenderString {
            offset: Vector2::new(0, 0),
            string,
            min_size: DimsBox::new2(0, 0),
            draw_data: None,
            draw_cursor: false,
            cursor_pos: 0,
            highlight_range: 0..0,
            cursor_target_x_px: None,
        }
    }

    #[inline]
    pub fn string(&self) -> &str {
        &self.string
    }

    #[inline]
    pub fn string_mut(&mut self) -> &mut String {
        if let Some(ref mut draw_data) = self.draw_data {
            draw_data.shaped_glyphs.clear();
        }
        &mut self.string
    }

    #[inline]
    pub fn min_size(&self) -> DimsBox<D2, i32> {
        self.min_size
    }

    #[inline]
    pub fn text_rect(&self) -> Option<BoundBox<D2, i32>> {
        self.draw_data.as_ref().and_then(|d| d.text_rect)
    }

    fn reshape_glyphs<'a, F>(&mut self,
        rect: BoundBox<D2, i32>,
        shape_text: F,
        text_style: &ThemeText,
        face: &mut Face<Any>,
        dpi: DPI,
    ) -> &[RenderGlyph]
        where F: FnOnce(&str, &mut Face<Any>) -> &'a ShapedBuffer
    {
        let use_cached_glyphs: bool;
        match self.draw_data {
            Some(ref mut draw_data) => {
                use_cached_glyphs =
                    draw_data.shaped_glyphs.len() != 0 &&
                    (text_style, dpi, rect) ==
                    (&draw_data.text_style, draw_data.dpi, draw_data.draw_rect);

                // Update draw_data contents to reflect new values
                draw_data.text_style = text_style.clone();
                draw_data.dpi = dpi;
                draw_data.draw_rect = rect;
            },
            None => {
                use_cached_glyphs = false;
                self.draw_data = Some(StringDrawData {
                    shaped_glyphs: Vec::new(),
                    text_style: text_style.clone(),
                    dpi,
                    draw_rect: rect,
                    text_rect: None
                });
            }
        }

        let draw_cursor = self.draw_cursor();
        let draw_data = self.draw_data.as_mut().unwrap();
        if !use_cached_glyphs {
            let shaped_buffer = shape_text(&self.string, face);
            draw_data.shaped_glyphs.clear();

            let shaped_data = shape_glyphs::shape_glyphs(
                rect,
                shaped_buffer,
                text_style,
                face,
                dpi,
                &mut draw_data.shaped_glyphs
            );
            draw_data.text_rect = Some(shaped_data.text_rect);

            self.min_size = match text_style.line_wrap {
                LineWrap::None => {
                    let mut dims_margins = shaped_data.text_rect.dims();
                    dims_margins.dims.x += text_style.margins.width() as i32;
                    dims_margins.dims.y += text_style.margins.height() as i32;
                    dims_margins
                },
                _ => DimsBox::new2(0, 0)
            };
        }

        // If the cursor is outside of the draw rectangle, offset the text so that the cursor and
        // cursor glyph get drawn.
        if draw_cursor {
            let (draw_width, draw_height) = (draw_data.draw_rect.width(), draw_data.draw_rect.height());

            // Used to work around ICE
            fn get_glyph(s: &RenderString, cursor_pos: usize) -> Option<RenderGlyph> {
                s.glyph_iter().skip_while(|glyph| glyph.str_index != cursor_pos).next()
            }

            let mut offset = Vector2::new(0, 0);
            if let Some(cursor_glyph) = get_glyph(self, self.cursor_pos) {
                let cursor_x = cursor_glyph.highlight_rect.min.x;
                let cursor_y_start = cursor_glyph.highlight_rect.min.y;
                let cursor_y_end = cursor_glyph.highlight_rect.max.y;

                offset.x += match () {
                    _ if cursor_x < 0 => -cursor_x,
                    _ if draw_width < cursor_x => draw_width - cursor_x - 1,
                    _ => 0
                };
                offset.y += match () {
                    _ if cursor_y_start < 0 => -cursor_y_start,
                    _ if draw_height < cursor_y_end => draw_height - cursor_y_end,
                    _ => 0
                };
            }

            self.offset += offset;
        }

        &self.draw_data.as_ref().unwrap().shaped_glyphs[..]
    }

    fn glyph_iter<'a>(&'a self) -> impl 'a + Iterator<Item=RenderGlyph> + DoubleEndedIterator {
        let glyph_offset = self.offset;
        let offset_glyph = move |g: RenderGlyph| g.offset(glyph_offset);
        let empty_iter = [].iter().cloned().chain(None).map(offset_glyph.clone());

        let shaped_glyphs = match self.draw_data {
            Some(ref draw_data) => &draw_data.shaped_glyphs,
            None => return empty_iter
        };

        if let Some(last_glyph) = shaped_glyphs.last().cloned() {
            let dummy_last_glyph_pos_x = last_glyph.pos.x + last_glyph.highlight_rect.width();
            let dummy_last_glyph = RenderGlyph {
                pos: Point2::new(dummy_last_glyph_pos_x, last_glyph.pos.y),
                highlight_rect: BoundBox::new2(
                    dummy_last_glyph_pos_x, last_glyph.highlight_rect.min.y,
                    dummy_last_glyph_pos_x, last_glyph.highlight_rect.min.y + last_glyph.highlight_rect.height()
                ),
                str_index: self.string.len(),
                grapheme_len: 0,
                glyph_index: None
            };
            shaped_glyphs.iter().cloned().chain(Some(dummy_last_glyph)).map(offset_glyph)
        } else {
            empty_iter
        }
    }

    #[inline]
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    #[inline]
    pub fn cursor_pos_mut(&mut self) -> &mut usize {
        self.cursor_target_x_px = None;
        &mut self.cursor_pos
    }

    #[inline]
    pub fn highlight_range(&self) -> Range<usize> {
        self.highlight_range.clone()
    }

    pub fn move_cursor_vertical(&mut self, dist: isize, expand_selection: bool) {
        let cursor_start_pos = self.cursor_pos;

        let mut cursor_pos = self.cursor_pos;
        let mut cursor_target_x_px = self.cursor_target_x_px;

        macro_rules! search_for_glyph {
            ($iter:expr) => {{
                let cursor_pos_move = cursor_pos;
                let mut glyph_iter = $iter.skip_while(move |g| g.str_index != cursor_pos_move);
                if let Some(cursor_glyph) = glyph_iter.next() {
                    let cursor_pos_px = cursor_glyph.highlight_rect.min;
                    if cursor_target_x_px.is_none() {
                        cursor_target_x_px = Some(cursor_pos_px.x);
                    }
                    let target_x_px = cursor_target_x_px.unwrap();

                    let mut min_dist_x = i32::max_value();
                    let mut cur_line_y = cursor_glyph.highlight_rect.min.y;
                    let mut line_delta = 0;

                    for glyph in glyph_iter {
                        let glyph_dist_x = (target_x_px - glyph.highlight_rect.min.x).abs();
                        if glyph.highlight_rect.min.y != cur_line_y {
                            line_delta += 1;
                            cur_line_y = glyph.highlight_rect.min.y;
                            if line_delta > dist.abs() {
                                break;
                            }

                            min_dist_x = glyph_dist_x;
                            cursor_pos = glyph.str_index;

                            continue;
                        }
                        if line_delta == 0 {
                            continue;
                        }

                        if glyph_dist_x < min_dist_x {
                            min_dist_x = glyph_dist_x;
                            cursor_pos = glyph.str_index;
                        }
                    }
                }
            }}
        }

        let glyph_iter = self.glyph_iter();
        match dist.signum() {
             0 => return,
             1 => search_for_glyph!(glyph_iter),
            -1 => search_for_glyph!(glyph_iter.rev()),
            _ => unreachable!()
        }
        self.cursor_pos = cursor_pos;
        self.cursor_target_x_px = cursor_target_x_px;

        if expand_selection {
            self.expand_selection_to_cursor(cursor_start_pos);
        } else {
            self.highlight_range = 0..0;
        }
    }

    pub fn move_cursor_horizontal(&mut self, dist: isize, jump_to_word_boundaries: bool, expand_selection: bool) {
        let cursor_start_pos = self.cursor_pos;
        self.cursor_target_x_px = None;
        self.cursor_pos = match (self.highlight_range.len() * !expand_selection as usize, dist.signum(), jump_to_word_boundaries) {
            (_, 0, _) => return,
            (0, 1, false) =>
                self.string[self.cursor_pos..].grapheme_indices(true)
                    .skip(dist as usize).map(|(i, _)| i + self.cursor_pos)
                    .next().unwrap_or(self.string.len()),
            (0, -1, false) =>
                self.string[..self.cursor_pos].grapheme_indices(true)
                    .rev().skip(dist.abs() as usize - 1).map(|(i, _)| i)
                    .next().unwrap_or(0),
            (0, 1, true) =>
                self.string[self.cursor_pos..].unicode_words()
                .skip(dist as usize).next()
                .map(|word| word.as_ptr() as usize - self.string.as_ptr() as usize)
                .unwrap_or(self.string.len()),
            (0, -1, true) => self.string[..self.cursor_pos].unicode_words()
                .rev().skip(dist.abs() as usize - 1).next()
                .map(|word| word.as_ptr() as usize - self.string.as_ptr() as usize)
                .unwrap_or(0),
            (_, 1, _) => self.highlight_range.end,
            (_, -1, _) => self.highlight_range.start,
            _ => unreachable!()
        };
        if expand_selection {
            self.expand_selection_to_cursor(cursor_start_pos);
        } else {
            self.highlight_range = 0..0;
        }
    }

    fn expand_selection_to_cursor(&mut self, cursor_start_pos: usize) {
        if self.highlight_range.len() == 0 {
            self.highlight_range = cursor_start_pos..cursor_start_pos;
        }

        match (cursor_start_pos == self.highlight_range.start, self.cursor_pos < self.highlight_range.end) {
            (false, true) if self.cursor_pos < self.highlight_range.start => {
                self.highlight_range.end = self.highlight_range.start;
                self.highlight_range.start = self.cursor_pos;
            }
            (false, _) => self.highlight_range.end = self.cursor_pos,
            (true, true) => self.highlight_range.start = self.cursor_pos,
            (true, false) => {
                self.highlight_range.start = self.highlight_range.end;
                self.highlight_range.end = self.cursor_pos;
            }
        }
    }

    fn draw_cursor(&self) -> bool {
        self.draw_cursor && self.highlight_range.len() == 0
    }

    pub fn select_on_line(&mut self, mut segment: Segment<D2, i32>) {
        let shaped_glyphs = match self.draw_data {
            Some(ref draw_data) => &draw_data.shaped_glyphs,
            None => {self.highlight_range = 0..0; return}
        };

        // let mut min_y_dist = None;
        let dist = |min: i32, max: i32, point: i32| match (min.cmp(&point), max.cmp(&point)) {
            (Ordering::Equal, _) |
            (_, Ordering::Equal) |
            (Ordering::Less, Ordering::Greater) => 0,
            (Ordering::Greater, _) => min - point,
            (_, Ordering::Less) => point - min
        };


        let (mut min_start_x_dist, mut min_start_y_dist) = (i32::max_value(), i32::max_value());
        let (mut min_end_x_dist, mut min_end_y_dist) = (i32::max_value(), i32::max_value());
        let (mut start_index, mut end_index) = (0, 0);

        // Offset the segment so that we're properly selecting offset text.
        segment = Segment::new(segment.start - self.offset, segment.end - self.offset);

        for glyph in shaped_glyphs.iter() {
            let x_dist = |point: Point2<_>| dist(glyph.highlight_rect.min.x, glyph.highlight_rect.max.x, point.x);
            let y_dist = |point: Point2<_>| dist(glyph.highlight_rect.min.y, glyph.highlight_rect.max.y, point.y);
            let glyph_start_x_dist = x_dist(segment.start);
            let glyph_start_y_dist = y_dist(segment.start);
            let glyph_end_x_dist = x_dist(segment.end);
            let glyph_end_y_dist = y_dist(segment.end);
            let highlight_center = glyph.highlight_rect.center();

            if glyph_start_y_dist < min_start_y_dist {
                min_start_y_dist = glyph_start_y_dist;
                min_start_x_dist = glyph_start_x_dist;
                start_index = glyph.str_index + (highlight_center.x <= segment.start.x) as usize;
            }
            if glyph_end_y_dist < min_end_y_dist {
                min_end_y_dist = glyph_end_y_dist;
                min_end_x_dist = glyph_end_x_dist;
                end_index = glyph.str_index + (highlight_center.x <= segment.end.x) as usize;
            }
            if glyph_start_x_dist < min_start_x_dist && glyph_start_y_dist <= min_start_y_dist {
                min_start_x_dist = glyph_start_x_dist;
                start_index = glyph.str_index + (highlight_center.x <= segment.start.x) as usize;
            }
            if glyph_end_x_dist < min_end_x_dist && glyph_end_y_dist <= min_end_y_dist {
                min_end_x_dist = glyph_end_x_dist;
                end_index = glyph.str_index + (highlight_center.x <= segment.end.x) as usize;
            }
        }

        self.highlight_range = cmp::min(start_index, end_index)..cmp::max(start_index, end_index);
        self.cursor_pos = end_index;
    }

    pub fn select_all(&mut self) {
        self.highlight_range = 0..self.string.len();
        self.cursor_pos = self.highlight_range.end;
    }

    pub fn deselect_all(&mut self) {
        self.highlight_range = 0..0;
    }

    pub fn insert_char(&mut self, c: char) {
        if self.highlight_range.len() != 0 {
            let highlight_range = self.highlight_range.clone();
            self.string_mut().drain(highlight_range);
            self.cursor_pos = self.highlight_range.start;
            self.highlight_range = 0..0;
        }
        let cursor_pos = self.cursor_pos;
        self.string_mut().insert(cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn insert_str(&mut self, s: &str) {
        if self.highlight_range.len() != 0 {
            let highlight_range = self.highlight_range.clone();
            self.string_mut().drain(highlight_range);
            self.cursor_pos = self.highlight_range.start;
            self.highlight_range = 0..0;
        }
        let cursor_pos = self.cursor_pos;
        self.string_mut().insert_str(cursor_pos, s);
        self.cursor_pos += s.len();
    }

    pub fn delete_chars(&mut self, dist: isize, jump_to_word_boundaries: bool) {
        let drain_range = if self.highlight_range.len() != 0 {
            self.highlight_range.clone()
        } else {
            let old_pos = self.cursor_pos;
            self.move_cursor_horizontal(dist, jump_to_word_boundaries, false);
            let new_pos = self.cursor_pos;
            cmp::min(old_pos, new_pos)..cmp::max(old_pos, new_pos)
        };
        self.string_mut().drain(drain_range.clone());
        self.highlight_range = 0..0;
        self.cursor_pos = drain_range.start;
    }
}

impl RenderGlyph {
    fn offset(mut self, offset: Vector2<i32>) -> RenderGlyph {
        self.pos += offset;
        self.highlight_rect = self.highlight_rect + offset;
        self
    }
}
