pub mod img;
pub mod text;
pub mod vs;

use self::vs::{Part, ThemeClass};
use self::text::*;
use self::img::*;

use dct::geometry::{Px, Point, OriginRect, OffsetRect, Rect};

use winapi::*;
use user32;
use gdi32;
use uxtheme;

use std::{mem, ptr, char};
use std::marker::PhantomData;
use std::io;

use ucs2::{UCS2_CONVERTER, WithString, Ucs2Str};

macro_rules! context_wrapper {
    ($(pub struct $context_name:ident;)+) => {$(
        #[derive(Debug)]
        pub struct $context_name( HDC, HWND );

        unsafe impl DeviceContext for $context_name {
            fn hdc(&self) -> HDC {
                self.0
            }

            fn hwnd(&self) -> HWND {
                self.1
            }
        }
    )*}
}

#[derive(Debug)]
pub struct PaintInit<'a>( HWND, PhantomData<&'a ()> );

context_wrapper!{
    pub struct RetrievedContext;
    pub struct BufferedContext;
    pub struct MemoryContext;
}
#[derive(Debug)]
pub struct PaintContext( PAINTSTRUCT, HWND );

thread_local!{
    /// See ThreadBufferedPaint docs for details
    static THREAD_BUFFERED_PAINT: ThreadBufferedPaint = ThreadBufferedPaint::new();
}

pub unsafe trait DeviceContext {
    fn hdc(&self) -> HDC;
    fn hwnd(&self) -> HWND;

    #[inline]
    fn with_font<F, R>(&self, font: &Font, run: F) -> R
            where F: FnOnce(&Self) -> R
    {
        unsafe {
            let old_font = gdi32::SelectObject(self.hdc(), font.hfont() as HGDIOBJ);
            let ret = run(self);
            gdi32::SelectObject(self.hdc(), old_font);
            ret
        }
    }

    #[inline]
    fn with_bmp<F, R>(&self, bmp: &Bitmap, run: F) -> R
            where F: FnOnce(&Self) -> R
    {
        unsafe {
            let old_bmp = gdi32::SelectObject(self.hdc(), bmp.hbitmap() as HGDIOBJ);
            if old_bmp == ptr::null_mut() {
                panic!("Bitmaps can only be selected by one DC at a time");
            }

            let ret = run(self);
            gdi32::SelectObject(self.hdc(), old_bmp);
            ret
        }
    }

    /// Create a `MemoryContext` that is compatible with this context.
    #[inline]
    fn create_compatible_dc(&self) -> MemoryContext {
        let dc = unsafe{ gdi32::CreateCompatibleDC(self.hdc()) };
        MemoryContext(dc, self.hwnd())
    }

    /// Blit the contents of the given rectangle in the source DC to the location `dest` in the
    /// `self` dc.
    #[inline]
    fn bit_copy<C: DeviceContext>(&self, src_dc: &C, src_rect: OffsetRect, dest: Point) -> io::Result<()> {
        self.bit_blt(src_dc, src_rect, dest, RopCode::SrcCopy)
    }

