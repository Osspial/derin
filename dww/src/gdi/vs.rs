use winapi::*;
use uxtheme;
use gdi32;

use window::{BaseWindow, WindowBuilder, BlankBase};
use std::{ptr, mem};
use std::marker::{Send, Sync};
use gdi::img::BitmapRef;
use gdi::text::Font;
use dct::color::Color24;
use dct::hints::Margins;
use dct::geometry::{Point, OffsetRect};

struct ThemeWindow(BlankBase);
unsafe impl Send for ThemeWindow {}
unsafe impl Sync for ThemeWindow {}

lazy_static!{
    static ref THEME_HWND: ThemeWindow = ThemeWindow(WindowBuilder::default().show_window(false).build_blank());
}

pub unsafe trait ThemeClass<P: Part> {
    fn htheme(&self) -> HTHEME;

    #[inline]
    fn get_theme_bitmap(&self, part: P) -> Option<BitmapRef> {
        let mut bitmap_handle = ptr::null_mut();
        unsafe{ uxtheme::GetThemeBitmap(
            self.htheme(),
            part.part_id(),
            part.state_id(),
            0,
            1,
            &mut bitmap_handle
        ) };

        if bitmap_handle != ptr::null_mut() {
            unsafe{ Some(BitmapRef::from_raw(bitmap_handle)) }
        } else {
            None
        }
    }

    #[inline]
    fn get_theme_bool(&self, part: P, prop: BoolProp) -> Option<bool> {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            let mut theme_bool = FALSE;
            let result = uxtheme::GetThemeBool(
                self.htheme(),
                part.part_id(),
                part.state_id(),
                prop_int,
                &mut theme_bool
            );

            if result == S_OK {
                Some(theme_bool == TRUE)
            } else {
                None
            }
        }
    }

    #[inline]
    fn get_theme_sys_bool(&self, prop: SysBoolProp) -> bool {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            TRUE == uxtheme::GetThemeSysBool(
                self.htheme(),
                prop_int
            )
        }
    }

    #[inline]
    fn get_theme_color(&self, part: P, prop: ColorProp) -> Option<Color24> {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            let mut theme_color = 0;
            let result = uxtheme::GetThemeColor(
                self.htheme(),
                part.part_id(),
                part.state_id(),
                prop_int,
                &mut theme_color
            );

            if result == S_OK {
                Some(Color24 {
                    red: (theme_color & 0xFF) as u8,
                    green: ((theme_color >> 2) & 0xFF) as u8,
                    blue: ((theme_color >> 4) & 0xFF) as u8
                })
            } else {
                None
            }
        }
    }

    #[inline]
    fn get_theme_font(&self, part: P, prop: FontProp) -> Option<Font> {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            let mut log_font = mem::uninitialized();
            let result = uxtheme::GetThemeFont(
                self.htheme(),
                ptr::null_mut(),
                part.part_id(),
                part.state_id(),
                prop_int,
                &mut log_font
            );

            if result == S_OK {
                Some(Font::from_raw(gdi32::CreateFontIndirectW(&log_font)))
            } else {
                None
            }
        }
    }

    #[inline]
    fn get_theme_sys_font(&self, prop: SysFontProp) -> Option<Font> {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            let mut log_font = mem::uninitialized();
            let result = uxtheme::GetThemeSysFont(
                self.htheme(),
                prop_int,
                &mut log_font
            );

            if result == S_OK {
                Some(Font::from_raw(gdi32::CreateFontIndirectW(&log_font)))
            } else {
                None
            }
        }
    }

    #[inline]
    fn get_theme_int(&self, part: P, prop: IntProp) -> Option<i32> {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            let mut int = 0;
            let result = uxtheme::GetThemeInt(
                self.htheme(),
                part.part_id(),
                part.state_id(),
                prop_int,
                &mut int
            );

            if result == S_OK {
                Some(int)
            } else {
                None
            }
        }
    }

    #[inline]
    fn get_theme_margins(&self, part: P, prop: MarginsProp) -> Option<Margins> {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            let mut margins = mem::uninitialized();
            let result = uxtheme::GetThemeMargins(
                self.htheme(),
                ptr::null_mut(),
                part.part_id(),
                part.state_id(),
                prop_int,
                ptr::null_mut(),
                &mut margins
            );

            if result == S_OK {
                Some(Margins {
                    left: margins.cxLeftWidth,
                    top: margins.cyTopHeight,
                    right: margins.cxRightWidth,
                    bottom: margins.cyBottomHeight
                })
            } else {
                None
            }
        }
    }

    #[inline]
    fn get_theme_position(&self, part: P, prop: PositionProp) -> Option<Point> {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            let mut point = mem::uninitialized();
            let result = uxtheme::GetThemePosition(
                self.htheme(),
                part.part_id(),
                part.state_id(),
                prop_int,
                &mut point
            );

            if result == S_OK {
                Some(Point::new(point.x, point.y))
            } else {
                None
            }
        }
    }

    #[inline]
    fn get_theme_rect(&self, part: P, prop: RectProp) -> Option<OffsetRect> {
        unsafe {
            let prop_int: c_int = mem::transmute(prop);
            let mut rect = mem::uninitialized();
            let result = uxtheme::GetThemeRect(
                self.htheme(),
                part.part_id(),
                part.state_id(),
                prop_int,
                &mut rect
            );

            if result == S_OK {
                Some(OffsetRect::new(rect.left, rect.top, rect.right, rect.bottom))
            } else {
                None
            }
        }
    }
}

pub unsafe trait Part: Copy {
    fn part_id(self) -> c_int;
    fn state_id(self) -> c_int;
}

