use crate::{
    HasLifetimeIterator,
    rect_layout::{
        text::{FaceManager, FaceMetrics, Glyph},
        theme::{Color, FontFaceId, ImageId},
    }
};
use cgmath_geometry::{D2, rect::BoundBox};
use font_kit::{
    canvas::RasterizationOptions,
    family_name::FamilyName,
    hinting::HintingOptions,
    loader::FontTransform,
    loaders::default::Font,
    properties::Properties,
    source::SystemSource,
};
use std::collections::HashMap;
use euclid::default::Point2D;

pub struct FontKitFaceManager {
    source: SystemSource,
    font_cache: HashMap<FontFaceId, Font>,
    map_glyph_image_id: HashMap<GlyphProperties, (ImageId, BoundBox<D2, i32>)>,
    map_image_id_glyph: HashMap<ImageId, (GlyphProperties, BoundBox<D2, i32>)>,
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
        face_id: FontFaceId,
        face_size: u32,
        text: &str
    ) -> <Self as HasLifetimeIterator<'a, Glyph>>::Iter
    {
        unimplemented!()
    }
}

impl<'a> HasLifetimeIterator<'a, Glyph> for FontKitFaceManager {
    type Iter = std::iter::Once<Glyph>;
}
