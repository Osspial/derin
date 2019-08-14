use cgmath_geometry::{
    D2,
    cgmath::Point2,
    rect::{BoundBox, DimsBox, GeoBox},
};
use crate::{
    Content,
    LayoutContent,
    LayoutResult,
    rect_layout::{
        self,
        Rect,
        ImageManager,
        ImageLayout,
        ImageLayoutData,
        TextLayoutData,
        text::{FaceManager, StringLayoutData},
        theme::{Color, ImageId, WidgetStyle},
    },
    theme::Theme,
};
use derin_core::{
    render::{DisplayEngine, DisplayEngineLayoutRender},
    widget::{WidgetId, WidgetPathEntry},
};
use derin_common_types::layout::SizeBounds;
use glutin::{
    ContextWrapper, PossiblyCurrent,
    dpi::PhysicalSize,
    window::Window,
};
use gullery::{
    ContextState,
    framebuffer::{Framebuffer, FramebufferDefault},
    texture::Texture,
    image_format::{Rgba, SRgba},
};
use std::{
    collections::HashMap,
    rc::Rc,
};

pub trait FaceRasterizer: FaceManager + for<'a> Rasterizer<'a> { }
pub trait ImageRasterizer: ImageManager + for<'a> Rasterizer<'a> { }
pub trait Rasterizer<'a> {
    type PixelIter: 'a + Iterator<Item=Color>;
    fn rasterize(&'a mut self, image: ImageId) -> Option<(DimsBox<D2, u32>, Self::PixelIter)>;
}

struct Vertex {
    pixel: Point2<u16>,
    texture: u8,
    texture_coordinate: Point2<u16>,
}

pub struct GulleryDisplayEngine<T, F, I>
    where T: 'static + Theme<Style=WidgetStyle>,
          F: FaceRasterizer,
          I: ImageManager,
{
    pub theme: T,
    pub face_rasterizer: F,
    pub image_manager: I,
    window: ContextWrapper<PossiblyCurrent, Window>,
    context_state: Rc<ContextState>,
    framebuffer: FramebufferDefault,
    vertex_buffer: Buffer<Vertex>,
    index_buffer: Buffer<u16>,
    textures: HashMap<ImageId, Texture<D2, SRgba>>,
    widget_rects: HashMap<WidgetId, Vec<Rect>>,
}

impl<T, F, I> GulleryDisplayEngine<T, F, I>
    where T: Theme<Style=WidgetStyle>,
          F: FaceRasterizer,
          I: ImageManager,
{
    fn dpi(&self) -> u32 {
        (self.window.window().hidpi_factor() * 72.0).round() as u32
    }
}

impl<T, F, I> DisplayEngine for GulleryDisplayEngine<T, F, I>
    where T: Theme<Style=WidgetStyle>,
          F: FaceRasterizer,
          I: ImageManager,
{
    fn resized(&mut self, new_size: DimsBox<D2, u32>) {
        self.window.resize(PhysicalSize::new(new_size.width() as f64, new_size.height() as f64));
    }
    fn dims(&self) -> DimsBox<D2, u32> {
        let size = self.window.window().inner_size();
        DimsBox::new2(size.width as u32, size.height as u32)
    }
    fn widget_removed(&mut self, widget_id: WidgetId) {
        self.widget_rects.remove(&widget_id);
    }
    fn start_frame(&mut self) {
        self.framebuffer.clear_color_all(Rgba::new(0.0, 0.0, 0.0, 1.0));
    }
    fn finish_frame(&mut self) {}
}
impl<'d, T, F, I> DisplayEngineLayoutRender<'d> for GulleryDisplayEngine<T, F, I>
    where T: Theme<Style=WidgetStyle>,
          F: FaceRasterizer,
          I: ImageManager,
{
    type Layout = GulleryLayout<'d, T, F, I>;
    type Renderer = GulleryRenderer<'d>;

    fn layout(
        &'d mut self,
        widget_path: &'d [WidgetPathEntry],
        dims: DimsBox<D2, i32>
    ) -> Self::Layout {
        GulleryLayout {
            path: widget_path,
            dpi: self.dpi(),
            theme: &mut self.theme,
            face_rasterizer: &mut self.face_rasterizer,
            image_manager: &mut self.image_manager,
            widget_rects: &mut self.widget_rects,
            widget_dims: dims,
        }
    }

    fn render(
        &'d mut self,
        widget_id: WidgetId,
        position: Point2<i32>,
        clip: BoundBox<D2, i32>
    ) -> Self::Renderer {
        unimplemented!()
    }
}