macro_rules! theme_class {
    (
        $(pub class $class_name:ident
                where mod parts = $parts_mod_name:ident
        {
            $(part $part_name:ident = $part_id:tt $({
                $($state:ident = $state_id:tt),+
            })*),+
        })+
    ) => {$(
        pub struct $class_name( HTHEME );

        pub mod $parts_mod_name {$(
            if_tokens!{($($($state)+)*) {
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
                #[repr(u8)]
                pub enum $part_name {$(
                    $($state = $state_id),+
                )*}

                unsafe impl super::Part for $part_name {
                    #[inline]
                    fn part_id(self) -> ::winapi::c_int {
                        $part_id
                    }
                    #[inline]
                    fn state_id(self) -> ::winapi::c_int {
                        use std::mem;
                        unsafe{ mem::transmute::<_, u8>(self) as ::winapi::c_int }
                    }
                }
            } else {
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
                pub struct $part_name;

                unsafe impl super::Part for $part_name {
                    #[inline]
                    fn part_id(self) -> ::winapi::c_int {
                        $part_id
                    }
                    #[inline]
                    fn state_id(self) -> ::winapi::c_int {
                        0
                    }
                }
            }}
        )+}

        impl $class_name {
            pub fn new() -> Option<$class_name> {
                use ucs2::{self, Ucs2String};

                lazy_static!{
                    static ref CLASS_NAME_UCS2: Ucs2String = ucs2::ucs2_str(stringify!($class_name)).collect();
                }

                let theme_handle = unsafe{ uxtheme::OpenThemeData(THEME_HWND.0.hwnd(), CLASS_NAME_UCS2.as_ptr()) };
                if theme_handle != ptr::null_mut() {
                    Some($class_name(theme_handle))
                } else {
                    None
                }
            }


            #[allow(non_upper_case_globals)]
            pub fn new_subclass(subclass_name: &str) -> Option<$class_name> {
                use ucs2::{self, ucs2_str, WithString};

                let class_name = stringify!($class_name);
                // Include space for the class name, subclass name, and "::"
                let full_class_name_len = class_name.len() + subclass_name.len() + 2;

                ucs2::UCS2_CONVERTER.with_ucs2_buffer(full_class_name_len, |buf| {
                    let full_class_name_iter = ucs2_str(subclass_name).chain(ucs2_str("::").chain(ucs2_str(class_name)));
                    for (buf_entry, ucs2_char) in buf.iter_mut().zip(full_class_name_iter) {
                        *buf_entry = ucs2_char;
                    }

                    let theme_handle = unsafe{ uxtheme::OpenThemeData(THEME_HWND.0.hwnd(), buf.as_ptr()) };
                    if theme_handle != ptr::null_mut() {
                        Some($class_name(theme_handle))
                    } else {
                        None
                    }
                })
            }
        }

        $(
            unsafe impl ThemeClass<self::$parts_mod_name::$part_name> for $class_name {
                #[inline]
                fn htheme(&self) -> HTHEME {
                    self.0
                }
            }
        )+

        impl Drop for $class_name {
            fn drop(&mut self) {
                unsafe{ uxtheme::CloseThemeData(self.0) };
            }
        }
    )+}
}

