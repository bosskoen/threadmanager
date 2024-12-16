#[derive(Debug, Clone, Copy)]
pub struct RGB{
    r:u8,
    g:u8,
    b:u8
}
impl RGB {
    pub fn new( r:u8, g: u8, b: u8) -> Self{
        Self{r,g,b}
    }
    pub fn from_hex(hex: u32) -> Self{
        Self::new(((hex >> 16) & 255)as u8, ((hex >> 8) & 255) as u8, (hex & 255) as u8)
    }
    pub fn to_hex(&self) -> String{
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    #[allow(non_snake_case)]
    pub fn RED()->Self{Self::new(255,0,0)}
    #[allow(non_snake_case)]
    pub fn GREEN() -> Self { Self::new(0, 255, 0) }
    #[allow(non_snake_case)]
    pub fn BLUE() -> Self { Self::new(0, 0, 255) }
    #[allow(non_snake_case)]
    pub fn WHITE() -> Self { Self::new(255, 255, 255) }
    #[allow(non_snake_case)]
    pub fn BLACK() -> Self { Self::new(0, 0, 0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_converter() {
        let x = RGB::from_hex(0x66f41e);
        assert_eq!(x.to_hex(), "#66F41E");
    }
}