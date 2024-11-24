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
        format!("#{:X}{:X}{:X}", self.r, self.g, self.b)
    }
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