theme_class!{
    // Where do all these numbers come from? Well, from the winapi Headers. I would've just used the
    // constants directly but then rust complained with the error "unimplemented constant
    // expression: tuple struct constructors" which hey I wasn't happy about but I needed some sort of
    // workaround.
    //
    // Also some classes are missing (GLOBALS and SEARCHEDITBOX), as well as a state (TBP_SIZINGBARBOTTOMLEFT)
    // because the Headers I have don't have those constants for whatever reason and I wasn't able to find them.
    pub class Button
            where mod parts = button_parts
    {
        part BpPushButton = 1 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Defaulted = 5
        },
        part BpRadioButton = 2 {
            UncheckedNormal = 1,
            UncheckedHot = 2,
            UncheckedPressed = 3,
            UncheckedDisabled = 4,
            CheckedNormal = 5,
            CheckedHot = 6,
            CheckedPressed = 7,
            CheckedDisabled = 8
        },
        part BpCheckbox = 3 {
            UncheckedNormal = 1,
            UncheckedHot = 2,
            UncheckedPressed = 3,
            UncheckedDisabled = 4,
            CheckedNormal = 5,
            CheckedHot = 6,
            CheckedPressed = 7,
            CheckedDisabled = 8,
            MixedNormal = 9,
            MixedHot = 10,
            MixedPressed = 11,
            MixedDisabled = 12
        },
        part BpGroupBox = 4 {
            Normal = 1,
            Disabled = 2
        },
        part BpUserButton = 5,
        part BpCommandLink = 6 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Defaulted = 5,
            DefaultedAnimating = 6
        },
        part BpCommandLinkGlyph = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Defaulted = 5
        }
    }
    pub class Clock
            where mod parts = clock_parts
    {
        part ClpTime = 1 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        }
    }
    pub class ComboBox
            where mod parts = combo_box_parts
    {
        part CpDropdownButton = 1 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part CpBackground = 2,
        part CpTransparentBackground = 3 {
            Normal = 1,
            Hot = 2,
            Disabled = 3,
            Focused = 4
        },
        part CpBorder = 4 {
            Normal = 1,
            Hot = 2,
            Focused = 3,
            Disabled = 4
        },
        part CpReadOnly = 5 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part CpDropdownButtonRight = 6 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part CpDropdownButtonLeft = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part CpCueBanner = 8 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        }
    }
    pub class Communications
            where mod parts = communications_parts
    {
        part CsstTab = 1 {
            Normal = 1,
            Hot = 2,
            Selected = 3
        }
    }
    pub class ControlPanel
            where mod parts = control_panel_parts
    {
        part CPanelNavigationPane = 1,
        part CPanelContentPane = 2,
        part CPanelNavigationPaneLabel = 3,
        part CPanelContentPaneLabel = 4,
        part CPanelTitle = 5,
        part CPanelBodyText = 6,
        part CPanelHelpLink = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part CPanelTaskLink = 8 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Page = 5
        },
        part CPanelGroupText = 9,
        part CPanelContentLink = 10 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part CPanelSectionTitleLink = 11 {
            Normal = 1,
            Hot = 2
        },
        part CPanelLargeCommandArea = 12,
        part CPanelSmallCommandArea = 13,
        part CPanelButton = 14,
        part CPanelMessageText = 15,
        part CPanelNavigationPaneLine = 16,
        part CPanelContentPaneLine = 17,
        part CPanelBannerArea = 18,
        part CPanelBodyTitle = 19
    }
    pub class DatePicker
            where mod parts = date_picker_parts
    {
        part DpDateText = 1 {
            Normal = 1,
            Disabled = 2,
            Selected = 3
        },
        part DpDateBorder = 2 {
            Normal = 1,
            Hot = 2,
            Focused = 3,
            Disabled = 4
        },
        part DpShowCalendarButtonRight = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        }
    }
    pub class DragDrop
            where mod parts = drag_drop_parts
    {
        part DdCopy = 1 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdMove = 2 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdUpdateMetadata = 3 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdCreateLink = 4 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdWarning = 5 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdNone = 6 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdImageBG = 7,
        part DdTextBG = 8
    }
    pub class Edit
            where mod parts = edit_parts
    {
        part EpEditText = 1 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5,
            ReadOnly = 6,
            Assist = 7,
            CueBanner = 8
        },
        part EpCaret = 2,
        part EpBackground = 3 {
            Normal = 1,
            Hot = 2,
            Disabled = 3,
            Focused = 4,
            ReadOnly = 5,
            Assist = 6
        },
        part EpPassword = 4,
        part EpBackgroundWithBorder = 5 {
            Normal = 1,
            Hot = 2,
            Disabled = 3,
            Focused = 4
        },
        part EpEditBorderNoScroll = 6 {
            Normal = 1,
            Hot = 2,
            Focused = 3,
            Disabled = 4
        },
        part EpEditBorderHScroll = 7 {
            Normal = 1,
            Hot = 2,
            Focused = 3,
            Disabled = 4
        },
        part EpEditBorderVScroll = 8 {
            Normal = 1,
            Hot = 2,
            Focused = 3,
            Disabled = 4
        },
        part EpEditBorderHVScroll = 9 {
            Normal = 1,
            Hot = 2,
            Focused = 3,
            Disabled = 4
        }
    }
    pub class ExplorerBar
            where mod parts = explorer_bar_parts
    {
        part EbpHeaderBackground = 1,
        part EbpHeaderClose = 2 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part EbpHeaderPin = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            SelectedNormal = 4,
            SelectedHot = 5,
            SelectedPressed = 6
        },
        part EbpIeBarMenu = 4 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part EbpNormalGroupBackground = 5,
        part EbpNormalGroupCollapse = 6 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part EbpNormalGroupExpand = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part EbpNormalGroupHead = 8,
        part EbpSpecialGroupBackground = 9,
        part EbpSpecialGroupCollapse = 10 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part EbpSpecialGroupExpand = 11 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part EbpSpecialGroupHead = 12
    }
    pub class Flyout
            where mod parts = flyout_parts
    {
        part FlyoutHeader = 1,
        part FlyoutBody = 2 {
            Normal = 1,
            Emphasized = 2
        },
        part FlyoutLabel = 3 {
            Normal = 1,
            Selected = 2,
            Emphasized = 3,
            Disabled = 4
        },
        part FlyoutLink = 4 {
            Normal = 1,
            Hover = 2
        },
        part FlyoutDivider = 5,
        part FlyoutWindow = 6,
        part FlyoutLinkArea = 7,
        part FlyoutLinkHeader = 8 {
            Normal = 1,
            Hover = 2
        }
    }
    pub class Header
            where mod parts = header_parts
    {
        part HpHeaderItem = 1 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            SortedNormal = 4,
            SortedHot = 5,
            SortedPressed = 6,
            IconNormal = 7,
            IconHot = 8,
            IconPressed = 9,
            IconSortedNormal = 10,
            IconSortedHot = 11,
            IconSortedPressed = 12
        },
        part HpHeaderItemLeft = 2 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part HpHeaderItemRight = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part HpHeaderSortArrow = 4 {
            SortedUp = 1,
            SortedDown = 2
        },
        part HpHeaderDropdown = 5 {
            Normal = 1,
            SoftHot = 2,
            Hot = 3
        },
        part HpHeaderDropdownFilter = 6 {
            Normal = 1,
            SoftHot = 2,
            Hot = 3
        },
        part HpHeaderOverflow = 7 {
            Normal = 1,
            Hot = 2
        }
    }
    pub class ListBox
            where mod parts = list_box_parts
    {
        part LbcpBorderHScroll = 1 {
            Normal = 1,
            Focused = 2,
            Hot = 3,
            Disabled = 4
        },
        part LbcpBorderHVScroll = 2 {
            Normal = 1,
            Focused = 2,
            Hot = 3,
            Disabled = 4
        },
        part LbcpBorderNoScroll = 3 {
            Normal = 1,
            Focused = 2,
            Hot = 3,
            Disabled = 4
        },
        part LbcpBorderVScroll = 4 {
            Normal = 1,
            Focused = 2,
            Hot = 3,
            Disabled = 4
        },
        part LbcpItem = 5 {
            Hot = 1,
            HotSelected = 2,
            Selected = 3,
            SelectedNotFocus = 4
        }
    }
    pub class ListView
            where mod parts = list_view_parts
    {
        part LvpListItem = 1 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            SelectedNotFocus = 5,
            HotSelected = 6
        },
        part LvpListGroup = 2,
        part LvpListDetail = 3,
        part LvpListSortedDetail = 4,
        part LvpEmptyText = 5,
        part LvpGroupHeader = 6 {
            Open = 1,
            OpenHot = 2,
            OpenSelected = 3,
            OpenSelectedHot = 4,
            OpenSelectedNotFocused = 5,
            OpenSelectedNotFocusedHot = 6,
            OpenMixedSelection = 7,
            OpenMixedSelectionHot = 8,
            Close = 9,
            CloseHot = 10,
            CloseSelected = 11,
            CloseSelectedHot = 12,
            CloseSelectedNotFocused = 13,
            CloseSelectedNotFocusedHot = 14,
            CloseMixedSelection = 15,
            CloseMixedSelectionHot = 16
        },
        part LvpGroupHeaderLine = 7 {
            Open = 1,
            OpenHot = 2,
            OpenSelected = 3,
            OpenSelectedHot = 4,
            OpenSelectedNotFocused = 5,
            OpenSelectedNotFocusedHot = 6,
            OpenMixedSelection = 7,
            OpenMixedSelectionHot = 8,
            Close = 9,
            CloseHot = 10,
            CloseSelected = 11,
            CloseSelectedHot = 12,
            CloseSelectedNotFocused = 13,
            CloseSelectedNotFocusedHot = 14,
            CloseMixedSelection = 15,
            CloseMixedSelectionHot = 16
        },
        part LvpExpandButton = 8 {
            Normal = 1,
            Hover = 2,
            Pushed = 3
        },
        part LvpCollapseButton = 9 {
            Normal = 1,
            Hover = 2,
            Pushed = 3
        },
        part LvpColumnDetail = 10
    }
    pub class Menu
            where mod parts = menu_parts
    {
        part MenuMenuItemTmSchema = 1,
        part MenuMenuDropdownTmSchema = 2,
        part MenuMenuBarItemTmSchema = 3,
        part MenuMenuBarDropdownTmSchema = 4,
        part MenuChevronTmSchema = 5,
        part MenuSeparatorTmSchema = 6,
        part MenuBarBackground = 7 {
            Active = 1,
            Inactive = 2
        },
        part MenuBarItem = 8 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4,
            DisabledHot = 5,
            DisabledPushed = 6
        },
        part MenuPopupBackground = 9,
        part MenuPopupBorders = 10,
        part MenuPopupCheck = 11 {
            CheckmarkNormal = 1,
            CheckmarkDisabled = 2,
            BulletNormal = 3,
            BulletDisabled = 4
        },
        part MenuPopupCheckBackground = 12 {
            Disabled = 1,
            Normal = 2,
            Bitmap = 3
        },
        part MenuPopupGutter = 13,
        part MenuPopupItem = 14 {
            Normal = 1,
            Hot = 2,
            Disabled = 3,
            DisabledHot = 4
        },
        part MenuPopupSeparator = 15,
        part MenuPopupSubmenu = 16 {
            Normal = 1,
            Disabled = 2
        },
        part MenuSystemClose = 17 {
            Normal = 1,
            Disabled = 2
        },
        part MenuSystemMaximize = 18 {
            Normal = 1,
            Disabled = 2
        },
        part MenuSystemMinimize = 19 {
            Normal = 1,
            Disabled = 2
        },
        part MenuSystemRestore = 20 {
            Normal = 1,
            Disabled = 2
        }
    }
    pub class MenuBand
            where mod parts = menu_band_parts
    {
        part MdpNewAppButton = 1 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Checked = 5,
            HotChecked = 6
        },
        part MdpSeperator = 2
    }
    pub class Navigation
            where mod parts = navigation_parts
    {
        part NavBackButton = 1 {
            BbNormal = 1,
            BbHot = 2,
            BbPressed = 3,
            BbDisabled = 4
        },
        part NavForwardButton = 2 {
            FbNormal = 1,
            FbHot = 2,
            FbPressed = 3,
            FbDisabled = 4
        },
        part NavMenuButton = 3 {
            MbNormal = 1,
            MbHot = 2,
            MbPressed = 3,
            MbDisabled = 4
        }
    }
    pub class Page
            where mod parts = page_parts
    {
        part PgrpUp = 1 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part PgrpDown = 2 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part PgrpUpHorz = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part PgrpDownHorz = 4 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        }
    }
    pub class Progress
            where mod parts = progress_parts
    {
        part PpBar = 1,
        part PpBarVert = 2,
        part PpChunk = 3,
        part PpChunkVert = 4,
        part PpFill = 5 {
            Normal = 1,
            Error = 2,
            Paused = 3,
            Partial = 4
        },
        part PpFillVert = 6 {
            Normal = 1,
            Error = 2,
            Paused = 3,
            Partial = 4
        },
        part PpPulseOverlay = 7,
        part PpMoveOverlay = 8,
        part PpPulseOverlayVert = 9,
        part PpMoveOverlayVert = 10,
        part PpTransparentBar = 11 {
            Normal = 1,
            Partial = 2
        },
        part PpTransparentBarVert = 12 {
            Normal = 1,
            Partial = 2
        }
    }
    pub class Rebar
            where mod parts = rebar_parts
    {
        part RpGripper = 1,
        part RpGripperVert = 2,
        part RpBand = 3,
        part RpChevron = 4 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part RpChevronVert = 5 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part RpBackground = 6,
        part RpSplitter = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part RpSplitterVert = 8 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        }
    }
    pub class ScrollBar
            where mod parts = scroll_bar_parts
    {
        part SbpArrowBtn = 1 {
            UpNormal = 1,
            UpHot = 2,
            UpPressed = 3,
            UpDisabled = 4,
            DownNormal = 5,
            DownHot = 6,
            DownPressed = 7,
            DownDisabled = 8,
            LeftNormal = 9,
            LeftHot = 10,
            LeftPressed = 11,
            LeftDisabled = 12,
            RightNormal = 13,
            RightHot = 14,
            RightPressed = 15,
            RightDisabled = 16,
            UpHover = 17,
            DownHover = 18,
            LeftHover = 19,
            RightHover = 20
        },
        part SbpThumbBtnHorz = 2 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Hover = 5
        },
        part SbpThumbBtnVert = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Hover = 5
        },
        part SbpLowerTrackHorz = 4 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Hover = 5
        },
        part SbpUpperTrackHorz = 5 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Hover = 5
        },
        part SbpLowerTrackVert = 6 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Hover = 5
        },
        part SbpUpperTrackVert = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Hover = 5
        },
        part SbpGripperHorz = 8 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Hover = 5
        },
        part SbpGripperVert = 9 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Hover = 5
        },
        part SbpSizeBox = 10 {
            RightAlign = 1,
            LeftAlign = 2,
            TopRightAlign = 3,
            TopLeftAlign = 4,
            HalfBottomRightAlign = 5,
            HalfBottomLeftAlign = 6,
            HalfTopRightAlign = 7,
            HalfTopLeftAlign = 8
        }
    }
    pub class Spin
            where mod parts = spin_parts
    {
        part SpnpUp = 1 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part SpnpDown = 2 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part SpnpUpHorz = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        },
        part SpnpDownHorz = 4 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4
        }
    }
    pub class StartPanel
            where mod parts = start_panel_parts
    {
        part SppUserPane = 1,
        part SppMorePrograms = 2,
        part SppMoreProgramsArrow = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part SppProgList = 4,
        part SppProgListSeparator = 5,
        part SppPlacesList = 6,
        part SppPlacesListSeparator = 7,
        part SppLogoff = 8,
        part SppLogoffButtons = 9 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part SppUserpicture = 10,
        part SppPreView = 11
    }
    pub class Status
            where mod parts = status_parts
    {
        part SpPane = 1,
        part SpGripperPane = 2,
        part SpGripper = 3
    }
    pub class Tab
            where mod parts = tab_parts
    {
        part TabpTabItem = 1 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5
        },
        part TabpTabItemLeftEdge = 2 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5
        },
        part TabpTabItemRightEdge = 3 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5
        },
        part TabpTabItemBothEdge = 4 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5
        },
        part TabpTopTabItem = 5 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5
        },
        part TabpTopTabItemLeftEdge = 6 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5
        },
        part TabpTopTabItemRightEdge = 7 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5
        },
        part TabpTopTabItemBothEdge = 8 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            Focused = 5
        },
        part TabpPane = 9,
        part TabpBody = 10,
        part TabpAeroWizardBody = 11
    }
    pub class TaskBand
            where mod parts = task_band_parts
    {
        part TdpGroupcount = 1,
        part TdpFlashButton = 2,
        part TdpFlashButtonGroupMenu = 3
    }
    pub class TaskBar
            where mod parts = task_bar_parts
    {
        part TbpBackgroundBottom = 1,
        part TbpBackgroundRight = 2,
        part TbpBackgroundTop = 3,
        part TbpBackgroundLeft = 4,
        part TbpSizingBarBottom = 5,
        part TbpSizingBarRight = 6,
        part TbpSizingBarTop = 7
    }
    pub class TaskDialog
            where mod parts = task_dialog_parts
    {
        part TdlgPrimaryPanel = 1,
        part TdlgMainInstructionPane = 2,
        part TdlgMainIcon = 3,
        part TdlgContentPane = 4 {
            Standalone = 1
        },
        part TdlgContentIcon = 5,
        part TdlgExpandedcontent = 6,
        part TdlgCommandLinkPane = 7,
        part TdlgSecondaryPanel = 8,
        part TdlgControlPane = 9,
        part TdlgButtonSection = 10,
        part TdlgButtonWrapper = 11,
        part TdlgExpandoText = 12,
        part TdlgExpandoButton = 13 {
            Normal = 1,
            Hover = 2,
            Pressed = 3,
            ExpandedNormal = 4,
            ExpandedHover = 5,
            ExpandedPressed = 6
        },
        part TdlgVerificationText = 14,
        part TdlgFootNotePane = 15,
        part TdlgFootNoteArea = 16,
        part TdlgFootNoteSeparator = 17,
        part TdlgExpandedfooterArea = 18,
        part TdlgProgressBar = 19,
        part TdlgImageAlignment = 20,
        part TdlgRadioButtonPane = 21
    }
    pub class TextStyle
            where mod parts = text_style_parts
    {
        part TextMainInstruction = 1,
        part TextInstruction = 2,
        part TextBodyTitle = 3,
        part TextBodyText = 4,
        part TextSecondaryText = 5,
        part TextHyperLinkText = 6 {
            HyperLinkNormal = 1,
            HyperLinkHot = 2,
            HyperLinkPressed = 3,
            HyperLinkDisabled = 4
        },
        part TextExpanded = 7,
        part TextLabel = 8,
        part TextControlLabel = 9 {
            ControlLabelNormal = 1,
            ControlLabelDisabled = 2
        }
    }
    pub class ToolBar
            where mod parts = tool_bar_parts
    {
        part TpButton = 1 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Checked = 5,
            HotChecked = 6,
            NearHot = 7,
            OtherSideHot = 8
        },
        part TpDropdownButton = 2 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Checked = 5,
            HotChecked = 6,
            NearHot = 7,
            OtherSideHot = 8
        },
        part TpSplitButton = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Checked = 5,
            HotChecked = 6,
            NearHot = 7,
            OtherSideHot = 8
        },
        part TpSplitButtonDropdown = 4 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Checked = 5,
            HotChecked = 6,
            NearHot = 7,
            OtherSideHot = 8
        },
        part TpSeparator = 5 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Checked = 5,
            HotChecked = 6,
            NearHot = 7,
            OtherSideHot = 8
        },
        part TpSeparatorVert = 6 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Checked = 5,
            HotChecked = 6,
            NearHot = 7,
            OtherSideHot = 8
        },
        part TpDropdownButtonGlyph = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Disabled = 4,
            Checked = 5,
            HotChecked = 6,
            NearHot = 7,
            OtherSideHot = 8
        }
    }
    pub class Tooltip
            where mod parts = tooltip_parts
    {
        part TtpStandard = 1 {
            Normal = 1,
            Link = 2
        },
        part TtpStandardTitle = 2 {
            Normal = 1,
            Link = 2
        },
        part TtpBalloon = 3 {
            Normal = 1,
            Link = 2
        },
        part TtpBalloonTitle = 4,
        part TtpClose = 5 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        },
        part TtpBalloonStem = 6 {
            PointingUpLeftWall = 1,
            PointingUpCentered = 2,
            PointingUpRightWall = 3,
            PointingDownRightWall = 4,
            PointingDownCentered = 5,
            PointingDownLeftWall = 6
        },
        part TtpWrench = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3
        }
    }
    pub class TrackBar
            where mod parts = track_bar_parts
    {
        part TkpTrack = 1 {
            Normal = 1
        },
        part TkpTrackVert = 2 {
            Normal = 1
        },
        part TkpThumb = 3 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Focused = 4,
            Disabled = 5
        },
        part TkpThumbBottom = 4 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Focused = 4,
            Disabled = 5
        },
        part TkpThumbTop = 5 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Focused = 4,
            Disabled = 5
        },
        part TkpThumbVert = 6 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Focused = 4,
            Disabled = 5
        },
        part TkpThumbLeft = 7 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Focused = 4,
            Disabled = 5
        },
        part TkpThumbRight = 8 {
            Normal = 1,
            Hot = 2,
            Pressed = 3,
            Focused = 4,
            Disabled = 5
        },
        part TkpTics = 9 {
            Normal = 1
        },
        part TkpTicsVert = 10 {
            Normal = 1
        }
    }
    pub class TrayNotify
            where mod parts = tray_botify_parts
    {
        part TnpBackground = 1,
        part TnpAnimBackground = 2
    }
    pub class TreeView
            where mod parts = tree_view_parts
    {
        part TvpTreeItem = 1 {
            Normal = 1,
            Hot = 2,
            Selected = 3,
            Disabled = 4,
            SelectedNotFocus = 5,
            HotSelected = 6
        },
        part TvpGlyph = 2 {
            Closed = 1,
            Opened = 2
        },
        part TvpBranch = 3,
        part TvpHotGlyph = 4 {
            Closed = 1,
            Opened = 2
        }
    }
    pub class Window
            where mod parts = window_parts
    {
        part WpCaption = 1 {
            Active = 1,
            Inactive = 2,
            Disabled = 3
        },
        part WpSmallCaption = 2,
        part WpMinCaption = 3 {
            Active = 1,
            Inactive = 2,
            Disabled = 3
        },
        part WpSmallMinCaption = 4,
        part WpMaxCaption = 5 {
            Active = 1,
            Inactive = 2,
            Disabled = 3
        },
        part WpSmallMaxCaption = 6,
        part WpFrameLeft = 7,
        part WpFrameRight = 8,
        part WpFrameBottom = 9,
        part WpSmallFrameLeft = 10,
        part WpSmallFrameRight = 11,
        part WpSmallFrameBottom = 12,
        part WpSysButton = 13 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpMdiSysButton = 14,
        part WpMinButton = 15 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpMdiMinButton = 16,
        part WpMaxButton = 17 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpCloseButton = 18 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpSmallCloseButton = 19,
        part WpMdiCloseButton = 20,
        part WpRestoreButton = 21 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpMdiRestoreButton = 22,
        part WpHelpButton = 23 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpMdiHelpButton = 24,
        part WpHorzScroll = 25 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpHorzThumb = 26 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpVertScroll = 27 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpVertThumb = 28 {
            Normal = 1,
            Hot = 2,
            Pushed = 3,
            Disabled = 4
        },
        part WpDialog = 29,
        part WpCaptionSizingTemplate = 30,
        part WpSmallCaptionSizingTemplate = 31,
        part WpFrameLeftSizingTemplate = 32,
        part WpSmallFrameLeftSizingTemplate = 33,
        part WpFrameRightSizingTemplate = 34,
        part WpSmallFrameRightSizingTemplate = 35,
        part WpFrameBottomSizingTemplate = 36,
        part WpSmallFrameBottomSizingTemplate = 37,
        part WpFrame = 38 {
            Active = 1,
            Inactive = 2
        }
    }
}

