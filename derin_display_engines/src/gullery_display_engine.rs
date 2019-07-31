use cgmath_geometry::{
    D2,
    cgmath::Point2,
    rect::{BoundBox, DimsBox, GeoBox},
};
use crate::{
    Content,
    GraphemeCluster,
    LayoutContent,
    LayoutResult,
    LayoutString,
    rect_layout::{
        Rect,
        text::FaceManager,
        theme::{Color, ImageId, WidgetStyle},
    },
    theme::Theme,
};
use derin_core::{
    render::{DisplayEngine, DisplayEngineLayoutRender},
    widget::{WidgetId, WidgetPathEntry},
};
use glutin::{
    ContextWrapper, PossiblyCurrent,
    dpi::PhysicalSize,
    window::Window,
};
use gullery::{
    ContextState,
    framebuffer::FramebufferDefault,
    texture::Texture,
    image_format::SRgba,
};
use std::{
    collections::HashMap,
    rc::Rc,
};

pub trait FaceRasterizer: FaceManager + for<'a> FaceRasterizerRasterize<'a> { }
pub trait FaceRasterizerRasterize<'a> {
    type PixelIter: 'a + Iterator<Item=Color>;
    fn rasterize_glyph(&'a mut self, glyph_image: ImageId) -> Option<(DimsBox<D2, u32>, Self::PixelIter)>;
}

pub struct GulleryDisplayEngine<T>
    where T: 'static + Theme<Style=WidgetStyle>,
{
    pub theme: T,
    window: ContextWrapper<PossiblyCurrent, Window>,
    context_state: Rc<ContextState>,
    framebuffer: FramebufferDefault,
    textures: HashMap<ImageId, Texture<D2, SRgba>>,
    widgets: HashMap<WidgetId, Vec<Rect>>,
}

impl<T> GulleryDisplayEngine<T>
    where T: Theme<Style=WidgetStyle>,
{
    fn dpi(&self) -> u32 {
        (self.window.window().hidpi_factor() * 72.0).round() as u32
    }
}

impl<T: Theme<Style=WidgetStyle>> DisplayEngine for GulleryDisplayEngine<T> {
    fn resized(&mut self, new_size: DimsBox<D2, u32>) {
        self.window.resize(PhysicalSize::new(new_size.width() as f64, new_size.height() as f64));
    }
    fn dims(&self) -> DimsBox<D2, u32> {
        let size = self.window.window().inner_size();
        DimsBox::new2(size.width as u32, size.height as u32)
    }
    fn widget_removed(&mut self, widget_id: WidgetId) {
        self.widgets.remove(&widget_id);
    }
    fn start_frame(&mut self) {}
    fn finish_frame(&mut self) {}
}
impl<'d, T: Theme<Style=WidgetStyle>> DisplayEngineLayoutRender<'d> for GulleryDisplayEngine<T> {
    type Layout = GulleryLayout<'d, T>;
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

pub struct GulleryLayout<'d, T: Theme<Style=WidgetStyle>> {
    path: &'d [WidgetPathEntry],
    theme: &'d mut T,
    dpi: u32,
    widget_dims: DimsBox<D2, i32>,

}
pub struct GulleryRenderer<'d> {_ref: &'d ()}

// impl<'d> LayoutContent for GulleryLayout<'d> {
//     fn layout_content<C: Content>(self, content: &C) -> LayoutResult {
//         unimplemented!()
//     }
// }

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
