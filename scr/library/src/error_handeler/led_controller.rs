// linter could be broken if the feature les is not enabled

use super::{LedNumber, LedOption, RGB};
#[cfg(feature = "GPIO")]
pub mod led {
    use crate::error_handeler::{LedNumber, RGB};
    use rppal;

    #[derive(Debug)]
    pub enum LedError{
        #[allow(non_camel_case_types)]
        rrpalError(String),
    }
    impl std::fmt::Display for LedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LedError::rrpalError(msg) => write!(f, "{}", msg),
            }
        }   
    }
    impl std::error::Error for LedError{}
    impl From<rppal::i2c::Error> for LedError {
        fn from(err: rppal::i2c::Error) -> Self {
            LedError::rrpalError(format!("RPPAL I2C Error: {}", err))
        }
    }
    impl From<rppal::gpio::Error> for LedError {
        fn from(err: rppal::gpio::Error) -> Self {
            LedError::rrpalError(format!("RPPAL GPIO Error: {}", err))
        }
    }

    const PCA9685: u16 = 0x40;
    const MODE1: u8 = 0x00;
    const LED_OFSET: u8 = 12;
    const LED1: u8 = 0x06;
    const LED_SUB1: u8 = LED1+4;
    const LED_SUB2: u8 = LED1+8;
    const PRE_SCALE: u8 = 0xFE;
    const MAX_LEVEL: u8 = 16;
    const GAMMA: f32 = 2.2;
    
    /// LED controller for the PCA9685 chip.
    /// This struct handles the I2C communication with the PCA9685 chip and controls the colors and brightness of 5 LEDs.
    /// the fist pin on the PCA9685 is for the red color, the second for the green color and the third for the blue color, that repeats for all 5 leds.
    /// the PCA9685 self is connected to the rasbarypi is this way:
    /// pin 1 for power
    /// pin 6 or 9 for ground
    /// pin 3 for SDA and pin 5 for SCL
    /// pin 7 for the OF pin, this pin is used to turn the PCA9685 on and off
    pub struct LedController {
        pin: Option<rppal::gpio::OutputPin>,
        control: Option<rppal::i2c::I2c>,
        color: [RGB;5],
        brightness: [u8;5],
    }

    impl Drop for LedController{
        fn drop(&mut self) {
            if let None = self.pin {
                return;
            }
            if let Err( err) = self.set_color_all([RGB::BLACK();5]){
                eprint!("Error while drpping the Led controller at set color: {}", err);
            }
            for i in 0..self.color.len(){
                if let Err(err) =self.red_reset(i.into()){
                    eprint!("Error while drpping the Led controller at red reset: {}", err);
                }
                if let Err(err) =self.green_reset(i.into()){
                    eprint!("Error while drpping the Led controller at green reset: {}", err);
                }
                if let Err(err) =self.blue_reset(i.into()){
                    eprint!("Error while drpping the Led controller at blue reset: {}", err);
                }
            }
        }
    }

    impl LedController {
        pub fn dummy() -> Self{
            Self {
                pin: None,
                control: None,
                color: [RGB::BLACK();5],
                brightness: [0;5],
            }
        }
        pub fn new(color: [RGB;5], led_level: [u8;5])-> Result<Self, LedError> { // TODO: add a chrck to see if the hard ware is present and maby return a dummy led controller if not
            let pin = Some(rppal::gpio::Gpio::new()?.get(4)?.into_output_low());
            let mut i2c = Some(rppal::i2c::I2c::new()?);
            let i2c_ref = i2c.as_mut().expect("I2C not initialized correctly");
            i2c_ref.set_slave_address(PCA9685)?;
            i2c_ref.smbus_write_byte(MODE1, 0b0011_0001)?;
            i2c_ref.smbus_write_byte(PRE_SCALE, 101)?;
            i2c_ref.smbus_write_byte(MODE1, 0b0010_0001)?;
            let mut all_values = [0; 60];
            for i in 0..color.len() {
                let values = map_values(color[i], led_level[i], [0; 6]);
                all_values[i * 12..(i + 1) * 12].copy_from_slice(&values);
            }
            i2c_ref.block_write(LED1, &all_values)?;
            Ok(Self { pin, color, brightness: led_level, control: i2c })
        }
        pub fn off(&mut self){
            if let None = self.pin {
                return;
            }
            self.pin.as_mut().expect("the led controller was not initialize correctly but still tride to acses the gpio pins").set_high(); //////////
        }
        pub fn on(&mut self){
            if let None = self.pin {
                return;
            }
            self.pin.as_mut().expect("the led controller was not initialize correctly but still tride to acses the gpio pins").set_low();
        }
        pub fn set_color(&mut self, color: RGB, led_number: LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let base_addr = LED1 + (led_number * LED_OFSET);
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let values: [u8; 6] = [
                cont_ref.smbus_read_byte(base_addr + 1)?,
                cont_ref.smbus_read_byte(base_addr + 3)?,
                cont_ref.smbus_read_byte(base_addr + 5)?,
                cont_ref.smbus_read_byte(base_addr + 7)?,
                cont_ref.smbus_read_byte(base_addr + 9)?,
                cont_ref.smbus_read_byte(base_addr + 11)?,
            ];
            cont_ref.block_write(base_addr, &map_values(color, self.brightness[led_number as usize], values))?;
            self.color[led_number as usize] = color;
            Ok(())
        }

        pub fn set_brightness(&mut self, level: u8, led_number: LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let base_addr = LED1 + (led_number * LED_OFSET);
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let values: [u8; 6] = [
            cont_ref.smbus_read_byte(base_addr + 1)?,
            cont_ref.smbus_read_byte(base_addr + 3)?,
            cont_ref.smbus_read_byte(base_addr + 5)?,
            cont_ref.smbus_read_byte(base_addr + 7)?,
            cont_ref.smbus_read_byte(base_addr + 9)?,
            cont_ref.smbus_read_byte(base_addr + 11)?,
            ];
            cont_ref.block_write(base_addr, &map_values(self.color[led_number as usize], level, values))?;                                                   
            self.brightness[led_number as usize] = level;
            Ok(())
        }

        pub fn set_color_all(&mut self, color: [RGB; 5]) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for (i, &col) in color.iter().enumerate() {
            self.set_color(col, i.into())?;
            }
            Ok(())
        }
        pub fn set_brightness_all(&mut self, level: [u8; 5]) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for (i, &lvl) in level.iter().enumerate() {
            self.set_brightness(lvl, i.into())?;
            }
            Ok(())
        }

        pub fn red_on(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED1 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED1 + 1+ (led_number*LED_OFSET), value | 0x10)?;
            value = cont_ref.smbus_read_byte(LED1 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED1 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn red_off(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED1 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED1 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = cont_ref.smbus_read_byte(LED1 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED1 + 3+ (led_number*LED_OFSET), value | 0x10)?;
            Ok(())
        }
        pub fn red_reset(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED1 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED1 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = cont_ref.smbus_read_byte(LED1 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED1 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn green_on(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED_SUB1 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB1 + 1+ (led_number*LED_OFSET), value | 0x10)?;
            value = cont_ref.smbus_read_byte(LED_SUB1 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB1 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn green_off(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED_SUB1 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB1 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = cont_ref.smbus_read_byte(LED_SUB1 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB1 + 3+ (led_number*LED_OFSET), value | 0x10)?;
            Ok(())
        }
        pub fn green_reset(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED_SUB1 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB1 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = cont_ref.smbus_read_byte(LED_SUB1 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB1 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn blue_on(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED_SUB2 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB2 + 1+ (led_number*LED_OFSET), value | 0x10)?;
            value = cont_ref.smbus_read_byte(LED_SUB2 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB2 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn blue_off(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED_SUB2 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB2 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = cont_ref.smbus_read_byte(LED_SUB2 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB2 + 3+ (led_number*LED_OFSET), value | 0x10)?;
            Ok(())
        }
        pub fn blue_reset(&mut self, led_number:LedNumber) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            check_index(self.color.len() as u8, led_number as u8)?;
            let cont_ref = self.control.as_ref().expect("the led controller was not initialize correctly but still tride to acses the gpio pins");
            let mut value = cont_ref.smbus_read_byte(LED_SUB2 + 1+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB2 + 1+ (led_number*LED_OFSET), value & 0x0F)?;
            value = cont_ref.smbus_read_byte(LED_SUB2 + 3+ (led_number*LED_OFSET))?;
            cont_ref.smbus_write_byte(LED_SUB2 + 3+ (led_number*LED_OFSET), value & 0x0F)?;
            Ok(())
        }
        pub fn red_reset_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.red_reset(i.into())?;
            }
            Ok(())
        }
        pub fn green_reset_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.green_reset(i.into())?;
            }
            Ok(())
        }
        pub fn blue_reset_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.blue_reset(i.into())?;
            }
            Ok(())
        }
        pub fn red_off_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.red_off(i.into())?;
            }
            Ok(())
        }
        pub fn green_off_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.green_off(i.into())?;
            }
            Ok(())
        }
        pub fn blue_off_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.blue_off(i.into())?;
            }
            Ok(())
        }
        pub fn red_on_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.red_on(i.into())?;
            }
            Ok(())
        }
        pub fn green_on_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.green_on(i.into())?;
            }
            Ok(())
        }
        pub fn blue_on_all(&mut self) -> Result<(), LedError> {
            if let None = self.pin {
                return Ok(());
            }
            for i in 0..self.color.len() {
                self.blue_on(i.into())?;
            }
            Ok(())
        }
    }

    fn check_index(max:u8, index:u8) -> Result<(),LedError>{
        if index >= max as u8 {
            return Err(LedError::rrpalError("Index out of bounds".to_string()));
        } 
        Ok(())
    }
    fn map_values(color: RGB, level: u8, existingvalues: [u8; 6]) -> [u8; 12] {
        let brightness = level.clamp(0, MAX_LEVEL);
        if brightness == 0 {
            return [0; 12];
        }
        let brightness_factor = (brightness as f32 / MAX_LEVEL as f32).powf(GAMMA);
        let (r,g,b) =color.to_tuple();
        let mut values = [0 as u16; 3];
        values[0] = (((r as f32 * 4095.0 / 255.0) as f32)* brightness_factor) as u16;
        values[1] = (((g as f32 * 4095.0 / 255.0) as f32)* brightness_factor) as u16;
        values[2] = (((b as f32 * 4095.0 / 255.0) as f32)* brightness_factor) as u16;
        let mut result= [0; 12];
        for (i, value) in values.iter().enumerate(){
            result[i * 4] = 0;
            result[i * 4 + 1] = 0 | (existingvalues[i * 2] & 0x10);
            result[i * 4 + 2] = (value & 0xFF) as u8;
            result[i * 4 + 3] = (value >> 8) as u8 | (existingvalues[i*2 +1] & 0x10);
        }
        result
    }
}

