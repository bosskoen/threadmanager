use super::RGB;

#[cfg(feature = "GPIO")]
mod led {

    use super::*;
    use rppal;

    const PCA9685: u8 = 0x40;
    const MODE1: u8 = 0x00;
    const LED1: u8 = 0x06;
    const LED_SUB1: u8 = LED1+4;
    const LED_SUB2: u8 = LED1+8;
    const PRE_SCALE: u8 = 0xFE;
    const MAX_LEVEL: u8 = 16;
    const GAMMA: f32 = 2.2;
    

    pub struct LedController {
        pin: rppal::gpio::OutputPin,
        control: rppal::i2c::I2c,
        color: RGB,
        brightness: u8,
    }

    impl Drop for LedController{
        fn drop(&mut self) {
            self.set_color(RGB::BLACK());
            self.red_reset();
            self.green_reset();
            self.blue_reset();
        }
    }

    impl LedController {
        pub fn new(color: RGB, led_level: u8)-> Result<Self, rppal::gpio::Error> {
            let pin =rppal::gpio::Gpio::new()?.get(4)?.into_output_low();
            let i2c =rppal::i2c::I2c::new()?;
            i2c.set_slave_address(PCA9685)?;
            i2c.smbus_write_byte(MODE1, 0b0011_0001)?;
            i2c.smbus_write_byte(PRE_SCALE, 101)?;
            i2c.smbus_write_byte(MODE1, 0b0010_0001)?;
            i2c.block_write(LED1, &map_values(color, led_level, [0;6]))?;
            Ok(Self { pin, color, brightness: led_level, control: i2c })
        }
        pub fn off(&mut self){
            self.pin.set_high();
        }
        pub fn on(&mut self){
            self.pin.set_low();
        }
        pub fn set_color(&mut self, color: RGB) -> Result<(), rppal::i2c::Error> {
            let mut values: [u8; 6] = [0; 6];
            values [0] = self.control.smbus_read_byte(LED1+1)?;
            values [1] = self.control.smbus_read_byte(LED1+3)?;
            values [2] = self.control.smbus_read_byte(LED_SUB1+1)?;
            values [3] = self.control.smbus_read_byte(LED_SUB1+3)?;
            values [4] = self.control.smbus_read_byte(LED_SUB2+1)?;
            values [5] = self.control.smbus_read_byte(LED_SUB2+3)?;
            self.control.block_write(LED1, &map_values(color, self.brightness, values))?;
            self.color = color;
            Ok(())
        }
        pub fn set_brightness(&mut self, level: u8) -> Result<(), rppal::i2c::Error> {
            let mut values: [u8; 6] = [0; 6];
            values [0] = self.control.smbus_read_byte(LED1+1)?;
            values [1] = self.control.smbus_read_byte(LED1+3)?;
            values [2] = self.control.smbus_read_byte(LED_SUB1+1)?;
            values [3] = self.control.smbus_read_byte(LED_SUB1+3)?;
            values [4] = self.control.smbus_read_byte(LED_SUB2+1)?;
            values [5] = self.control.smbus_read_byte(LED_SUB2+3)?;
            self.control.block_write(LED1, &map_values(self.color, level, values))?;
            self.brightness = level;
            Ok(())
        }