    #[inline]
    fn bit_blt<C: DeviceContext>(&self, src_dc: &C, src_rect: OffsetRect, dest: Point, rop: RopCode) -> io::Result<()> {
        let (width, height) = (src_rect.width(), src_rect.height());
        let result = unsafe{ gdi32::BitBlt(
            self.hdc(),
            dest.x,
            dest.y,
            width,
            height,
            src_dc.hdc(),
            src_rect.topleft.x,
            src_rect.topleft.y,
            mem::transmute(rop)
        ) };

        if result != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[inline]
    fn alpha_blend<C: DeviceContext>(
        &self, src_dc: &C, src_rect: OffsetRect, dest_rect: OffsetRect, const_alpha: u8,
        alpha_format: AlphaFormat
    ) -> Result<(), ()> {
        let blend_function = BLENDFUNCTION {
            BlendOp: 0, // AC_SRC_OVER
            BlendFlags: 0,
            SourceConstantAlpha: const_alpha,
            AlphaFormat: unsafe{ mem::transmute(alpha_format) }
        };
        let result = unsafe{ gdi32::GdiAlphaBlend(
            self.hdc(),
            dest_rect.topleft().x,
            dest_rect.topleft().y,
            dest_rect.width(),
            dest_rect.height(),
            src_dc.hdc(),
            src_rect.topleft().x,
            src_rect.topleft().y,
            src_rect.width(),
            src_rect.height(),
            blend_function
        ) };

        if result == TRUE {
            Ok(())
        } else {
            Err(())
        }
    }

    #[inline]
    fn draw_text(&self, text: &str, rect: OffsetRect, text_format: TextFormat) -> OffsetRect {
        UCS2_CONVERTER.with_string(text, |text_ucs2| unsafe {
            self.draw_text_ucs2(text_ucs2, rect, text_format)
        })
    }

    #[inline]
    fn calc_text_rect(&self, text: &str, text_format: TextFormat) -> OriginRect {
        UCS2_CONVERTER.with_string(text, |text_ucs2| unsafe {
            self.calc_text_rect_ucs2(text_ucs2, text_format)
        })
    }

    #[inline]
    fn draw_theme_background<T, P>(&self, theme: &T, part: P, rect: OffsetRect, clip_rect: Option<OffsetRect>)
            where T: ThemeClass<P>,
                  P: Part
    {
        let rect_winapi = RECT {
            left: rect.topleft.x as LONG,
            top: rect.topleft.y as LONG,
            right: rect.lowright.x as LONG,
            bottom: rect.lowright.y as LONG
        };

        let clip_rect_winapi: RECT;
        let clip_rect_ptr = if let Some(clip_rect) = clip_rect {
            clip_rect_winapi = RECT {
                left: clip_rect.topleft.x as LONG,
                top: clip_rect.topleft.y as LONG,
                right: clip_rect.lowright.x as LONG,
                bottom: clip_rect.lowright.y as LONG
            };
            &clip_rect_winapi
        } else {ptr::null()};

        unsafe{ uxtheme::DrawThemeBackground(
            theme.htheme(),
            self.hdc(),
            part.part_id(),
            part.state_id(),
            &rect_winapi,
            clip_rect_ptr
        ) };
    }

    #[inline]
    fn draw_theme_text<T, P>(&self, theme: &T, part: P, text: &str, rect: OffsetRect, text_format: TextFormat)
            where T: ThemeClass<P>,
                  P: Part
    {
        UCS2_CONVERTER.with_string(text, |text_ucs2| unsafe {
            self.draw_theme_text_ucs2(theme, part, text_ucs2, rect, text_format)
        })
    }

    #[inline]
    fn calc_theme_text_rect<T, P>(
        &self, theme: &T, part: P, text: &str, text_format: TextFormat
    ) -> OffsetRect
            where T: ThemeClass<P>,
                  P: Part
    {
        UCS2_CONVERTER.with_string(text, |text_ucs2| unsafe {
            self.calc_theme_text_rect_ucs2(theme, part, text_ucs2, text_format)
        })
    }

    #[inline]
    fn calc_theme_content_rect<T, P>(&self, theme: &T, part: P, background_rect: OffsetRect) -> OffsetRect
            where T: ThemeClass<P>,
                  P: Part
    {
        let mut background_rect = RECT {
            left: background_rect.topleft.x as LONG,
            top: background_rect.topleft.y as LONG,
            right: background_rect.lowright.x as LONG,
            bottom: background_rect.lowright.y as LONG
        };
        let mut content_rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0
        };

        unsafe{ uxtheme::GetThemeBackgroundContentRect(
            theme.htheme(),
            self.hdc(),
            part.part_id(),
            part.state_id(),
            &mut background_rect,
            &mut content_rect
        ) };

        OffsetRect::new(
            content_rect.left as Px,
            content_rect.top as Px,
            content_rect.right as Px,
            content_rect.bottom as Px
        )
    }

