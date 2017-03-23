use std::marker::PhantomData;
use dct::geometry::{Px, OriginRect, OffsetRect};
use std::{slice, mem, ptr, str};

pub trait UserMsgConverter<M: UserMsg> {
    unsafe fn push_param<P>(&mut self, offset: usize)
            where P: MsgParam;
}

pub trait UserMsg: Sized + Copy {
    fn discriminant(&self) -> u16;
    unsafe fn empty(discriminant: u16) -> Self;
    /// Give a converter the functions necessary for decoding and encoding this `UserMsg`. This
    /// function *must* be deterministic - i.e. the same discriminant must always give the same
    /// type parameters to `convert_fns` in `C`.
    fn register_conversion<C>(discriminant: u16, &mut C)
            where C: UserMsgConverter<Self>;
}

impl UserMsg for () {
    fn discriminant(&self) -> u16 {0}
    unsafe fn empty(_: u16) -> () {()}
    fn register_conversion<C>(_: u16, _: &mut C)
            where C: UserMsgConverter<()> {}
}

pub struct ParamLock<'a, T>( T, PhantomData<&'a ()> );
impl<'a, T> ParamLock<'a, T> {
    pub fn read(self) -> T {
        self.0
    }
}

pub unsafe trait PrimMsgParam: Copy + Sized {}

pub trait MsgParam {
    type EncodedParam: PrimMsgParam;
    fn encode(self) -> Self::EncodedParam;
    fn decode(ParamLock<Self::EncodedParam>) -> Self;
}

macro_rules! primitive_msg_params {
    () => {};
    (impl MsgParam for $name:ty; $($rest:tt)*) => {
        unsafe impl PrimMsgParam for $name {}
        impl MsgParam for $name {
            type EncodedParam = $name;
            #[inline]
            fn encode(self) -> $name {
                self
            }
            #[inline]
            fn decode(encoded: ParamLock<$name>) -> $name {
                encoded.read()
            }
        }

        primitive_msg_params!{$($rest)*}
    };
    (impl<$($generic:ident $(: $($generic_constraint:path)|*)*),*> MsgParam for $name:ty; $($rest: tt)*) => {
        unsafe impl<$($generic $(: $($generic_constraint +)*)*),*> PrimMsgParam for $name {}
        impl<$($generic $(: $($generic_constraint +)*)*),*> MsgParam for $name {
            type EncodedParam = $name;
            #[inline]
            fn encode(self) -> $name {
                self
            }
            #[inline]
            fn decode(encoded: ParamLock<$name>) -> $name {
                encoded.read()
            }
        }

        primitive_msg_params!{$($rest)*}
    }
}

primitive_msg_params!{
    impl MsgParam for bool;
    impl MsgParam for char;
    impl MsgParam for f32;
    impl MsgParam for f64;

    impl MsgParam for u8;
    impl MsgParam for u16;
    impl MsgParam for u32;
    impl MsgParam for u64;
    impl MsgParam for i8;
    impl MsgParam for i16;
    impl MsgParam for i32;
    impl MsgParam for i64;

    impl MsgParam for ();

    impl<T: PrimMsgParam> MsgParam for [T; 1];
    impl<T: PrimMsgParam> MsgParam for [T; 2];
    impl<T: PrimMsgParam> MsgParam for [T; 3];
    impl<T: PrimMsgParam> MsgParam for [T; 4];
    impl<T: PrimMsgParam> MsgParam for [T; 5];
    impl<T: PrimMsgParam> MsgParam for [T; 6];
    impl<T: PrimMsgParam> MsgParam for [T; 7];
    impl<T: PrimMsgParam> MsgParam for [T; 8];
    impl<T: PrimMsgParam> MsgParam for [T; 9];
    impl<T: PrimMsgParam> MsgParam for [T; 10];
    impl<T: PrimMsgParam> MsgParam for [T; 11];
    impl<T: PrimMsgParam> MsgParam for [T; 12];
    impl<T: PrimMsgParam> MsgParam for [T; 13];
    impl<T: PrimMsgParam> MsgParam for [T; 14];
    impl<T: PrimMsgParam> MsgParam for [T; 15];
    impl<T: PrimMsgParam> MsgParam for [T; 16];
}

impl<'a, T> MsgParam for &'a T {
    type EncodedParam = u64;
    fn encode(self) -> u64 {
        self as *const T as u64
    }
    fn decode(encoded: ParamLock<u64>) -> &'a T {
        unsafe{ &*(encoded.read() as *const T) }
    }
}

impl<'a, T> MsgParam for &'a mut T {
    type EncodedParam = u64;
    fn encode(self) -> u64 {
        self as *mut T as u64
    }
    fn decode(encoded: ParamLock<u64>) -> &'a mut T {
        unsafe{ &mut *(encoded.read() as *mut T) }
    }
}

impl<T> MsgParam for *const T {
    type EncodedParam = u64;
    fn encode(self) -> u64 {
        self as u64
    }
    fn decode(encoded: ParamLock<u64>) -> *const T {
        encoded.read() as *const T
    }
}

impl<T> MsgParam for *mut T {
    type EncodedParam = u64;
    fn encode(self) -> u64 {
        self as u64
    }
    fn decode(encoded: ParamLock<u64>) -> *mut T {
        encoded.read() as *mut T
    }
}

