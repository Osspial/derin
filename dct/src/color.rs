#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color24 {
    pub red: u8,
    pub green: u8,
    pub blue: u8
}

impl Color24 {
    #[inline]
    pub fn new(red: u8, green: u8, blue: u8) -> Color24 {
        Color24{ red, green, blue }
    }

    #[inline]
    pub fn white() -> Color24 {
        Color24::new(255, 255, 255)
    }

    #[inline]
    pub fn black() -> Color24 {
        Color24::new(0, 0, 0)
    }
}