#[repr(i32)]
pub enum BoolProp {
    Transparent = TMT_TRANSPARENT,
    AutoSize = TMT_AUTOSIZE,
    BorderOnly = TMT_BORDERONLY,
    Composited = TMT_COMPOSITED,
    BGFill = TMT_BGFILL,
    GlyphTransparent = TMT_GLYPHTRANSPARENT,
    GlyphOnly = TMT_GLYPHONLY,
    AlwaysShowSizingBar = TMT_ALWAYSSHOWSIZINGBAR,
    MirrorImage = TMT_MIRRORIMAGE,
    UniformSizing = TMT_UNIFORMSIZING,
    IntegralSizing = TMT_INTEGRALSIZING,
    SourceGrow = TMT_SOURCEGROW,
    SourceShrink = TMT_SOURCESHRINK,
    UserPicture = TMT_USERPICTURE
}

#[repr(i32)]
pub enum SysBoolProp {
    FlatMenus = TMT_FLATMENUS
}

#[repr(i32)]
pub enum ColorProp {
    AccentColorHint = TMT_ACCENTCOLORHINT,
    ActiveBorder = TMT_ACTIVEBORDER,
    ActiveCaption = TMT_ACTIVECAPTION,
    AppWorkspace = TMT_APPWORKSPACE,
    Background = TMT_BACKGROUND,
    BlendColor = TMT_BLENDCOLOR,
    BodyTextColor = TMT_BODYTEXTCOLOR,
    BorderColor = TMT_BORDERCOLOR,
    BorderColorHint = TMT_BORDERCOLORHINT,
    BtnFace = TMT_BTNFACE,
    BtnHighlight = TMT_BTNHIGHLIGHT,
    BtnShadow = TMT_BTNSHADOW,
    BtnText = TMT_BTNTEXT,
    ButtonAlternateFace = TMT_BUTTONALTERNATEFACE,
    CaptionText = TMT_CAPTIONTEXT,
    DkShadow3d = TMT_DKSHADOW3D,
    EdgeDkShadowColor = TMT_EDGEDKSHADOWCOLOR,
    EdgeFillColor = TMT_EDGEFILLCOLOR,
    EdgeHighlightColor = TMT_EDGEHIGHLIGHTCOLOR,
    EdgeLightColor = TMT_EDGELIGHTCOLOR,
    EdgeShadowColor = TMT_EDGESHADOWCOLOR,
    FillColor = TMT_FILLCOLOR,
    FillColorHint = TMT_FILLCOLORHINT,
    FromColor1 = TMT_FROMCOLOR1,
    FromColor2 = TMT_FROMCOLOR2,
    FromColor3 = TMT_FROMCOLOR3,
    FromColor4 = TMT_FROMCOLOR4,
    FromColor5 = TMT_FROMCOLOR5,
    GlowColor = TMT_GLOWCOLOR,
    GlyphTextColor = TMT_GLYPHTEXTCOLOR,
    GlyphTransparentColor = TMT_GLYPHTRANSPARENTCOLOR,
    GradientActiveCaption = TMT_GRADIENTACTIVECAPTION,
    GradientColor1 = TMT_GRADIENTCOLOR1,
    GradientColor2 = TMT_GRADIENTCOLOR2,
    GradientColor3 = TMT_GRADIENTCOLOR3,
    GradientColor4 = TMT_GRADIENTCOLOR4,
    GradientColor5 = TMT_GRADIENTCOLOR5,
    GradientInactiveCaption = TMT_GRADIENTINACTIVECAPTION,
    GrayText = TMT_GRAYTEXT,
    Heading1TextColor = TMT_HEADING1TEXTCOLOR,
    Heading2TextColor = TMT_HEADING2TEXTCOLOR,
    Highlight = TMT_HIGHLIGHT,
    HighlightText = TMT_HIGHLIGHTTEXT,
    HotTracking = TMT_HOTTRACKING,
    InactiveBorder = TMT_INACTIVEBORDER,
    InactiveCaption = TMT_INACTIVECAPTION,
    InactiveCaptionText = TMT_INACTIVECAPTIONTEXT,
    InfoBk = TMT_INFOBK,
    InfoText = TMT_INFOTEXT,
    Light3d = TMT_LIGHT3D,
    Menu = TMT_MENU,
    MenuBar = TMT_MENUBAR,
    MenuHilight = TMT_MENUHILIGHT,
    MenuText = TMT_MENUTEXT,
    ScrollBar = TMT_SCROLLBAR,
    ShadowColor = TMT_SHADOWCOLOR,
    TextBorderColor = TMT_TEXTBORDERCOLOR,
    TextColor = TMT_TEXTCOLOR,
    TextColorHint = TMT_TEXTCOLORHINT,
    TextShadowColor = TMT_TEXTSHADOWCOLOR,
    TransparentColor = TMT_TRANSPARENTCOLOR,
    Window = TMT_WINDOW,
    WindowFrame = TMT_WINDOWFRAME,
    WindowText = TMT_WINDOWTEXT
}