#[cfg(not(feature = "GPIO"))]
pub mod led {
    use crate::error_handeler::{RGB, LedNumber};

    #[derive(Debug)]
   pub enum LedError{
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        rrpalError(String),
    }
    impl std::fmt::Display for LedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LedError::rrpalError(msg) => write!(f, "{}", msg),
            }
        }   
    }
    impl std::error::Error for LedError{}

    /// This module is a placeholder for the LED controller when GPIO is not enabled.
    /// It provides a simple interface for controlling LED colors and brightness without actual hardware interaction.
    pub struct LedController {
        color: [RGB;5],
        brightness: [u8;5],
    }

    impl LedController {
        pub fn dummy() -> Self{
            Self {
                color: [RGB::BLACK();5],
                brightness: [0;5],
            }
        }

        pub fn new(color: [RGB; 5], led_level: [u8; 5]) -> Result<Self, LedError> {
            Ok(Self { color, brightness: led_level })
        }
        pub fn off(&mut self) {
            println!("led_controler LED turned off");
        }
        pub fn on(&mut self) {
            println!("led_controler LED turned on");
        }
        pub fn set_color(&mut self, color: RGB, led_number: LedNumber)-> Result<(), LedError> {
            self.color[led_number as usize] = color;
            println!("led_controler {} color set to {:?}", led_number, color);
            Ok(())
        }

        pub fn set_brightness(&mut self, level: u8, led_number: LedNumber)-> Result<(), LedError> {
            self.brightness[led_number as usize] = level;
            println!("led_controler {} brightness set to {}", led_number, level);
            Ok(())
        }
        pub fn set_color_all(&mut self, color: [RGB; 5]) -> Result<(), LedError>{
            self.color = color;
            println!("led_controler All LED colors set");
            Ok(())
        }

        pub fn set_brightness_all(&mut self, level: [u8; 5]) -> Result<(), LedError>{
            self.brightness = level;
            println!("led_controler All LED brightness levels set");
            Ok(())
        }
        
        pub fn red_on(&mut self, _led_number:LedNumber) -> Result<(), LedError>{
            println!("led_controler Red LED turned on");
            Ok(())
        }
        pub fn red_off(&mut self, _led_number:LedNumber)-> Result<(), LedError> {
            println!("led_controler Red LED turned off");
            Ok(())
        }
        pub fn red_reset(&mut self, _led_number:LedNumber) -> Result<(), LedError>{
            println!("led_controler Red LED reset");
            Ok(())
        }
        pub fn green_on(&mut self, _led_number:LedNumber) -> Result<(), LedError>{
            println!("led_controler Green LED turned on");
            Ok(())
        }
        pub fn green_off(&mut self, _led_number:LedNumber)-> Result<(), LedError> {
            println!("led_controler Green LED turned off");
            Ok(())
        }
        pub fn green_reset(&mut self, _led_number:LedNumber) -> Result<(), LedError>{
            println!("led_controler Green LED reset");
            Ok(())
        }
        pub fn blue_on(&mut self, _led_number:LedNumber) -> Result<(), LedError>{
            println!("led_controler Blue LED turned on");
            Ok(())
        }
        pub fn blue_off(&mut self, _led_number:LedNumber) -> Result<(), LedError>{
            println!("led_controler Blue LED turned off");
            Ok(())
        }
        pub fn blue_reset(&mut self, _led_number:LedNumber) -> Result<(), LedError>{
            println!("led_controler Blue LED reset");
            Ok(())
        }

        pub fn red_reset_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Red LEDs reset");
            Ok(())
        }
        pub fn green_reset_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Green LEDs reset");
            Ok(())
        }
        pub fn blue_reset_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Blue LEDs reset");
            Ok(())
        }
        pub fn red_off_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Red LEDs turned off");
            Ok(())
        }
        pub fn green_off_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Green LEDs turned off");
            Ok(())
        }
        pub fn blue_off_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Blue LEDs turned off");
            Ok(())
        }
        pub fn red_on_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Red LEDs turned on");
            Ok(())
        }
        pub fn green_on_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Green LEDs turned on");
            Ok(())
        }
        pub fn blue_on_all(&mut self) -> Result<(), LedError>{
            println!("led_controler All Blue LEDs turned on");
            Ok(())
        }
    }

}

