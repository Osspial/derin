use dct::geometry::{Px, OriginRect, OffsetRect};

use winapi::*;
use user32;
use gdi32;

use std::{mem, ptr, char};

use Window;
use ucs2::{UCS2_CONVERTER, WithString, Ucs2Str};

#[derive(Debug)]
pub struct PaintContext( PAINTSTRUCT, HWND );

impl PaintContext {
    pub fn begin_paint<W: Window>(window: W) -> Option<PaintContext> {
        unsafe {
            let hwnd = window.hwnd();
            let mut paint_info = mem::uninitialized::<PAINTSTRUCT>();
            if ptr::null_mut() != user32::BeginPaint(hwnd, &mut paint_info) {
                Some(PaintContext( paint_info, hwnd ))
            } else {
                None
            }
        }
    }

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

unsafe impl DeviceContext for PaintContext {
    unsafe fn hdc(&self) -> HDC {
        self.0.hdc
    }
}

impl Drop for PaintContext {
    fn drop(&mut self) {
        unsafe {
            user32::EndPaint(self.1, &self.0);
        }
    }
}

pub struct RetrievedContext( HDC, HWND );

impl RetrievedContext {
    pub unsafe fn retrieve_dc(hwnd: HWND) -> Option<RetrievedContext> {
        let hdc = user32::GetDC(hwnd);
        if ptr::null_mut() != hdc {
            Some(RetrievedContext(hdc, hwnd))
        } else {
            None
        }
    }
}

unsafe impl DeviceContext for RetrievedContext {
    unsafe fn hdc(&self) -> HDC {
        self.0
    }
}

impl Drop for RetrievedContext {
    fn drop(&mut self) {
        unsafe {
            user32::ReleaseDC(self.1, self.0);
        }
    }
}

pub unsafe trait DeviceContext {
    unsafe fn hdc(&self) -> HDC;

    fn draw_text(&self, text: &str, rect: OffsetRect, draw_options: TextDrawOptions) -> OffsetRect {
        UCS2_CONVERTER.with_string(text, |text_ucs2| unsafe {
            self.draw_text_raw(text_ucs2, rect, draw_options)
        })
    }

    unsafe fn draw_text_raw(&self, text_ucs2: &Ucs2Str, rect: OffsetRect, draw_options: TextDrawOptions) -> OffsetRect {
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
            draw_options.into_text_format()
        );

        OffsetRect::new(rect.left as Px, rect.top as Px, rect.right as Px, rect.bottom as Px)
    }

    fn calc_text_rect(&self, text: &str, draw_options: TextDrawOptions) -> OriginRect {
        UCS2_CONVERTER.with_string(text, |text_ucs2| unsafe {
            self.calc_text_rect_raw(text_ucs2, draw_options)
        })
    }

    unsafe fn calc_text_rect_raw(&self, text_ucs2: &Ucs2Str, draw_options: TextDrawOptions) -> OriginRect {
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
            DT_CALCRECT | draw_options.into_text_format()
        );

        let ret = OriginRect::new(rect.right as Px, rect.bottom as Px);
        ret
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

#[derive(Debug, Clone, Copy)]
pub struct TextDrawOptions {
    pub expand_tabs: bool,
    pub clip: bool,
    pub proc_prefix: bool,
    pub truncate: Truncate,
    pub alignment: TextAlignment
}

impl TextDrawOptions {
    fn into_text_format(self) -> UINT {
        DT_EXPANDTABS * self.expand_tabs as UINT |
        DT_NOCLIP * !self.clip as UINT |
        DT_NOPREFIX * !self.proc_prefix as UINT |
        match self.truncate {
            Truncate::None => 0,
            Truncate::End  => DT_END_ELLIPSIS,
            Truncate::Word => DT_WORD_ELLIPSIS,
            Truncate::Path => DT_PATH_ELLIPSIS
        } |
        match self.alignment {
            TextAlignment::Left => DT_LEFT,
            TextAlignment::Center => DT_CENTER,
            TextAlignment::Right => DT_RIGHT
        }
    }
}

impl Default for TextDrawOptions {
    fn default() -> TextDrawOptions {
        TextDrawOptions {
            expand_tabs: true,
            clip: true,
            proc_prefix: true,
            truncate: Truncate::default(),
            alignment: TextAlignment::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Truncate {
    None,
    End,
    Word,
    Path
}

#[derive(Debug, Clone, Copy)]
pub enum TextAlignment {
    Left,
    Center,
    Right
}

impl Default for Truncate {
    fn default() -> Truncate {
        Truncate::None
    }
}

impl Default for TextAlignment {
    fn default() -> TextAlignment {
        TextAlignment::Left
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontMetrics {
    pub height: Px,
    pub ascent: Px,
    pub descent: Px,
    pub internal_leading: Px,
    pub external_leading: Px,
    pub ave_char_width: Px,
    pub max_char_width: Px,
    pub weight: i32,
    pub overhang: Px,
    pub digitized_aspect_x: i32,
    pub digitized_aspect_y: i32,
    pub first_char: char,
    pub last_char: char,
    pub default_char: char,
    pub break_char: char,
    pub italic: bool,
    pub underlined: bool,
    pub struck_out: bool,
    pub fixed_pitch: bool,
    pub vector: bool,
    pub truetype: bool,
    pub device: bool,
    pub char_set: CharSet
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharSet {
    Ansi,
    Baltic,
    ChineseBig5,
    Default,
    EastEurope,
    Gb2312,
    Greek,
    Hangul,
    Mac,
    OEM,
    Russian,
    ShiftJIS,
    Symbol,
    Turkish,
    Vietnamese,
    Johab,
    Arabic,
    Hebrew,
    Thai
}