pub struct GulleryLayout<'d, T, F, I>
    where T: Theme<Style=WidgetStyle>,
          F: FaceRasterizer,
          I: ImageManager,
{
    path: &'d [WidgetPathEntry],
    theme: &'d mut T,
    face_rasterizer: &'d mut F,
    image_manager: &'d mut I,
    widget_rects: &'d mut HashMap<WidgetId, Vec<Rect>>,
    dpi: u32,
    widget_dims: DimsBox<D2, i32>,

}
pub struct GulleryRenderer<'d> {_ref: &'d ()}

impl<'d, T, F, I> LayoutContent for GulleryLayout<'d, T, F, I>
    where T: Theme<Style=WidgetStyle>,
          F: FaceRasterizer,
          I: ImageManager,
{
    fn layout_content<C: Content>(self, content: &C) -> LayoutResult {
        let widget_id = self.path.last().unwrap().widget_id;

        self.theme.set_widget_content(self.path, content);
        let WidgetStyle {
            background: style_background,
            text: style_text,
            content_margins,
            size_bounds: widget_size_bounds,
        } = self.theme.style(widget_id);
        let text_rect = style_text.margins.apply(self.widget_dims.into());

        let mut image_size_bounds = SizeBounds::default();
        let image_layout_data = try {
            let image_id = style_background?;
            let ImageLayout {
                rescale,
                dims,
                size_bounds,
                margins,
            } = self.image_manager.image_layout(image_id);
            image_size_bounds = size_bounds;

            ImageLayoutData {
                image_id,
                rect: margins.apply(self.widget_dims.into()),
                rescale,
                dims,
            }
        };

        let mut text_size_bounds = SizeBounds::default();
        let string_layout: StringLayoutData;
        let text_layout_data: Option<TextLayoutData> = try {
            let render_string = content.string()?;
            string_layout = StringLayoutData::shape(
                render_string.string,
                self.widget_dims,
                self.dpi,
                style_text.layout,
                self.face_rasterizer
            );

            text_size_bounds.min = string_layout.min_size().unwrap_or(DimsBox::new2(0, 0));

            TextLayoutData {
                string_layout: &string_layout,
                decorations: render_string.decorations,
                render_style: style_text.render,
                offset: render_string.offset,
                clip_rect: text_rect,
            }
        };

        let rect_iter = rect_layout::layout_widget_rects(
            image_layout_data,
            text_layout_data,
            self.face_rasterizer,
        );
        self.widget_rects.entry(widget_id)
            .and_modify(|v| v.clear())
            .or_insert(Vec::new())
            .extend(rect_iter);

        LayoutResult {
            size_bounds: widget_size_bounds
                .union(image_size_bounds)
                .and_then(|sb| sb.union(text_size_bounds))
                .unwrap_or(SizeBounds::default()),
            content_rect: content_margins.apply(self.widget_dims.into()),
        }
    }
}

// impl<'d> LayoutString for GulleryLayout<'d> {
//     fn layout_string<C: Content>(
//         &mut self,
//         content: &C,
//         grapheme_clusters: &mut Vec<GraphemeCluster>
//     ) {
//         self.theme.set_widget_content(self.path, content);
//         unimplemented!()
//     }
// }
