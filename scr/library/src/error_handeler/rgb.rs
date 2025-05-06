#[derive(Debug, Clone, Copy)]
pub struct RGB {
    r: u8,
    g: u8,
    b: u8
}

impl RGB {
    /// Creates a new RGB color.
    ///
    /// # Arguments
    ///
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    ///
    /// # Example
    ///
    /// ```
    /// let color = RGB::new(255, 0, 0); // Red
    /// ```
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Creates an RGB color from a hexadecimal value.
    ///
    /// # Arguments
    ///
    /// * `hex` - Hexadecimal color value (e.g., 0xFF0000 for red)
    ///
    /// # Example
    ///
    /// ```
    /// let color = RGB::from_hex(0xFF0000); // Red
    /// ```
    pub fn from_hex(hex: u32) -> Self {
        Self::new(((hex >> 16) & 255) as u8, ((hex >> 8) & 255) as u8, (hex & 255) as u8)
    }

    /// Converts the RGB color to a hexadecimal string.
    ///
    /// # Example
    ///
    /// ```
    /// let color = RGB::new(255, 0, 0);
    /// assert_eq!(color.to_hex(), "#FF0000");
    /// ```
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Converts the RGB color to a tuple.
    ///
    /// # Example
    ///
    /// ```
    /// let color = RGB::new(255, 0, 0);
    /// assert_eq!(color.to_tuple(), (255, 0, 0));
    /// ```
    pub fn to_tuple(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    #[allow(non_snake_case)]
    /// Predefined red color (255, 0, 0).
    pub fn RED() -> Self { Self::new(255, 0, 0) }
    #[allow(non_snake_case)]
    /// Predefined green color (0, 255, 0).
    pub fn GREEN() -> Self { Self::new(0, 255, 0) }
    #[allow(non_snake_case)]
    /// Predefined blue color (0, 0, 255).
    pub fn BLUE() -> Self { Self::new(0, 0, 255) }
    #[allow(non_snake_case)]
    /// Predefined white color (255, 255, 255).
    pub fn WHITE() -> Self { Self::new(255, 255, 255) }
    #[allow(non_snake_case)]
    /// Predefined black color (0, 0, 0).
    pub fn BLACK() -> Self { Self::new(0, 0, 0) }
    #[allow(non_snake_case)]
    /// Predefined yellow color (255, 255, 0).
    pub fn YELLOW() -> Self { Self::new(255, 255, 0) } 
    #[allow(non_snake_case)]
    /// Predefined cyan color (0, 255, 255).
    pub fn CYAN() -> Self { Self::new(0, 255, 255) } 
    #[allow(non_snake_case)]
    /// Predefined magenta color (255, 0, 255).
    pub fn MAGENTA() -> Self { Self::new(255, 0, 255) } 


    // Preset colors for different message levels
    #[allow(non_snake_case)]
    /// Predefined bright red color for critical errors (255, 15, 15).
    pub fn CRITICAL_ERROR() -> Self { Self::new(255, 15, 15) }
    #[allow(non_snake_case)]
    /// Predefined orange red color for errors (255, 69, 0).
    pub fn ERROR() -> Self { Self::new(255, 69, 0) } 
    #[allow(non_snake_case)]
    /// Predefined orange color for warnings (255, 165, 0).
    pub fn WARNING() -> Self { Self::new(255, 165, 0) }
    #[allow(non_snake_case)]
    /// Predefined deep sky blue color for informational messages (0, 191, 255).
    pub fn INFO() -> Self { Self::new(0, 191, 255) } 
    #[allow(non_snake_case)]
    /// Predefined light green color for debug messages (144, 238, 144).
    pub fn DEBUG() -> Self { Self::new(144, 238, 144) } 
    // Additional colors
    #[allow(non_snake_case)]
    /// Predefined dark gray color for trace messages (169, 169, 169).
    pub fn TRACE() -> Self { Self::new(169, 169, 169) }
    #[allow(non_snake_case)]
    /// Predefined lime green color for success messages (50, 205, 50).
    pub fn SUCCESS() -> Self { Self::new(50, 205, 50) } 
    #[allow(non_snake_case)]
    /// Predefined steel blue color for notices (70, 130, 180).
    pub fn NOTICE() -> Self { Self::new(70, 130, 180) } 
    #[allow(non_snake_case)]
    /// Predefined gold color for alerts (255, 215, 0).
    pub fn ALERT() -> Self { Self::new(255, 215, 0) }
    #[allow(non_snake_case)]
    /// Predefined deep pink color for emergencies (255, 20, 147).
    pub fn EMERGENCY() -> Self { Self::new(255, 20, 147) } 


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
