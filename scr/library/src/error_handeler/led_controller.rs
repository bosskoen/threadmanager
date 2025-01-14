use super::RGB;

#[cfg(feature = "GPIO")]
mod led {

    use super::*;
    use rppal;

    const PCA9685: u8 = 0x40;
    const MODE1: u8 = 0x00;
    const LED_OFSET: u8 = 12;
    const LED1: u8 = 0x06;
    const LED_SUB1: u8 = LED1+4;
    const LED_SUB2: u8 = LED1+8;
    const PRE_SCALE: u8 = 0xFE;
    const MAX_LEVEL: u8 = 16;
    const GAMMA: f32 = 2.2;
    

    pub struct LedController {
        pin: rppal::gpio::OutputPin,
        control: rppal::i2c::I2c,
        color: [RGB;5],
        brightness: [u8;5],
    }

    impl Drop for LedController{
        fn drop(&mut self) {
            self.set_color_all([RGB::BLACK();5]);
            for i in 0..self.color.len(){
                self.red_reset(i);
                self.green_reset(i);
                self.blue_reset(i);
            }
        }
    }

    impl LedController {
        pub fn new(color: [RGB;5], led_level: [u8;5])-> Result<Self, rppal::gpio::Error> {
            let pin = rppal::gpio::Gpio::new()?.get(4)?.into_output_low();
            let i2c = rppal::i2c::I2c::new()?;
            i2c.set_slave_address(PCA9685)?;
            i2c.smbus_write_byte(MODE1, 0b0011_0001)?;
            i2c.smbus_write_byte(PRE_SCALE, 101)?;
            i2c.smbus_write_byte(MODE1, 0b0010_0001)?;
            let mut all_values = [0; 60];
            for i in 0..color.len() {
                let values = map_values(color[i], led_level[i], [0; 6]);
                all_values[i * 12..(i + 1) * 12].copy_from_slice(&values);
            }
            i2c.block_write(LED1, &all_values)?;
            Ok(Self { pin, color, brightness: led_level, control: i2c })
        }
        pub fn off(&mut self){
            self.pin.set_high();
        }
        pub fn on(&mut self){
            self.pin.set_low();
        }
        pub fn set_color(&mut self, color: RGB, led_number: u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let base_addr = LED1 + (led_number * LED_OFSET);
            let mut values: [u8; 6] = [
            self.control.smbus_read_byte(base_addr + 1)?,
            self.control.smbus_read_byte(base_addr + 3)?,
            self.control.smbus_read_byte(base_addr + 5)?,
            self.control.smbus_read_byte(base_addr + 7)?,
            self.control.smbus_read_byte(base_addr + 9)?,
            self.control.smbus_read_byte(base_addr + 11)?,
            ];
            self.control.block_write(base_addr, &map_values(color, self.brightness[led_number], values))?;
            self.color[led_number] = color;
            Ok(())
        }

        pub fn set_brightness(&mut self, level: u8, led_number: u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let base_addr = LED1 + (led_number * LED_OFSET);
            let mut values: [u8; 6] = [
            self.control.smbus_read_byte(base_addr + 1)?,
            self.control.smbus_read_byte(base_addr + 3)?,
            self.control.smbus_read_byte(base_addr + 5)?,
            self.control.smbus_read_byte(base_addr + 7)?,
            self.control.smbus_read_byte(base_addr + 9)?,
            self.control.smbus_read_byte(base_addr + 11)?,
            ];
            self.control.block_write(base_addr, &map_values(self.color[led_number], level, values))?;
            self.brightness[led_number] = level;
            Ok(())
        }

        pub fn set_color_all(&mut self, color: [RGB; 5]) -> Result<(), rppal::i2c::Error> {
            for (i, &col) in color.iter().enumerate() {
            self.set_color(col, i as u8)?;
            }
            Ok(())
        }

        pub fn set_brightness_all(&mut self, level: [u8; 5]) -> Result<(), rppal::i2c::Error> {
            for (i, &lvl) in level.iter().enumerate() {
            self.set_brightness(lvl, i as u8)?;
            }
            Ok(())
        }