pub fn change_led_color(led_controller: &mut led::LedController, color: RGB, led_number: LedNumber) -> Result<(), led::LedError> {
    if led_number == LedNumber::ALL{
        led_controller.set_color_all([color; 5])?;
        return Ok(());
    }
    led_controller.set_color(color, led_number)?;
    Ok(())
}

pub fn change_led_brightness(led_controller: &mut led::LedController, level: u8, led_number: LedNumber) -> Result<(), led::LedError> {
    if led_number == LedNumber::ALL {
        led_controller.set_brightness_all([level; 5])?;
        return Ok(());
    }
    led_controller.set_brightness(level, led_number)?;
    Ok(())
}

pub fn reset_color_led(led_controller: &mut led::LedController, color: LedOption ,led_number: LedNumber) -> Result<(), led::LedError> {
    if led_number == LedNumber::ALL {
        match color {
            LedOption::Red => led_controller.red_reset_all()?,
            LedOption::Green => led_controller.green_reset_all()?,
            LedOption::Blue => led_controller.blue_reset_all()?,
            LedOption::All => {
                led_controller.red_reset_all()?;
                led_controller.green_reset_all()?;
                led_controller.blue_reset_all()?;
            }
        }
       return Ok(());
    }
    match color {
        LedOption::Red => led_controller.red_reset(led_number)?,
        LedOption::Green => led_controller.green_reset(led_number)?,
        LedOption::Blue => led_controller.blue_reset(led_number)?,
        LedOption::All => {
            led_controller.red_reset(led_number)?;
            led_controller.green_reset(led_number)?;
            led_controller.blue_reset(led_number)?;
        }
    }
    Ok(())
}