    #[inline]
    fn calc_theme_background_rect<T, P>(&self, theme: &T, part: P, content_rect: OffsetRect) -> OffsetRect
            where T: ThemeClass<P>,
                  P: Part
    {
        let mut content_rect = RECT {
            left: content_rect.topleft.x as LONG,
            top: content_rect.topleft.y as LONG,
            right: content_rect.lowright.x as LONG,
            bottom: content_rect.lowright.y as LONG
        };
        let mut background_rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0
        };

        unsafe{ uxtheme::GetThemeBackgroundExtent(
            theme.htheme(),
            self.hdc(),
            part.part_id(),
            part.state_id(),
            &mut content_rect,
            &mut background_rect
        ) };

        OffsetRect::new(
            background_rect.left as Px,
            background_rect.top as Px,
            background_rect.right as Px,
            background_rect.bottom as Px
        )
    }

    fn begin_buffered_animation<F, G>(&self, rect: OffsetRect, anim_style: AnimStyle, duration: u32, context_from: F, context_into: G)
            where F: FnOnce(&BufferedContext),
                  G: FnOnce(&BufferedContext)
    {
        THREAD_BUFFERED_PAINT.with(|_| {
            let rect = RECT {
                left: rect.topleft.x as LONG,
                top: rect.topleft.y as LONG,
                right: rect.lowright.x as LONG,
                bottom: rect.lowright.y as LONG
            };
            let mut anim_params = BP_ANIMATIONPARAMS {
                cbSize: mem::size_of::<BP_ANIMATIONPARAMS>() as DWORD,
                dwFlags: 0,
                style: match anim_style {
                    AnimStyle::None => BPAS_NONE,
                    AnimStyle::Linear => BPAS_LINEAR,
                    AnimStyle::Cubic => BPAS_CUBIC,
                    AnimStyle::Sine => BPAS_SINE
                },
                dwDuration: duration
            };
            let (mut hdc_from, mut hdc_into) = (0 as HDC, 0 as HDC);
            unsafe {
                let anim_buffer = uxtheme::BeginBufferedAnimation(
                    self.hwnd(),
                    self.hdc(),
                    &rect,
                    BPBF_COMPATIBLEBITMAP,
                    ptr::null_mut(),
                    &mut anim_params,
                    &mut hdc_from,
                    &mut hdc_into
                );

                context_from(&BufferedContext(hdc_from, self.hwnd()));
                context_into(&BufferedContext(hdc_into, self.hwnd()));

                uxtheme::EndBufferedAnimation(anim_buffer, TRUE);
            }
        })
    }

    fn render_buffered_animation(&self) -> bool {
        unsafe{ uxtheme::BufferedPaintRenderAnimation(self.hwnd(), self.hdc()) == TRUE }
    }

    unsafe fn draw_text_ucs2(&self, text_ucs2: &Ucs2Str, rect: OffsetRect, text_format: TextFormat) -> OffsetRect {
        let mut rect = RECT {
            left: rect.topleft.x as LONG,
            top: rect.topleft.y as LONG,
            right: rect.lowright.x as LONG,
            bottom: rect.lowright.y as LONG
        };

        user32::DrawTextW(
            self.hdc(),
            text_ucs2.as_ptr(),
            -1,
            &mut rect,
            text_format.into_text_format()
        );

        OffsetRect::new(rect.left as Px, rect.top as Px, rect.right as Px, rect.bottom as Px)
    }

    unsafe fn calc_text_rect_ucs2(&self, text_ucs2: &Ucs2Str, text_format: TextFormat) -> OriginRect {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0
        };

        user32::DrawTextW(
            self.hdc(),
            text_ucs2.as_ptr(),
            -1,
            &mut rect,
            DT_CALCRECT | text_format.into_text_format()
        );

        OriginRect::new(rect.right as Px, rect.bottom as Px)
    }

    #[inline]
    unsafe fn draw_theme_text_ucs2<T, P>(
        &self, theme: &T, part: P, text: &Ucs2Str, rect: OffsetRect, text_format: TextFormat
    )
            where T: ThemeClass<P>,
                  P: Part
    {
        let mut rect = RECT {
            left: rect.topleft.x as LONG,
            top: rect.topleft.y as LONG,
            right: rect.lowright.x as LONG,
            bottom: rect.lowright.y as LONG
        };

        uxtheme::DrawThemeText(
            theme.htheme(),
            self.hdc(),
            part.part_id(),
            part.state_id(),
            text.as_ptr(),
            -1,
            text_format.into_text_format(),
            0,
            &mut rect
        );
    }

    #[inline]
    unsafe fn calc_theme_text_rect_ucs2<T, P>(
        &self, theme: &T, part: P, text: &Ucs2Str, text_format: TextFormat
    ) -> OffsetRect
            where T: ThemeClass<P>,
                  P: Part
    {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0
        };

        uxtheme::GetThemeTextExtent(
            theme.htheme(),
            self.hdc(),
            part.part_id(),
            part.state_id(),
            text.as_ptr(),
            -1,
            text_format.into_text_format(),
            ptr::null_mut(),
            &mut rect
        );

        OffsetRect::new(rect.left as Px, rect.top as Px, rect.right as Px, rect.bottom as Px)
    }

    #[inline]
    fn font_metrics(&self) -> FontMetrics {
        unsafe {
            let mut text_metrics: TEXTMETRICW = mem::zeroed();
            gdi32::GetTextMetricsW(self.hdc(), &mut text_metrics);

            FontMetrics {
                height: text_metrics.tmHeight as Px,
                ascent: text_metrics.tmAscent as Px,
                descent: text_metrics.tmDescent as Px,
                internal_leading: text_metrics.tmInternalLeading as Px,
                external_leading: text_metrics.tmExternalLeading as Px,
                ave_char_width: text_metrics.tmAveCharWidth as Px,
                max_char_width: text_metrics.tmMaxCharWidth as Px,
                weight: text_metrics.tmWeight as i32,
                overhang: text_metrics.tmOverhang as Px,
                digitized_aspect_x: text_metrics.tmDigitizedAspectX as i32,
                digitized_aspect_y: text_metrics.tmDigitizedAspectY as i32,
                first_char: char::from_u32_unchecked(text_metrics.tmFirstChar as u32),
                last_char: char::from_u32_unchecked(text_metrics.tmLastChar as u32),
                default_char: char::from_u32_unchecked(text_metrics.tmDefaultChar as u32),
                break_char: char::from_u32_unchecked(text_metrics.tmBreakChar as u32),
                italic: 0 != text_metrics.tmItalic,
                underlined: 0 != text_metrics.tmUnderlined,
                struck_out: 0 != text_metrics.tmStruckOut,
                fixed_pitch: 0 != (text_metrics.tmPitchAndFamily & TMPF_FIXED_PITCH),
                vector: 0 != (text_metrics.tmPitchAndFamily & TMPF_VECTOR),
                truetype: 0 != (text_metrics.tmPitchAndFamily & TMPF_TRUETYPE),
                device: 0 != (text_metrics.tmPitchAndFamily & TMPF_DEVICE),
                char_set: match text_metrics.tmCharSet as u32 {
                    ANSI_CHARSET => CharSet::Ansi,
                    BALTIC_CHARSET => CharSet::Baltic,
                    CHINESEBIG5_CHARSET => CharSet::ChineseBig5,
                    DEFAULT_CHARSET => CharSet::Default,
                    EASTEUROPE_CHARSET => CharSet::EastEurope,
                    GB2312_CHARSET => CharSet::Gb2312,
                    GREEK_CHARSET => CharSet::Greek,
                    HANGUL_CHARSET => CharSet::Hangul,
                    MAC_CHARSET => CharSet::Mac,
                    OEM_CHARSET => CharSet::OEM,
                    RUSSIAN_CHARSET => CharSet::Russian,
                    SHIFTJIS_CHARSET => CharSet::ShiftJIS,
                    SYMBOL_CHARSET => CharSet::Symbol,
                    TURKISH_CHARSET => CharSet::Turkish,
                    VIETNAMESE_CHARSET => CharSet::Vietnamese,
                    JOHAB_CHARSET => CharSet::Johab,
                    ARABIC_CHARSET => CharSet::Arabic,
                    HEBREW_CHARSET => CharSet::Hebrew,
                    THAI_CHARSET => CharSet::Thai,
                    _ => panic!("Bad tmCharSet value: {}", text_metrics.tmCharSet)
                }
            }
        }
    }
}