// diskstream
// AtlasImage = TMT_ATLASIMAGE

// enum
// TMT_BGTYPE
// TMT_BORDERTYPE
// TMT_CONTENTALIGNMENT
// TMT_FILLTYPE
// TMT_GLYPHTYPE
// TMT_GLYPHFONTSIZINGTYPE
// TMT_HALIGN
// TMT_ICONEFFECT
// TMT_IMAGELAYOUT
// TMT_IMAGESELECTTYPE
// TMT_OFFSETTYPE
// TMT_SIZINGTYPE
// TMT_TEXTSHADOWTYPE
// TMT_TRUESIZESCALINGTYPE
// TMT_VALIGN

// #[repr(i32)]
// pub enum FileNameProp {
//     GlyphImageFile = TMT_GLYPHIMAGEFILE,
//     ImageFile = TMT_IMAGEFILE,
//     ImageFile1 = TMT_IMAGEFILE1,
//     ImageFile2 = TMT_IMAGEFILE2,
//     ImageFile3 = TMT_IMAGEFILE3,
//     ImageFile4 = TMT_IMAGEFILE4,
//     ImageFile5 = TMT_IMAGEFILE5
// }

#[repr(i32)]
pub enum FontProp {
    BodyFont = TMT_BODYFONT,
    CaptionFont = TMT_CAPTIONFONT,
    GlyphFont = TMT_GLYPHFONT,
    Heading1Font = TMT_HEADING1FONT,
    Heading2Font = TMT_HEADING2FONT,
    IconTitleFont = TMT_ICONTITLEFONT,
    MenuFont = TMT_MENUFONT,
    MsgBoxFont = TMT_MSGBOXFONT,
    SmallCaptionFont = TMT_SMALLCAPTIONFONT,
    StatusFont = TMT_STATUSFONT
}