pub fn color_off(led_controller: &mut led::LedController, color: LedOption, led_number: LedNumber) -> Result<(), led::LedError> {
    if led_number == LedNumber::ALL {
        match color {
            LedOption::Red => led_controller.red_off_all()?,
            LedOption::Green => led_controller.green_off_all()?,
            LedOption::Blue => led_controller.blue_off_all()?,
            LedOption::All => {
                led_controller.red_off_all()?;
                led_controller.green_off_all()?;
                led_controller.blue_off_all()?;
            }
        }
        return Ok(());
    }
    match color {
        LedOption::Red => led_controller.red_off(led_number)?,
        LedOption::Green => led_controller.green_off(led_number)?,
        LedOption::Blue => led_controller.blue_off(led_number)?,
        LedOption::All => {
            led_controller.red_off(led_number)?;
            led_controller.green_off(led_number)?;
            led_controller.blue_off(led_number)?;
        }
    }
    Ok(())
}
pub fn color_on(led_controller: &mut led::LedController, color: LedOption, led_number: LedNumber) -> Result<(), led::LedError> {
    if led_number == LedNumber::ALL {
        match color {
            LedOption::Red => led_controller.red_on_all()?,
            LedOption::Green => led_controller.green_on_all()?,
            LedOption::Blue => led_controller.blue_on_all()?,
            LedOption::All => {
                led_controller.red_on_all()?;
                led_controller.green_on_all()?;
                led_controller.blue_on_all()?;
            }
        }
        return Ok(());
    }
    match color {
        LedOption::Red => led_controller.red_on(led_number)?,
        LedOption::Green => led_controller.green_on(led_number)?,
        LedOption::Blue => led_controller.blue_on(led_number)?,
        LedOption::All => {
            led_controller.red_on(led_number)?;
            led_controller.green_on(led_number)?;
            led_controller.blue_on(led_number)?;
        }
    }
    Ok(())
}