        pub fn red_on(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED1 + 1)?;
            self.control.smbus_write_byte(LED1 + 1, value | 0x10)?;
            value = self.control.smbus_read_byte(LED1 + 3)?;
            self.control.smbus_write_byte(LED1 + 3, value & 0x0F)?;
            Ok(())
        }
        pub fn red_off(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED1 + 1)?;
            self.control.smbus_write_byte(LED1 + 1, value & 0x0F)?;
            value = self.control.smbus_read_byte(LED1 + 3)?;
            self.control.smbus_write_byte(LED1 + 3, value | 0x10)?;
            Ok(())
        }
        pub fn red_reset(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED1 + 1)?;
            self.control.smbus_write_byte(LED1 + 1, value & 0x0F)?;
            value = self.control.smbus_read_byte(LED1 + 3)?;
            self.control.smbus_write_byte(LED1 + 3, value & 0x0F)?;
            Ok(())
        }
        pub fn green_on(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED_SUB1 + 1)?;
            self.control.smbus_write_byte(LED_SUB1 + 1, value | 0x10)?;
            value = self.control.smbus_read_byte(LED_SUB1 + 3)?;
            self.control.smbus_write_byte(LED_SUB1 + 3, value & 0x0F)?;
            Ok(())
        }
        pub fn green_off(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED_SUB1 + 1)?;
            self.control.smbus_write_byte(LED_SUB1 + 1, value & 0x0F)?;
            value = self.control.smbus_read_byte(LED_SUB1 + 3)?;
            self.control.smbus_write_byte(LED_SUB1 + 3, value | 0x10)?;
            Ok(())
        }
        pub fn green_reset(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED_SUB1 + 1)?;
            self.control.smbus_write_byte(LED_SUB1 + 1, value & 0x0F)?;
            value = self.control.smbus_read_byte(LED_SUB1 + 3)?;
            self.control.smbus_write_byte(LED_SUB1 + 3, value & 0x0F)?;
            Ok(())
        }
        pub fn blue_on(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED_SUB2 + 1)?;
            self.control.smbus_write_byte(LED_SUB2 + 1, value | 0x10)?;
            value = self.control.smbus_read_byte(LED_SUB2 + 3)?;
            self.control.smbus_write_byte(LED_SUB2 + 3, value & 0x0F)?;
            Ok(())
        }
        pub fn blue_off(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED_SUB2 + 1)?;
            self.control.smbus_write_byte(LED_SUB2 + 1, value & 0x0F)?;
            value = self.control.smbus_read_byte(LED_SUB2 + 3)?;
            self.control.smbus_write_byte(LED_SUB2 + 3, value | 0x10)?;
            Ok(())
        }
        pub fn blue_reset(&mut self) -> Result<(), rppal::i2c::Error> {
            let mut value = self.control.smbus_read_byte(LED_SUB2 + 1)?;
            self.control.smbus_write_byte(LED_SUB2 + 1, value & 0x0F)?;
            value = self.control.smbus_read_byte(LED_SUB2 + 3)?;
            self.control.smbus_write_byte(LED_SUB2 + 3, value & 0x0F)?;
            Ok(())
        }
    }

    fn map_values(color: RGB, level: u8, existingvalues: [u8; 6]) -> [u8; 12] {
        let mut brightness = level.clamp(0, MAX_LEVEL);
        if brightness == 0 {
            return [0; 12];
        }
        let brightness_factor = (brightness as f32 / MAX_LEVEL as f32).powf(GAMMA);
        let (r,g,b) =color.to_tuple();
        let mut values = [0 as u16; 3];
        values[0] = (((r * 4095 / 255) as f32)* brightness_factor) as u16;
        values[1] = (((g * 4095 / 255) as f32)* brightness_factor) as u16;
        values[2] = (((b * 4095 / 255) as f32)* brightness_factor) as u16;
        let mut result= [0; 12];
        for (i, value) in values.iter().enumerate(){
            result[i * 4] = 0;
            result[i * 4 + 1] = 0 | (existingvalues[i * 2] & 0x10);
            result[i * 4 + 2] = (value & 0xFF) as u8;
            result[i * 4 + 3] = ((value >> 8) as u8 | (existingvalues[i*2 +1] & 0x10));
        }
        result
    }
}

#[cfg(not(feature = "GPIO"))]
mod led {

    use crate::error_handeler::{print_interup, RGB};
    pub struct LedController {
        color: RGB,
        brightness: u8,
    }

    impl LedController {
        pub fn new(color: RGB, led_level: u8) -> Self {
            Self { color, brightness: led_level }
        }
        pub fn off(&mut self) {
            print_interup("led_controler","LED turned off", RGB::NOTICE());
        }
        pub fn on(&mut self) {
            print_interup("led_controler","LED turned on", RGB::NOTICE());
        }
        pub fn set_color(&mut self, color: RGB) {
            self.color = color;
            print_interup("led_controler",&format!("LED color set to {:?}", color), RGB::NOTICE());
        }
        pub fn set_brightness(&mut self, level: u8) {
            self.brightness = level;
            print_interup("led_controler",&format!("LED brightness set to {}", level), RGB::NOTICE());
        }
        pub fn red_on(&mut self) {
            print_interup("led_controler","Red LED turned on", RGB::NOTICE());
        }
        pub fn red_off(&mut self) {
            print_interup("led_controler","Red LED turned off", RGB::NOTICE());
        }
        pub fn red_reset(&mut self) {
            print_interup("led_controler","Red LED reset", RGB::NOTICE());
        }
        pub fn green_on(&mut self) {
            print_interup("led_controler","Green LED turned on", RGB::NOTICE());
        }
        pub fn green_off(&mut self) {
            print_interup("led_controler","Green LED turned off", RGB::NOTICE());
        }
        pub fn green_reset(&mut self) {
            print_interup("led_controler","Green LED reset", RGB::NOTICE());
        }
        pub fn blue_on(&mut self) {
            print_interup("led_controler","Blue LED turned on", RGB::NOTICE());
        }
        pub fn blue_off(&mut self) {
            print_interup("led_controler","Blue LED turned off", RGB::NOTICE());
        }
        pub fn blue_reset(&mut self) {
            print_interup("led_controler", "Blue LED reset", RGB::NOTICE());
        }
    }
}