#[repr(i32)]
pub enum SysFontProp {
    CaptionFont = TMT_CAPTIONFONT,
    SmallCaptionFont = TMT_SMALLCAPTIONFONT,
    MenuFont = TMT_MENUFONT,
    StatusFont = TMT_STATUSFONT,
    MsgBoxFont = TMT_MSGBOXFONT,
    IconTitleFont = TMT_ICONTITLEFONT
}

#[repr(i32)]
pub enum IntProp {
    AlphaLevel = TMT_ALPHALEVEL,
    AlphaThreshold = TMT_ALPHATHRESHOLD,
    AnimationDelay = TMT_ANIMATIONDELAY,
    AnimationDuration = TMT_ANIMATIONDURATION,
    BorderSize = TMT_BORDERSIZE,
    CharSet = TMT_CHARSET,
    ColorizationColor = TMT_COLORIZATIONCOLOR,
    ColorizationOpacity = TMT_COLORIZATIONOPACITY,
    FramesPerSecond = TMT_FRAMESPERSECOND,
    FromHue1 = TMT_FROMHUE1,
    FromHue2 = TMT_FROMHUE2,
    FromHue3 = TMT_FROMHUE3,
    FromHue4 = TMT_FROMHUE4,
    FromHue5 = TMT_FROMHUE5,
    GlowIntensity = TMT_GLOWINTENSITY,
    GlyphIndex = TMT_GLYPHINDEX,
    GradientRatio1 = TMT_GRADIENTRATIO1,
    GradientRatio2 = TMT_GRADIENTRATIO2,
    GradientRatio3 = TMT_GRADIENTRATIO3,
    GradientRatio4 = TMT_GRADIENTRATIO4,
    GradientRatio5 = TMT_GRADIENTRATIO5,
    Height = TMT_HEIGHT,
    ImageCount = TMT_IMAGECOUNT,
    MinColorDepth = TMT_MINCOLORDEPTH,
    MinDPI1 = TMT_MINDPI1,
    MinDPI2 = TMT_MINDPI2,
    MinDPI3 = TMT_MINDPI3,
    MinDPI4 = TMT_MINDPI4,
    MinDPI5 = TMT_MINDPI5,
    Opacity = TMT_OPACITY,
    PixelsPerFrame = TMT_PIXELSPERFRAME,
    ProgressChunkSize = TMT_PROGRESSCHUNKSIZE,
    ProgressSpaceSize = TMT_PROGRESSSPACESIZE,
    RoundCornerHeight = TMT_ROUNDCORNERHEIGHT,
    RoundCornerWidth = TMT_ROUNDCORNERWIDTH,
    Saturation = TMT_SATURATION,
    TextBorderSize = TMT_TEXTBORDERSIZE,
    TextGlowSize = TMT_TEXTGLOWSIZE,
    ToColor1 = TMT_TOCOLOR1,
    ToColor2 = TMT_TOCOLOR2,
    ToColor3 = TMT_TOCOLOR3,
    ToColor4 = TMT_TOCOLOR4,
    ToColor5 = TMT_TOCOLOR5,
    ToHue1 = TMT_TOHUE1,
    ToHue2 = TMT_TOHUE2,
    ToHue3 = TMT_TOHUE3,
    ToHue4 = TMT_TOHUE4,
    ToHue5 = TMT_TOHUE5,
    TrueSizeStretchMark = TMT_TRUESIZESTRETCHMARK,
    Width = TMT_WIDTH
}

