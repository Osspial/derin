use dct::geometry::Px;

use winapi::*;
use gdi32;

use std::{mem, ptr, char};
use std::borrow::Borrow;


pub struct Font( HFONT );

impl Font {
    pub fn def_sys_font() -> Font {
        Font(ptr::null_mut())
    }

    pub fn sys_caption_font() -> Font {
        let non_client_metrics = ::non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfCaptionFont) })
    }

    pub fn sys_small_caption_font() -> Font {
        let non_client_metrics = ::non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfSmCaptionFont) })
    }

    pub fn sys_menu_font() -> Font {
        let non_client_metrics = ::non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfMenuFont) })
    }

    pub fn sys_status_font() -> Font {
        let non_client_metrics = ::non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfStatusFont) })
    }

    pub fn sys_message_font() -> Font {
        let non_client_metrics = ::non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfMessageFont) })
    }

    #[inline]
    pub fn hfont(&self) -> HFONT {
        self.0
    }
}

unsafe impl Send for Font {}
unsafe impl Sync for Font {}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe{ gdi32::DeleteObject(self.0 as HGDIOBJ) };
    }
}

pub struct DefaultFont;
impl Borrow<Font> for DefaultFont {
    fn borrow(&self) -> &Font {
        static DEFAULT_FONT: usize = 0;
        unsafe{ mem::transmute(&DEFAULT_FONT) }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextFormat {
    pub expand_tabs: bool,
    pub clip: bool,
    pub proc_prefix: bool,
    pub truncate: Truncate,
    pub h_align: TextAlign,
    /// The vertical alignment of the text. If this is `Some`, the text will only be drawn on a
    /// single line (ignoring line breaks)
    pub v_align: Option<TextAlign>
}

impl TextFormat {
    pub fn into_text_format(self) -> UINT {
        DT_EXPANDTABS * self.expand_tabs as UINT |
        DT_NOCLIP * !self.clip as UINT |
        DT_NOPREFIX * !self.proc_prefix as UINT |
        match self.truncate {
            Truncate::None => 0,
            Truncate::End  => DT_END_ELLIPSIS,
            Truncate::Word => DT_WORD_ELLIPSIS,
            Truncate::Path => DT_PATH_ELLIPSIS
        } |
        match self.h_align {
            TextAlign::Start => DT_LEFT,
            TextAlign::Center => DT_CENTER,
            TextAlign::End => DT_RIGHT
        } |
        match self.v_align {
            Some(TextAlign::Start) => DT_TOP | DT_SINGLELINE,
            Some(TextAlign::Center) => DT_VCENTER | DT_SINGLELINE,
            Some(TextAlign::End) => DT_BOTTOM | DT_SINGLELINE,
            None => 0
        }
    }
}

impl Default for TextFormat {
    fn default() -> TextFormat {
        TextFormat {
            expand_tabs: true,
            clip: true,
            proc_prefix: true,
            truncate: Truncate::default(),
            h_align: TextAlign::default(),
            v_align: None
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
pub enum TextAlign {
    Start,
    Center,
    End
}

impl Default for Truncate {
    fn default() -> Truncate {
        Truncate::None
    }
}

impl Default for TextAlign {
    fn default() -> TextAlign {
        TextAlign::Start
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