        pub fn red_on(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED1 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED1 + 1+ (led_number*LED_OFSET), value | 0x10)?;
            value = self.control.smbus_read_byte(LED1 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED1 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn red_off(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED1 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED1 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = self.control.smbus_read_byte(LED1 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED1 + 3+ (led_number*LED_OFSET), value | 0x10)?;
            Ok(())
        }
        pub fn red_reset(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED1 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED1 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = self.control.smbus_read_byte(LED1 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED1 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn green_on(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED_SUB1 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB1 + 1+ (led_number*LED_OFSET), value | 0x10)?;
            value = self.control.smbus_read_byte(LED_SUB1 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB1 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn green_off(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED_SUB1 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB1 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = self.control.smbus_read_byte(LED_SUB1 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB1 + 3+ (led_number*LED_OFSET), value | 0x10)?;
            Ok(())
        }
        pub fn green_reset(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED_SUB1 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB1 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = self.control.smbus_read_byte(LED_SUB1 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB1 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn blue_on(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED_SUB2 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB2 + 1+ (led_number*LED_OFSET), value | 0x10)?;
            value = self.control.smbus_read_byte(LED_SUB2 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB2 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn blue_off(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED_SUB2 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB2 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = self.control.smbus_read_byte(LED_SUB2 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB2 + 3+ (led_number*LED_OFSET), value | 0x10)?;
            Ok(())
        }
        pub fn blue_reset(&mut self, led_number:u8) -> Result<(), rppal::i2c::Error> {
            check_index(self.color.len(), led_number)?;
            let mut value = self.control.smbus_read_byte(LED_SUB2 + 1+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB2 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = self.control.smbus_read_byte(LED_SUB2 + 3+ (led_number*LED_OFSET))?;
            self.control.smbus_write_byte(LED_SUB2 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
    }

    fn check_index(max:u8, index:u8) -> Result<(),rppal::i2c::Error>{
        if index >= max as u8 {
            return Err(rppal::i2c::Error::InvalidInput);
        } 
        Ok(())
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
        color: [RGB;5],
        brightness: [u8;5],
    }

    impl LedController {
        pub fn new(color: [RGB; 5], led_level: [u8; 5]) -> Self {
            Self { color, brightness: led_level }
        }
        pub fn off(&mut self) {
            print_interup("led_controler","LED turned off", RGB::NOTICE());
        }
        pub fn on(&mut self) {
            print_interup("led_controler","LED turned on", RGB::NOTICE());
        }
        pub fn set_color(&mut self, color: RGB, led_number: u8) {
            self.color[led_number] = color;
            print_interup("led_controler", &format!("LED {} color set to {:?}", led_number, color), RGB::NOTICE());
        }

        pub fn set_brightness(&mut self, level: u8, led_number: u8) {
            self.brightness[led_number] = level;
            print_interup("led_controler", &format!("LED {} brightness set to {}", led_number, level), RGB::NOTICE());
        }

        pub fn set_color_all(&mut self, color: [RGB; 5]) {
            self.color = color;
            print_interup("led_controler", "All LED colors set", RGB::NOTICE());
        }

        pub fn set_brightness_all(&mut self, level: [u8; 5]) {
            self.brightness = level;
            print_interup("led_controler", "All LED brightness levels set", RGB::NOTICE());
        }
        
        pub fn red_on(&mut self, led_number:u8) {
            print_interup("led_controler","Red LED turned on", RGB::NOTICE());
        }
        pub fn red_off(&mut self, led_number:u8) {
            print_interup("led_controler","Red LED turned off", RGB::NOTICE());
        }
        pub fn red_reset(&mut self, led_number:u8) {
            print_interup("led_controler","Red LED reset", RGB::NOTICE());
        }
        pub fn green_on(&mut self, led_number:u8) {
            print_interup("led_controler","Green LED turned on", RGB::NOTICE());
        }
        pub fn green_off(&mut self, led_number:u8) {
            print_interup("led_controler","Green LED turned off", RGB::NOTICE());
        }
        pub fn green_reset(&mut self, led_number:u8) {
            print_interup("led_controler","Green LED reset", RGB::NOTICE());
        }
        pub fn blue_on(&mut self, led_number:u8) {
            print_interup("led_controler","Blue LED turned on", RGB::NOTICE());
        }
        pub fn blue_off(&mut self, led_number:u8) {
            print_interup("led_controler","Blue LED turned off", RGB::NOTICE());
        }
        pub fn blue_reset(&mut self, led_number:u8) {
            print_interup("led_controler", "Blue LED reset", RGB::NOTICE());
        }
    }
}
