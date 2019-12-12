use crate::{
    HasLifetimeIterator,
    gullery_display_engine::FaceRasterizer,
    rect_layout::{
        text::{FaceManager, FaceMetrics, Glyph},
        theme::{Color, FontFaceId, ImageId},
    }
};
use crate::cgmath::{Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox, DimsBox}};
use font_kit::{
    canvas::{Canvas, Format, RasterizationOptions},
    family_name::FamilyName,
    hinting::HintingOptions,
    loader::FontTransform,
    loaders::default::Font,
    properties::Properties,
    source::SystemSource,
};
use std::{cmp, collections::HashMap};
use euclid::default::{Point2D, Size2D};

pub struct FontKitFaceManager {
    source: SystemSource,
    font_cache: HashMap<FontFaceId, Font>,
    map_glyph_image_id: HashMap<GlyphProperties, (ImageId, BoundBox<D2, i32>)>,
    map_image_id_glyph: HashMap<ImageId, (GlyphProperties, BoundBox<D2, i32>)>,
    canvas: Canvas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphProperties {
    face: FontFaceId,
    size_16_16: u32,
    glyph_index: u32,
    color: Color,
}

fn calculate_px_per_em_16_16(pt_per_em: u32, dpi: u32) -> u32 {
    pt_per_em * dpi / 72
}
fn calculate_px_per_em(pt_per_em: u32, dpi: u32) -> f32 {
    calculate_px_per_em_16_16(pt_per_em, dpi) as f32 / 64.0
}
fn font_units_to_px(px_per_em: f32, units_per_em: u32, font_units: f32) -> f32 {
    font_units * px_per_em / (units_per_em as f32)
}
fn px_16_16_to_f32(px: u32) -> f32 {
    px as f32 / 64.0
}
fn px_f32_to_16_16(px: f32) -> i32 {
    (px * 64.0).round() as i32
}

impl FaceManager for FontKitFaceManager {
    type FaceQuery = FamilyName;
    fn query_face(&mut self, face_query: FamilyName) -> Option<FontFaceId> {
        self.query_face_best_match(&[face_query])
    }
    fn query_face_best_match(
        &mut self,
        face_query: &[Self::FaceQuery]
    ) -> Option<FontFaceId> {
        // TODO: MAKE PROPERTIES SELECTABLE BY USER
        let properties = Properties::new();
        let font = self.source
            .select_best_match(face_query, &properties).ok()?
            .load().ok()?;
        let font_id = FontFaceId::new();
        self.font_cache.insert(font_id, font);
        Some(font_id)
    }

    fn face_metrics(
        &mut self,
        face_id: FontFaceId,
        face_size: u32,
        dpi: u32
    ) -> FaceMetrics
    {
        let font = self.font_cache.get(&face_id).expect("TODO DUMMY FACE METRICS");
        let metrics = font.metrics();
        let px_per_em = calculate_px_per_em(face_size, dpi);
        let transform = |f| px_f32_to_16_16(font_units_to_px(px_per_em, metrics.units_per_em, f));
        let space_advance = font.glyph_for_char(' ')
            .and_then(|g| font.raster_bounds(
                g,
                px_per_em,
                &FontTransform::new(0., 0., 0., 0.),
                &Point2D::new(0., 0.),
                HintingOptions::Full(px_per_em),
                RasterizationOptions::GrayscaleAa).ok())
            .map(|r| r.max().x)
            .unwrap_or(0);

        FaceMetrics {
            line_height: transform(metrics.line_gap),
            ascender: transform(metrics.ascent),
            descender: transform(metrics.descent),
            space_advance,
            cursor_width: transform(1.0),
        }
    }
    fn glyph_image(
        &mut self,
        face: FontFaceId,
        face_size: u32,
        dpi: u32,
        glyph_index: u32,
        color: Color
    ) -> (ImageId, BoundBox<D2, i32>)
    {
        let size_16_16 = calculate_px_per_em_16_16(face_size, dpi);
        let glyph_properties = GlyphProperties {
            face,
            size_16_16,
            glyph_index,
            color,
        };
        if let Some(pair) = self.map_glyph_image_id.get(&glyph_properties) {
            return *pair;
        }

        let px_per_em = px_16_16_to_f32(size_16_16);
        let rect = self.font_cache.get(&face)
            .and_then(|face| face.raster_bounds(
                glyph_index,
                px_per_em,
                &FontTransform::new(0., 0., 0., 0.),
                &Point2D::new(0., 0.),
                HintingOptions::Full(px_per_em),
                RasterizationOptions::GrayscaleAa).ok())
            .map(|r| BoundBox::new2(r.min_x(), r.min_y(), r.max_x(), r.max_y()))
            .unwrap_or(BoundBox::new2(0, 0, 0, 0));

        let image_id = ImageId::new();
        self.map_glyph_image_id.insert(glyph_properties, (image_id, rect));
        self.map_image_id_glyph.insert(image_id, (glyph_properties, rect));
        (image_id, rect)
    }
    fn shape_text<'a>(
        &'a mut self,
        face: FontFaceId,
        face_size: u32,
        dpi: u32,
        text: &'a str
    ) -> <Self as HasLifetimeIterator<'a, Glyph>>::Iter
    {
        // TODO: REPLACE WITH HARFBUZZ
        let px_per_em = calculate_px_per_em(face_size, dpi);
        let face = self.font_cache.get(&face);
        ShapeTextGlyphIter {
            cursor: 0,
            px_per_em,
            face,
            char_indices: text.char_indices(),
        }
    }
}