impl<'a> PaintInit<'a> {
    #[inline]
    pub(crate) fn new(hwnd: HWND) -> PaintInit<'a> {
        PaintInit(hwnd, PhantomData)
    }

    pub fn begin_paint(self) -> Option<PaintContext> {
        unsafe {
            let mut paint_info = mem::uninitialized::<PAINTSTRUCT>();
            if ptr::null_mut() != user32::BeginPaint(self.0, &mut paint_info) {
                Some(PaintContext( paint_info, self.0 ))
            } else {
                None
            }
        }
    }
}

impl PaintContext {
    pub fn needs_erase(&self) -> bool {
        self.0.fErase != 0
    }

    pub fn paint_rect(&self) -> OffsetRect {
        OffsetRect::new(
            self.0.rcPaint.left as Px,
            self.0.rcPaint.top as Px,
            self.0.rcPaint.right as Px,
            self.0.rcPaint.bottom as Px
        )
    }
}

impl RetrievedContext {
    pub unsafe fn retrieve_dc(hwnd: HWND) -> Option<RetrievedContext> {
        let hdc = user32::GetDC(hwnd);
        if ptr::null_mut() != hdc {
            Some(RetrievedContext(hdc, hwnd))
        } else {
            None
        }
    }

    pub fn screen_dc() -> Option<RetrievedContext> {
        unsafe{ RetrievedContext::retrieve_dc(ptr::null_mut()) }
    }
}

