use dct::geometry::{Px, OriginRect, OffsetRect};

use winapi::*;
use user32;

use std::{mem, ptr};

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

impl DeviceContext for PaintContext {
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

impl DeviceContext for RetrievedContext {
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

pub trait DeviceContext {
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