impl<'a> HasLifetimeIterator<'a, Glyph> for FontKitFaceManager {
    type Iter = ShapeTextGlyphIter<'a>;
}

impl FaceRasterizer for FontKitFaceManager {
    fn rasterize(&mut self, image: ImageId) -> Option<(DimsBox<D2, u32>, GlyphRasterizeIter<'_>)> {
        let (glyph_properties, bounds) = self.map_image_id_glyph.get(&image)?;
        let face = self.font_cache.get(&glyph_properties.face)?;
        let glyph_width = bounds.width() as u32;
        let glyph_height = bounds.height() as u32;
        if self.canvas.size.width < glyph_width || self.canvas.size.height < glyph_height {
            self.canvas = Canvas::new(
                &Size2D::new(
                    cmp::max(glyph_width, self.canvas.size.width),
                    cmp::max(glyph_height, self.canvas.size.height),
                ),
                Format::A8,
            );
        }

        let px_per_em = px_16_16_to_f32(glyph_properties.size_16_16);
        face.rasterize_glyph(
            &mut self.canvas,
            glyph_properties.glyph_index,
            px_per_em,
            &FontTransform::new(0., 0., 0., 0.),
            &Point2D::new(0., 0.),
            HintingOptions::Full(px_per_em),
            RasterizationOptions::GrayscaleAa
        ).ok()?;

        Some((DimsBox::new2(glyph_width, glyph_height), self.canvas.pixels.iter().cloned().map(|v| Color::new(v, v, v, v)))) // terry cavanagh game prototype
    }
}

impl<'a> HasLifetimeIterator<'a, Color> for FontKitFaceManager {
    type Iter = GlyphRasterizeIter<'a>;
}

pub struct ShapeTextGlyphIter<'a> {
    cursor: i32,
    px_per_em: f32,
    face: Option<&'a Font>,
    char_indices: std::str::CharIndices<'a>,
}

impl<'a> Iterator for ShapeTextGlyphIter<'a> {
    type Item = Glyph;

    fn next(&mut self) -> Option<Glyph> {
        let (str_index, char) = self.char_indices.next()?;
        let glyph_index = self.face?.glyph_for_char(char).unwrap_or(0);
        let bounds = self.face?.raster_bounds(
                glyph_index,
                self.px_per_em,
                &FontTransform::new(0., 0., 0., 0.),
                &Point2D::new(0., 0.),
                HintingOptions::Full(self.px_per_em),
                RasterizationOptions::GrayscaleAa
            )
            .map(|r| BoundBox::new2(r.min_x(), r.min_y(), r.max_x(), r.max_y()))
            .unwrap_or(BoundBox::new2(0, 0, 0, 0));
        let pos = Point2::new(self.cursor, 0);
        let advance = Vector2::new(bounds.width(), 0);
        self.cursor += advance.x;
        Some(Glyph {
            glyph_index,
            advance,
            pos,
            str_index,
        })
    }
}

pub type GlyphRasterizeIter<'a> = impl 'a + Iterator<Item=Color>;