unsafe impl DeviceContext for PaintContext {
    fn hdc(&self) -> HDC {
        self.0.hdc
    }

    fn hwnd(&self) -> HWND {
        self.1
    }
}

impl Drop for PaintContext {
    fn drop(&mut self) {
        unsafe {
            user32::EndPaint(self.1, &self.0);
        }
    }
}

impl Drop for RetrievedContext {
    fn drop(&mut self) {
        unsafe {
            user32::ReleaseDC(self.1, self.0);
        }
    }
}

impl Drop for MemoryContext {
    fn drop(&mut self) {
        unsafe {
            gdi32::DeleteDC(self.0);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RopCode {
    Blackness = BLACKNESS,
    CaptureBlt = CAPTUREBLT,
    DstInvert = DSTINVERT,
    MergeCopy = MERGECOPY,
    MergePaint = MERGEPAINT,
    NotSrcCopy = NOTSRCCOPY,
    NotSrcErase = NOTSRCERASE,
    PatCopy = PATCOPY,
    PatInvert = PATINVERT,
    PatPaint = PATPAINT,
    SrcAnd = SRCAND,
    SrcCopy = SRCCOPY,
    SrcErase = SRCERASE,
    SrcInvert = SRCINVERT,
    SrcPaint = SRCPAINT,
    Whiteness = WHITENESS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimStyle {
    None,
    Linear,
    Cubic,
    Sine
}

const AC_SRC_ALPHA: BYTE = 0x01;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AlphaFormat {
    PerPixel = AC_SRC_ALPHA,
    Opaque = 0
}

/// Handles initializing and uninitializing thread buffered painting.
struct ThreadBufferedPaint;

impl ThreadBufferedPaint {
    fn new() -> ThreadBufferedPaint {
        unsafe{ uxtheme::BufferedPaintInit() };
        ThreadBufferedPaint
    }
}

impl Drop for ThreadBufferedPaint {
    fn drop(&mut self) {
        unsafe{ uxtheme::BufferedPaintUnInit() };
    }
}
