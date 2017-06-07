use winapi::*;
use uxtheme;

use window::{BaseWindow, WindowBuilder, BlankBase};
use std::ptr;
use std::marker::{Send, Sync};

struct ThemeWindow(BlankBase);
unsafe impl Send for ThemeWindow {}
unsafe impl Sync for ThemeWindow {}

lazy_static!{
    static ref THEME_HWND: ThemeWindow = ThemeWindow(WindowBuilder::default().show_window(false).build_blank());
}

pub unsafe trait ThemeClass<P: Part> {
    fn htheme(&self) -> HTHEME;
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
        part BpCheckbox = 3 {
            CheckedDisabled = 8,
            CheckedHot = 6,
            CheckedNormal = 5,
            CheckedPressed = 7,
            MixedDisabled = 12,
            MixedHot = 10,
            MixedNormal = 9,
            MixedPressed = 11,
            UnCheckedDisabled = 4,
            UnCheckedHot = 2,
            UnCheckedNormal = 1,
            UnCheckedPressed = 3
        },
        part BpCommandLink = 6 {
            Defaulted = 5,
            DefaultedAnimating = 6,
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part BpCommandLinkGlyph = 7 {
            Defaulted = 5,
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part BpGroupBox = 4 {
            Disabled = 2,
            Normal = 1
        },
        part BpPushButton = 1 {
            Defaulted = 5,
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part BpRadioButton = 2 {
            CheckedDisabled = 8,
            CheckedHot = 6,
            CheckedNormal = 5,
            CheckedPressed = 7,
            UnCheckedDisabled = 4,
            UnCheckedHot = 2,
            UnCheckedNormal = 1,
            UnCheckedPressed = 3
        },
        part BpUserButton = 5
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
        part CpBackground = 2,
        part CpBorder = 4 {
            Disabled = 4,
            Focused = 3,
            Hot = 2,
            Normal = 1
        },
        part CpCueBanner = 8 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part CpDropdownButton = 1 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part CpDropdownButtonLeft = 7 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part CpDropdownButtonRight = 6 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part CpTransparentBackground = 3 {
            Disabled = 3,
            Focused = 4,
            Hot = 2,
            Normal = 1
        },
        part CpReadOnly = 5 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        }
    }
    pub class Communications
            where mod parts = communications_parts
    {
        part CsstTab = 1 {
            Hot = 2,
            Normal = 1,
            Selected = 3
        }
    }
    pub class ControlPanel
            where mod parts = control_panel_parts
    {
        part CPanelBannerArea = 18,
        part CPanelBodyText = 6,
        part CPanelBodyTitle = 19,
        part CPanelButton = 14,
        part CPanelContentLink = 10 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part CPanelContentPane = 2,
        part CPanelContentPaneLabel = 4,
        part CPanelContentPaneLine = 17,
        part CPanelGroupText = 9,
        part CPanelHelpLink = 7 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part CPanelLargeCommandArea = 12,
        part CPanelMessageText = 15,
        part CPanelNavigationPane = 1,
        part CPanelNavigationPaneLabel = 3,
        part CPanelNavigationPaneLine = 16,
        part CPanelSectionTitleLink = 11 {
            Hot = 2,
            Normal = 1
        },
        part CPanelSmallCommandArea = 13,
        part CPanelTaskLink = 8 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Page = 5,
            Pressed = 3
        },
        part CPanelTitle = 5
    }
    pub class DatePicker
            where mod parts = date_picker_parts
    {
        part DpDateBorder = 2 {
            Disabled = 4,
            Focused = 3,
            Hot = 2,
            Normal = 1
        },
        part DpDateText = 1 {
            Disabled = 2,
            Normal = 1,
            Selected = 3
        },
        part DpShowCalendarButtonRight = 3 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        }
    }
    pub class DragDrop
            where mod parts = drag_drop_parts
    {
        part DdCopy = 1 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdCreateLink = 4 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdImageBG = 7,
        part DdMove = 2 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdNone = 6 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdTextBG = 8,
        part DdUpdateMetadata = 3 {
            Highlight = 1,
            NoHighlight = 2
        },
        part DdWarning = 5 {
            Highlight = 1,
            NoHighlight = 2
        }
    }
    pub class Edit
            where mod parts = edit_parts
    {
        part EpBackground = 3 {
            Assist = 6,
            Disabled = 3,
            Focused = 4,
            Hot = 2,
            Normal = 1,
            ReadOnly = 5
        },
        part EpBackgroundWithBorder = 5 {
            Disabled = 3,
            Focused = 4,
            Hot = 2,
            Normal = 1
        },
        part EpCaret = 2,
        part EpEditBorderHScroll = 7 {
            Disabled = 4,
            Focused = 3,
            Hot = 2,
            Normal = 1
        },
        part EpEditBorderHVScroll = 9 {
            Disabled = 4,
            Focused = 3,
            Hot = 2,
            Normal = 1
        },
        part EpEditBorderNoScroll = 6 {
            Disabled = 4,
            Focused = 3,
            Hot = 2,
            Normal = 1
        },
        part EpEditBorderVScroll = 8 {
            Disabled = 4,
            Focused = 3,
            Hot = 2,
            Normal = 1
        },
        part EpEditText = 1 {
            Assist = 7,
            CueBanner = 8,
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            ReadOnly = 6,
            Selected = 3
        },
        part EpPassword = 4
    }
    pub class ExplorerBar
            where mod parts = explorer_bar_parts
    {
        part EbpHeaderBackground = 1,
        part EbpHeaderClose = 2 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part EbpHeaderPin = 3 {
            Hot = 2,
            Normal = 1,
            Pressed = 3,
            SelectedHot = 5,
            SelectedNormal = 4,
            SelectedPressed = 6
        },
        part EbpIeBarMenu = 4 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part EbpNormalGroupBackground = 5,
        part EbpNormalGroupCollapse = 6 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part EbpNormalGroupExpand = 7 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part EbpNormalGroupHead = 8,
        part EbpSpecialGroupBackground = 9,
        part EbpSpecialGroupCollapse = 10 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part EbpSpecialGroupExpand = 11 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part EbpSpecialGroupHead = 12
    }
    pub class Flyout
            where mod parts = flyout_parts
    {
        part FlyoutBody = 2 {
            Emphasized = 2,
            Normal = 1
        },
        part FlyoutDivider = 5,
        part FlyoutHeader = 1,
        part FlyoutLabel = 3 {
            Disabled = 4,
            Emphasized = 3,
            Normal = 1,
            Selected = 2
        },
        part FlyoutLink = 4 {
            Hover = 2,
            Normal = 1
        },
        part FlyoutLinkArea = 7,
        part FlyoutLinkHeader = 8 {
            Hover = 2,
            Normal = 1
        },
        part FlyoutWindow = 6
    }
    pub class Header
            where mod parts = header_parts
    {
        part HpHeaderDropdown = 5 {
            Hot = 3,
            Normal = 1,
            SoftHot = 2
        },
        part HpHeaderDropdownFilter = 6 {
            Hot = 3,
            Normal = 1,
            SoftHot = 2
        },
        part HpHeaderItem = 1 {
            Hot = 2,
            IconHot = 8,
            IconNormal = 7,
            IconPressed = 9,
            IconSortedHot = 11,
            IconSortedNormal = 10,
            IconSortedPressed = 12,
            Normal = 1,
            Pressed = 3,
            SortedNormal = 4,
            SortedHot = 5,
            SortedPressed = 6
        },
        part HpHeaderItemLeft = 2 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part HpHeaderItemRight = 3 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part HpHeaderOverflow = 7 {
            Hot = 2,
            Normal = 1
        },
        part HpHeaderSortArrow = 4 {
            SortedDown = 2,
            SortedUp = 1
        }
    }
    pub class ListBox
            where mod parts = list_box_parts
    {
        part LbcpBorderHScroll = 1 {
            Disabled = 4,
            Focused = 2,
            Hot = 3,
            Normal = 1
        },
        part LbcpBorderHVScroll = 2 {
            Disabled = 4,
            Focused = 2,
            Hot = 3,
            Normal = 1
        },
        part LbcpBorderNoScroll = 3 {
            Disabled = 4,
            Focused = 2,
            Hot = 3,
            Normal = 1
        },
        part LbcpBorderVScroll = 4 {
            Disabled = 4,
            Focused = 2,
            Hot = 3,
            Normal = 1
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
        part LvpCollapseButton = 9 {
            Hover = 2,
            Normal = 1,
            Pushed = 3
        },
        part LvpColumnDetail = 10,
        part LvpEmptyText = 5,
        part LvpExpandButton = 8 {
            Hover = 2,
            Normal = 1,
            Pushed = 3
        },
        part LvpGroupHeader = 6 {
            Close = 9,
            CloseHot = 10,
            CloseSelected = 11,
            CloseSelectedHot = 12,
            CloseSelectedNotFocused = 13,
            CloseSelectedNotFocusedHot = 14,
            CloseMixedSelection = 15,
            CloseMixedSelectionHot = 16,
            Open = 1,
            OpenHot = 2,
            OpenSelected = 3,
            OpenSelectedHot = 4,
            OpenSelectedNotFocused = 5,
            OpenSelectedNotFocusedHot = 6,
            OpenMixedSelection = 7,
            OpenMixedSelectionHot = 8
        },
        part LvpGroupHeaderLine = 7 {
            Close = 9,
            CloseHot = 10,
            CloseSelected = 11,
            CloseSelectedHot = 12,
            CloseSelectedNotFocused = 13,
            CloseSelectedNotFocusedHot = 14,
            CloseMixedSelection = 15,
            CloseMixedSelectionHot = 16,
            Open = 1,
            OpenHot = 2,
            OpenSelected = 3,
            OpenSelectedHot = 4,
            OpenSelectedNotFocused = 5,
            OpenSelectedNotFocusedHot = 6,
            OpenMixedSelection = 7,
            OpenMixedSelectionHot = 8
        },
        part LvpListGroup = 2,
        part LvpListDetail = 3,
        part LvpListItem = 1 {
            Disabled = 4,
            Hot = 2,
            HotSelected = 6,
            Normal = 1,
            Selected = 3,
            SelectedNotFocus = 5
        },
        part LvpListSortedDetail = 4
    }
    pub class Menu
            where mod parts = menu_parts
    {
        part MenuBarBackground = 7 {
            Active = 1,
            Inactive = 2
        },
        part MenuBarItem = 8 {
            Disabled = 4,
            DisabledHot = 5,
            DisabledPushed = 6,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part MenuChevronTmSchema = 5,
        part MenuMenuBarDropdownTmSchema = 4,
        part MenuMenuBarItemTmSchema = 3,
        part MenuMenuDropdownTmSchema = 2,
        part MenuMenuItemTmSchema = 1,
        part MenuPopupBackground = 9,
        part MenuPopupBorders = 10,
        part MenuPopupCheck = 11 {
            BulletDisabled = 4,
            BulletNormal = 3,
            CheckmarkDisabled = 2,
            CheckmarkNormal = 1
        },
        part MenuPopupCheckBackground = 12 {
            Bitmap = 3,
            Disabled = 1,
            Normal = 2
        },
        part MenuPopupGutter = 13,
        part MenuPopupItem = 14 {
            Disabled = 3,
            DisabledHot = 4,
            Hot = 2,
            Normal = 1
        },
        part MenuPopupSeparator = 15,
        part MenuPopupSubmenu = 16 {
            Disabled = 2,
            Normal = 1
        },
        part MenuSeparatorTmSchema = 6,
        part MenuSystemClose = 17 {
            Disabled = 2,
            Normal = 1
        },
        part MenuSystemMaximize = 18 {
            Disabled = 2,
            Normal = 1
        },
        part MenuSystemMinimize = 19 {
            Disabled = 2,
            Normal = 1
        },
        part MenuSystemRestore = 20 {
            Disabled = 2,
            Normal = 1
        }
    }
    pub class MenuBand
            where mod parts = menu_band_parts
    {
        part MdpNewAppButton = 1 {
            Checked = 5,
            Disabled = 4,
            Hot = 2,
            HotChecked = 6,
            Normal = 1,
            Pressed = 3
        },
        part MdpSeperator = 2
    }
    pub class Navigation
            where mod parts = navigation_parts
    {
        part NavBackButton = 1 {
            BbDisabled = 4,
            BbHot = 2,
            BbNormal = 1,
            BbPressed = 3
        },
        part NavForwardButton = 2 {
            FbDisabled = 4,
            FbHot = 2,
            FbNormal = 1,
            FbPressed = 3
        },
        part NavMenuButton = 3 {
            MbDisabled = 4,
            MbHot = 2,
            MbNormal = 1,
            MbPressed = 3
        }
    }
    pub class Page
            where mod parts = page_parts
    {
        part PgrpDown = 2 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part PgrpDownHorz = 4 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part PgrpUp = 1 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part PgrpUpHorz = 3 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
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
            Error = 2,
            Normal = 1,
            Partial = 4,
            Paused = 3
        },
        part PpFillVert = 6 {
            Error = 2,
            Normal = 1,
            Partial = 4,
            Paused = 3
        },
        part PpMoveOverlay = 8,
        part PpMoveOverlayVert = 10,
        part PpPulseOverlay = 7,
        part PpPulseOverlayVert = 9,
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
        part RpBackground = 6,
        part RpBand = 3,
        part RpChevron = 4 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part RpChevronVert = 5 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part RpGripper = 1,
        part RpGripperVert = 2,
        part RpSplitter = 7 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part RpSplitterVert = 8 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        }
    }
    pub class ScrollBar
            where mod parts = scroll_bar_parts
    {
        part SbpArrowBtn = 1 {
            DownDisabled = 8,
            DownHot = 6,
            DownNormal = 5,
            DownPressed = 7,
            DownHover = 18,
            LeftDisabled = 12,
            LeftHot = 10,
            LeftHover = 19,
            LeftNormal = 9,
            LeftPressed = 11,
            RightDisabled = 16,
            RightHot = 14,
            RightHover = 20,
            RightNormal = 13,
            RightPressed = 15,
            UpDisabled = 4,
            UpHot = 2,
            UpHover = 17,
            UpNormal = 1,
            UpPressed = 3
        },
        part SbpGripperHorz = 8 {
            Disabled = 4,
            Hot = 2,
            Hover = 5,
            Normal = 1,
            Pressed = 3
        },
        part SbpGripperVert = 9 {
            Disabled = 4,
            Hot = 2,
            Hover = 5,
            Normal = 1,
            Pressed = 3
        },
        part SbpLowerTrackHorz = 4 {
            Disabled = 4,
            Hot = 2,
            Hover = 5,
            Normal = 1,
            Pressed = 3
        },
        part SbpLowerTrackVert = 6 {
            Disabled = 4,
            Hot = 2,
            Hover = 5,
            Normal = 1,
            Pressed = 3
        },
        part SbpSizeBox = 10 {
            HalfBottomRightAlign = 5,
            HalfBottomLeftAlign = 6,
            HalfTopRightAlign = 7,
            HalfTopLeftAlign = 8,
            LeftAlign = 2,
            RightAlign = 1,
            TopRightAlign = 3,
            TopLeftAlign = 4
        },
        part SbpThumbBtnHorz = 2 {
            Disabled = 4,
            Hot = 2,
            Hover = 5,
            Normal = 1,
            Pressed = 3
        },
        part SbpThumbBtnVert = 3 {
            Disabled = 4,
            Hot = 2,
            Hover = 5,
            Normal = 1,
            Pressed = 3
        },
        part SbpUpperTrackHorz = 5 {
            Disabled = 4,
            Hot = 2,
            Hover = 5,
            Normal = 1,
            Pressed = 3
        },
        part SbpUpperTrackVert = 7 {
            Disabled = 4,
            Hot = 2,
            Hover = 5,
            Normal = 1,
            Pressed = 3
        }
    }
    pub class Spin
            where mod parts = spin_parts
    {
        part SpnpDown = 2 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part SpnpDownHorz = 4 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part SpnpUp = 1 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part SpnpUpHorz = 3 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        }
    }
    pub class StartPanel
            where mod parts = start_panel_parts
    {
        part SppLogoff = 8,
        part SppLogoffButtons = 9 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part SppMorePrograms = 2,
        part SppMoreProgramsArrow = 3 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part SppPlacesList = 6,
        part SppPlacesListSeparator = 7,
        part SppPreView = 11,
        part SppProgList = 4,
        part SppProgListSeparator = 5,
        part SppUserPane = 1,
        part SppUserpicture = 10
    }
    pub class Status
            where mod parts = status_parts
    {
        part SpGripper = 3,
        part SpGripperPane = 2,
        part SpPane = 1
    }
    pub class Tab
            where mod parts = tab_parts
    {
        part TabpAeroWizardBody = 11,
        part TabpBody = 10,
        part TabpPane = 9,
        part TabpTabItem = 1 {
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            Selected = 3
        },
        part TabpTabItemBothEdge = 4 {
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            Selected = 3
        },
        part TabpTabItemLeftEdge = 2 {
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            Selected = 3
        },
        part TabpTabItemRightEdge = 3 {
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            Selected = 3
        },
        part TabpTopTabItem = 5 {
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            Selected = 3
        },
        part TabpTopTabItemBothEdge = 8 {
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            Selected = 3
        },
        part TabpTopTabItemLeftEdge = 6 {
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            Selected = 3
        },
        part TabpTopTabItemRightEdge = 7 {
            Disabled = 4,
            Focused = 5,
            Hot = 2,
            Normal = 1,
            Selected = 3
        }
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
        part TbpBackgroundLeft = 4,
        part TbpBackgroundRight = 2,
        part TbpBackgroundTop = 3,
        part TbpSizingBarBottom = 5,
        part TbpSizingBarRight = 6,
        part TbpSizingBarTop = 7
    }
    pub class TaskDialog
            where mod parts = task_dialog_parts
    {
        part TdlgButtonSection = 10,
        part TdlgButtonWrapper = 11,
        part TdlgCommandLinkPane = 7,
        part TdlgContentIcon = 5,
        part TdlgContentPane = 4 {
            Standalone = 1
        },
        part TdlgControlPane = 9,
        part TdlgExpandedcontent = 6,
        part TdlgExpandedfooterArea = 18,
        part TdlgExpandoButton = 13 {
            ExpandedHover = 5,
            ExpandedNormal = 4,
            ExpandedPressed = 6,
            Hover = 2,
            Normal = 1,
            Pressed = 3
        },
        part TdlgExpandoText = 12,
        part TdlgFootNoteArea = 16,
        part TdlgFootNotePane = 15,
        part TdlgFootNoteSeparator = 17,
        part TdlgImageAlignment = 20,
        part TdlgMainIcon = 3,
        part TdlgMainInstructionPane = 2,
        part TdlgPrimaryPanel = 1,
        part TdlgProgressBar = 19,
        part TdlgRadioButtonPane = 21,
        part TdlgSecondaryPanel = 8,
        part TdlgVerificationText = 14
    }
    pub class TextStyle
            where mod parts = text_style_parts
    {
        part TextBodyTitle = 3,
        part TextBodyText = 4,
        part TextControlLabel = 9 {
            ControlLabelDisabled = 2,
            ControlLabelNormal = 1
        },
        part TextExpanded = 7,
        part TextHyperLinkText = 6 {
            HyperLinkDisabled = 4,
            HyperLinkHot = 2,
            HyperLinkNormal = 1,
            HyperLinkPressed = 3
        },
        part TextInstruction = 2,
        part TextLabel = 8,
        part TextMainInstruction = 1,
        part TextSecondaryText = 5
    }
    pub class ToolBar
            where mod parts = tool_bar_parts
    {
        part TpButton = 1 {
            Checked = 5,
            Disabled = 4,
            Hot = 2,
            HotChecked = 6,
            NearHot = 7,
            Normal = 1,
            OtherSideHot = 8,
            Pressed = 3
        },
        part TpDropdownButton = 2 {
            Checked = 5,
            Disabled = 4,
            Hot = 2,
            HotChecked = 6,
            NearHot = 7,
            Normal = 1,
            OtherSideHot = 8,
            Pressed = 3
        },
        part TpDropdownButtonGlyph = 7 {
            Checked = 5,
            Disabled = 4,
            Hot = 2,
            HotChecked = 6,
            NearHot = 7,
            Normal = 1,
            OtherSideHot = 8,
            Pressed = 3
        },
        part TpSeparator = 5 {
            Checked = 5,
            Disabled = 4,
            Hot = 2,
            HotChecked = 6,
            NearHot = 7,
            Normal = 1,
            OtherSideHot = 8,
            Pressed = 3
        },
        part TpSeparatorVert = 6 {
            Checked = 5,
            Disabled = 4,
            Hot = 2,
            HotChecked = 6,
            NearHot = 7,
            Normal = 1,
            OtherSideHot = 8,
            Pressed = 3
        },
        part TpSplitButton = 3 {
            Checked = 5,
            Disabled = 4,
            Hot = 2,
            HotChecked = 6,
            NearHot = 7,
            Normal = 1,
            OtherSideHot = 8,
            Pressed = 3
        },
        part TpSplitButtonDropdown = 4 {
            Checked = 5,
            Disabled = 4,
            Hot = 2,
            HotChecked = 6,
            NearHot = 7,
            Normal = 1,
            OtherSideHot = 8,
            Pressed = 3
        }
    }
    pub class Tooltip
            where mod parts = tooltip_parts
    {
        part TtpBalloon = 3 {
            Link = 2,
            Normal = 1
        },
        part TtpBalloonStem = 6 {
            PointingUpLeftWall = 1,
            PointingUpCentered = 2,
            PointingUpRightWall = 3,
            PointingDownRightWall = 4,
            PointingDownCentered = 5,
            PointingDownLeftWall = 6
        },
        part TtpBalloonTitle = 4,
        part TtpClose = 5 {
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part TtpStandard = 1 {
            Link = 2,
            Normal = 1
        },
        part TtpStandardTitle = 2 {
            Link = 2,
            Normal = 1
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
        part TkpThumb = 3 {
            Disabled = 5,
            Focused = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part TkpThumbBottom = 4 {
            Disabled = 5,
            Focused = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part TkpThumbLeft = 7 {
            Disabled = 5,
            Focused = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part TkpThumbRight = 8 {
            Disabled = 5,
            Focused = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part TkpThumbTop = 5 {
            Disabled = 5,
            Focused = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part TkpThumbVert = 6 {
            Disabled = 5,
            Focused = 4,
            Hot = 2,
            Normal = 1,
            Pressed = 3
        },
        part TkpTics = 9 {
            Normal = 1
        },
        part TkpTicsVert = 10 {
            Normal = 1
        },
        part TkpTrack = 1 {
            Normal = 1
        },
        part TkpTrackVert = 2 {
            Normal = 1
        }
    }
    pub class TrayNotify
            where mod parts = tray_botify_parts
    {
        part TnpAnimBackground = 2,
        part TnpBackground = 1
    }
    pub class TreeView
            where mod parts = tree_view_parts
    {
        part TvpBranch = 3,
        part TvpGlyph = 2 {
            Closed = 1,
            Opened = 2
        },
        part TvpHotGlyph = 4 {
            Closed = 1,
            Opened = 2
        },
        part TvpTreeItem = 1 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Selected = 3,
            SelectedNotFocus = 5,
            HotSelected = 6
        }
    }
    pub class Window
            where mod parts = window_parts
    {
        part WpCaption = 1 {
            Active = 1,
            Disabled = 3,
            Inactive = 2
        },
        part WpCaptionSizingTemplate = 30,
        part WpCloseButton = 18 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpDialog = 29,
        part WpFrame = 38 {
            Active = 1,
            Inactive = 2
        },
        part WpFrameBottom = 9,
        part WpFrameBottomSizingTemplate = 36,
        part WpFrameLeft = 7,
        part WpFrameLeftSizingTemplate = 32,
        part WpFrameRight = 8,
        part WpFrameRightSizingTemplate = 34,
        part WpHelpButton = 23 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpHorzScroll = 25 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpHorzThumb = 26 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpMaxButton = 17 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpMaxCaption = 5 {
            Active = 1,
            Disabled = 3,
            Inactive = 2
        },
        part WpMdiCloseButton = 20,
        part WpMdiHelpButton = 24,
        part WpMdiMinButton = 16,
        part WpMdiRestoreButton = 22,
        part WpMdiSysButton = 14,
        part WpMinButton = 15 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpMinCaption = 3 {
            Active = 1,
            Disabled = 3,
            Inactive = 2
        },
        part WpRestoreButton = 21 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpSmallCaption = 2,
        part WpSmallCaptionSizingTemplate = 31,
        part WpSmallCloseButton = 19,
        part WpSmallFrameBottom = 12,
        part WpSmallFrameBottomSizingTemplate = 37,
        part WpSmallFrameLeft = 10,
        part WpSmallFrameLeftSizingTemplate = 33,
        part WpSmallFrameRight = 11,
        part WpSmallFrameRightSizingTemplate = 35,
        part WpSmallMaxCaption = 6,
        part WpSmallMinCaption = 4,
        part WpSysButton = 13 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpVertScroll = 27 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        },
        part WpVertThumb = 28 {
            Disabled = 4,
            Hot = 2,
            Normal = 1,
            Pushed = 3
        }
    }
}