// intlist
// TransitionDurations = TMT_TRANSITIONDURATIONS

#[repr(i32)]
pub enum MarginsProp {
    CaptionMargins = TMT_CAPTIONMARGINS,
    ContentMargins = TMT_CONTENTMARGINS,
    SizingMargins = TMT_SIZINGMARGINS
}

#[repr(i32)]
pub enum PositionProp {
    MinSize = TMT_MINSIZE,
    MinSize1 = TMT_MINSIZE1,
    MinSize2 = TMT_MINSIZE2,
    MinSize3 = TMT_MINSIZE3,
    MinSize4 = TMT_MINSIZE4,
    MinSize5 = TMT_MINSIZE5,
    NormalSize = TMT_NORMALSIZE,
    Offset = TMT_OFFSET,
    TextShadowOffset = TMT_TEXTSHADOWOFFSET
}

#[repr(i32)]
pub enum RectProp {
    AnimationButtonRect = TMT_ANIMATIONBUTTONRECT,
    AtlasRect = TMT_ATLASRECT,
    CustomSplitRect = TMT_CUSTOMSPLITRECT,
    DefaultPaneSize = TMT_DEFAULTPANESIZE
}

// #[repr(i32)]
// pub enum SizeProp {
//     CaptionBarHeight = TMT_CAPTIONBARHEIGHT,
//     CaptionBarWidth = TMT_CAPTIONBARWIDTH,
//     MenuBarHeight = TMT_MENUBARHEIGHT,
//     MenuBarWidth = TMT_MENUBARWIDTH,
//     PaddedBorderWidth = TMT_PADDEDBORDERWIDTH,
//     ScrollBarHeight = TMT_SCROLLBARHEIGHT,
//     ScrollBarWidth = TMT_SCROLLBARWIDTH,
//     SizingBorderWidth = TMT_SIZINGBORDERWIDTH,
//     SmCaptionBarHeight = TMT_SMCAPTIONBARHEIGHT,
//     SmCaptionBarWidth = TMT_SMCAPTIONBARWIDTH
// }

// #[repr(i32)]
// pub enum StringProp {
//     Alias = TMT_ALIAS,
//     AtlasInputImage = TMT_ATLASINPUTIMAGE,
//     Author = TMT_AUTHOR,
//     ClassicValue = TMT_CLASSICVALUE,
//     ColorSchemes = TMT_COLORSCHEMES,
//     Company = TMT_COMPANY,
//     Copyright = TMT_COPYRIGHT,
//     Description = TMT_DESCRIPTION,
//     DisplayName = TMT_DISPLAYNAME,
//     LastUpdated = TMT_LASTUPDATED,
//     Sizes = TMT_SIZES,
//     Text = TMT_TEXT,
//     Tooltip = TMT_TOOLTIP,
//     Url = TMT_URL,
//     Version = TMT_VERSION,
//     Name = TMT_NAME
// }