impl MsgParam for usize {
    type EncodedParam = u64;
    fn encode(self) -> u64 {
        self as u64
    }
    fn decode(encoded: ParamLock<u64>) -> usize {
        encoded.read() as usize
    }
}

impl MsgParam for isize {
    type EncodedParam = i64;
    fn encode(self) -> i64 {
        self as i64
    }
    fn decode(encoded: ParamLock<i64>) -> isize {
        encoded.read() as isize
    }
}

impl MsgParam for OffsetRect {
    type EncodedParam = [Px; 4];

    fn encode(self) -> [Px; 4] {
        [self.topleft.x, self.topleft.y, self.lowright.x, self.lowright.y]
    }

    fn decode(encoded: ParamLock<[Px; 4]>) -> OffsetRect {
        let encoded = encoded.read();
        OffsetRect::new(
            encoded[0],
            encoded[1],
            encoded[2],
            encoded[3]
        )
    }
}

impl MsgParam for OriginRect {
    type EncodedParam = [Px; 2];

    fn encode(self) -> [Px; 2] {
        [self.width, self.height]
    }

    fn decode(encoded: ParamLock<[Px; 2]>) -> OriginRect {
        let encoded = encoded.read();
        OriginRect::new(encoded[0], encoded[1])
    }
}

impl<'a, T> MsgParam for &'a [T] {
    type EncodedParam = [u64; 2];

    fn encode(self) -> [u64; 2] {
        [self.as_ptr() as u64, self.len() as u64]
    }
    fn decode(encoded: ParamLock<[u64; 2]>) -> &'a [T] {
        let encoded = encoded.read();
        unsafe{ slice::from_raw_parts(encoded[0] as *const T, encoded[1] as usize) }
    }
}

impl<'a, T> MsgParam for &'a mut [T] {
    type EncodedParam = [u64; 2];

    fn encode(self) -> [u64; 2] {
        [self.as_ptr() as u64, self.len() as u64]
    }
    fn decode(encoded: ParamLock<[u64; 2]>) -> &'a mut [T] {
        let encoded = encoded.read();
        unsafe{ slice::from_raw_parts_mut(encoded[0] as *mut T, encoded[1] as usize) }
    }
}

impl<'a> MsgParam for &'a str {
    type EncodedParam = [u64; 2];

    fn encode(self) -> [u64; 2] {
        [self.as_ptr() as u64, self.len() as u64]
    }
    fn decode(encoded: ParamLock<[u64; 2]>) -> &'a str {
        let encoded = encoded.read();
        unsafe{ str::from_utf8_unchecked(slice::from_raw_parts(encoded[0] as *const u8, encoded[1] as usize)) }
    }
}

#[must_use = "UserMsgEncoder must be forgotten - it cannot be dropped"]
struct UserMsgEncoder<M: UserMsg> {
    bytes: [u8; 16],
    cursor: usize,
    msg: M
}

impl<M: UserMsg> UserMsgEncoder<M> {
    fn new(msg: M) -> UserMsgEncoder<M> {
        UserMsgEncoder {
            bytes: [0; 16],
            cursor: 0,
            msg: msg
        }
    }
}

impl<M: UserMsg> UserMsgConverter<M> for UserMsgEncoder<M> {
    unsafe fn push_param<P>(&mut self, offset: usize)
            where P: MsgParam
    {
        let param = (&self.msg as *const M as *const u8).offset(offset as isize) as *const P;
        let param_bytes = slice::from_raw_parts(param as *const u8, mem::size_of::<P>());
        self.bytes[self.cursor..self.cursor + param_bytes.len()].copy_from_slice(param_bytes);
        self.cursor += param_bytes.len();
    }
}

impl<M: UserMsg> Drop for UserMsgEncoder<M> {
    fn drop(&mut self) {
        #[cfg(not(test))]
        panic!("UserMsgEncoder cannot be dropped! It must be forgotten")
    }
}

struct UserMsgDecoder<M: UserMsg> {
    bytes: [u8; 16],
    cursor: usize,
    msg: M
}

impl<M: UserMsg> UserMsgDecoder<M> {
    fn new(bytes: [u8; 16], discriminant: u16) -> UserMsgDecoder<M> {
        UserMsgDecoder {
            bytes: bytes,
            cursor: 0,
            msg: unsafe{ M::empty(discriminant) }
        }
    }
}

impl<M: UserMsg> UserMsgConverter<M> for UserMsgDecoder<M> {
    unsafe fn push_param<P>(&mut self, offset: usize)
            where P: MsgParam
    {
        let param_bytes = &mut self.bytes[self.cursor..self.cursor + mem::size_of::<P>()];
        let param = param_bytes.as_mut_ptr() as *mut P;
        ptr::swap(param, (&mut self.msg as *mut M as *mut u8).offset(offset as isize) as *mut P);
        self.cursor += param_bytes.len();
    }
}

pub fn encode<M: UserMsg>(msg: M) -> [u8; 16] {
    let discriminant = msg.discriminant();
    let mut encoder = UserMsgEncoder::new(msg);
    M::register_conversion(discriminant, &mut encoder);
    let encoded_bytes = encoder.bytes;
    mem::forget(encoder);
    encoded_bytes
}

pub unsafe fn decode<M: UserMsg>(discriminant: u16, bytes: [u8; 16]) -> M {
    let mut decoder = UserMsgDecoder::new(bytes, discriminant);
    M::register_conversion(discriminant, &mut decoder);
    decoder.msg
}
