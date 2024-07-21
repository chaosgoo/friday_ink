use ch58x_hal::gpio::{AnyPin, Input, Level, Output, OutputDrive};
use ch58x_hal::{i2c, println};
use core::cell::RefCell;
use embedded_hal_02::blocking::i2c::{Read, Write, WriteRead};
use embedded_hal_1::i2c::{AddressMode, SevenBitAddress};

enum SDA<'a> {
    OutputMode(Output<'a, AnyPin>),
    InputMode(Input<'a, AnyPin>),
}

pub struct SoftwareI2C {
    sda: RefCell<SDA<'static>>,
    sda_pin: u8,
    scl: RefCell<Output<'static, AnyPin>>,
}

impl SoftwareI2C {
    pub fn new(sda: u8, scl: u8) -> Self {
        println!("SoftwareI2C @ scl:{}, sda:{}", scl, sda);
        unsafe {
            let _sda: SDA = SDA::OutputMode(Output::new(
                AnyPin::steal(sda),
                Level::Low,
                OutputDrive::_5mA,
            ));
            let scl = Output::new(AnyPin::steal(scl), Level::Low, OutputDrive::_5mA);

            SoftwareI2C {
                sda: RefCell::new(_sda),
                sda_pin: sda,
                scl: RefCell::new(scl),
            }
        }
    }

    fn set_scl_high(&self) {
        self.scl.borrow_mut().set_high()
    }

    fn set_scl_low(&self) {
        self.scl.borrow_mut().set_low()
    }

    fn set_sda_high(&self) {
        let mut sda = self.sda.borrow_mut();
        match &mut *sda {
            SDA::OutputMode(output) => {
                output.set_high();
            }
            SDA::InputMode(_) => unsafe {
                let mut output =
                    Output::new(AnyPin::steal(self.sda_pin), Level::High, OutputDrive::_5mA);
                output.set_high();
                *sda = SDA::OutputMode(output);
            },
        };
    }

    fn set_sda_low(&self) {
        let mut sda = self.sda.borrow_mut();
        match &mut *sda {
            SDA::OutputMode(output) => {
                output.set_low();
            }
            SDA::InputMode(_) => unsafe {
                let output =
                    Output::new(AnyPin::steal(self.sda_pin), Level::Low, OutputDrive::_5mA);
                *sda = SDA::OutputMode(output);
            },
        };
    }

    fn read_sda(&self) -> bool {
        let mut sda = self.sda.borrow_mut();
        match &mut *sda {
            SDA::OutputMode(output) => {
                output.set_low();
                let to_input =
                    unsafe { Input::new(AnyPin::steal(self.sda_pin), ch58x_hal::gpio::Pull::Up) };
                let res = to_input.is_high();
                *sda = SDA::InputMode(to_input);
                res
            }
            SDA::InputMode(input) => input.is_high(),
        }
    }

    fn delay() {
        ch58x_hal::delay_us(20);
    }

    fn start_condition(&mut self) {
        self.set_scl_high();
        self.set_sda_high();
        Self::delay();
        self.set_sda_low();
        Self::delay();
        self.set_scl_low();
        Self::delay();
    }

    fn stop_condition(&mut self) {
        self.set_sda_low();
        self.set_scl_high();
        Self::delay();
        self.set_sda_high();
        Self::delay();
    }

    fn write_bit(&self, bit: bool) {
        if bit {
            self.set_sda_high();
        } else {
            self.set_sda_low();
        }
        Self::delay();
        self.set_scl_high();
        Self::delay();
        self.set_scl_low();
        Self::delay();
    }

    fn read_bit(&self) -> bool {
        // self.set_sda_high();
        self.read_sda();
        self.set_scl_low();
        Self::delay();
        self.set_scl_high();
        Self::delay();
        let bit = self.read_sda();
        Self::delay();
        self.set_scl_low();
        Self::delay();
        bit
    }

    fn write_byte(&self, byte: u8) -> bool {
        for i in 0..8 {
            self.write_bit((byte & (1 << (7 - i))) != 0);
        }
        // Read ACK/NACK
        !self.read_bit()
    }

    fn read_byte(&self, ack: bool) -> u8 {
        let mut byte = 0;
        for i in 0..8 {
            if self.read_bit() {
                byte |= 1 << (7 - i);
            }
        }
        self.write_bit(!ack);
        byte
    }
}

impl Write for SoftwareI2C {
    type Error = i2c::Error;
    /// 写入数据
    ///  - addr 设备地址
    ///  - bytes 写入数据 = \[reg, data\]
    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.start_condition();
        if !self.write_byte(addr) {
            self.stop_condition();
            return Err(i2c::Error::Nack);
        }
        for &byte in bytes {
            if !self.write_byte(byte) {
                self.stop_condition();
                return Err(i2c::Error::Nack);
            }
        }
        self.stop_condition();
        Ok(())
    }
}

impl WriteReg for SoftwareI2C {
    type Error = i2c::Error;
    fn write_reg(&mut self, addr: u8, reg: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.start_condition();
        if !self.write_byte(addr) {
            self.stop_condition();
            return Err(i2c::Error::Nack);
        }
        if !self.write_byte(reg) {
            self.stop_condition();
            return Err(i2c::Error::Nack);
        }
        for &byte in bytes {
            if !self.write_byte(byte) {
                self.stop_condition();
                return Err(i2c::Error::Nack);
            }
        }
        self.stop_condition();
        Ok(())
    }
}

impl Read for SoftwareI2C {
    type Error = i2c::Error;
    /// 读取
    ///  - addr 设备地址
    ///  - buffer 读取到的内容
    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.start_condition();
        if !self.write_byte(addr) {
            self.stop_condition();
            return Err(i2c::Error::Nack);
        }
        let mut i = 0;
        let len = buffer.len();
        for byte in buffer.iter_mut() {
            *byte = self.read_byte(i != len - 1);
            i += 1;
        }
        self.stop_condition();
        Ok(())
    }
}

impl WriteRead for SoftwareI2C {
    type Error = i2c::Error;
    /// 写入然后读取
    ///  - addr 设备地址
    ///  - bytes 数据内容 = \[reg, data\]
    ///  - buffer 读取到的内容
    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.write(addr, bytes)?;
        self.read(addr | 1, buffer)
    }
}

pub trait WriteReg<A: AddressMode = SevenBitAddress> {
    /// Error type
    type Error;
    /// 写入数据
    ///  - addr 设备地址
    ///  - reg 寄存器地址
    ///  - bytes 数据内容
    fn write_reg(&mut self, addr: A, reg: u8, bytes: &[u8]) -> Result<(), Self::Error>;
}
