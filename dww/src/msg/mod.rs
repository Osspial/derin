pub mod queue;
pub mod user;
pub mod notify;

use self::user::UserMsg;
use self::notify::Notification;
use dct::geometry::*;
use dct::buttons::*;
use dct::hints::SizeBounds;
use ucs2::Ucs2Str;
use gdi::PaintInit;

#[derive(Debug)]
pub enum Msg<'a, U: UserMsg> {
    Close,
    Size(OriginRect),
    MouseDown(MouseButton, Point),
    MouseDoubleDown(MouseButton, Point),
    MouseUp(MouseButton, Point),
    KeyDown(Key, RepeatedPress),
    KeyUp(Key, RepeatedPress),
    SetText(&'a Ucs2Str),
    Paint(PaintInit<'a>),
    EraseBackground,
    Notify(Notification),
    GetSizeBounds(&'a mut SizeBounds),
    User(U)
}

pub type RepeatedPress = bool;
