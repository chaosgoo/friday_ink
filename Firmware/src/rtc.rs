use core::cell::RefCell;

use crate::softwire::SoftwareI2C;
use crate::softwire::WriteReg;
use ch58x_hal::{
    i2c,
    prelude::{_embedded_hal_blocking_i2c_Write, _embedded_hal_blocking_i2c_WriteRead},
    println,
};

const PCF8563_ADDR: u8 = 0xA2;
const PCF8563_IS_RUNNING_FLAG: u8 = 0b00100000;
pub struct PCF8563 {
    i2c: RefCell<&'static mut SoftwareI2C>,
    addr: u8,
}

#[derive(Debug)]
pub struct Time {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub week: u8,
}

#[allow(unused)]
mod regs {
    pub const PCF8563_CLKOUTCONTROL: u8 = 0x0d; // Bit 7 PWR_MODE, bits 6:1 XG_OFFS_TC, bit 0 OTP_BNK_VLD
    pub const PCF8563_CONTROL_1: u8 = 0x0; //< Control and status register 1
    pub const PCF8563_CONTROL_2: u8 = 0x1; //< Control and status register 2
    pub const PCF8563_VL_SECONDS: u8 = 0x02; //< register address for VL_SECONDS
    pub const PCF8563_CLKOUT_MASK: u8 = 0x83; //< bitmask for SqwPinMode on CLKOUT pin
    pub const PCF8563_HOUR_ALARM: u8 = 0x0a; // 闹钟小时寄存器地址, 值取低6位, 取值范围 0~24
    pub const PCF8563_MINUTE_ALARM: u8 = 0x09; // 闹钟分钟寄存器地址, 值取低5位, 取值范围0~59
    pub const PCF8563_DAY_ALARM: u8 = 0x0b; // 闹钟天寄存器地址, 值取低5位, 取值范围0~59
    pub const PCF8563_WEEKDAY_ALARM: u8 = 0x0c; // 闹钟星期寄存器地址, 值取低5位, 取值范围0~59
}

impl PCF8563 {
    pub fn new(i2c: &'static mut SoftwareI2C) -> Self {
        Self {
            i2c: RefCell::new(i2c),
            addr: PCF8563_ADDR,
        }
    }

    pub fn start(&mut self) -> Result<(), i2c::Error> {
        println!("PCF8563 start");
        let mut buffer = [0u8; 1];
        self.read_byte(regs::PCF8563_CONTROL_1, &mut buffer)?;
        if (buffer[0] & PCF8563_IS_RUNNING_FLAG) == 0 {
            return Err(i2c::Error::Bus);
        }
        if buffer[0] & (1 << 5) != 0 {
            self.write_byte(regs::PCF8563_CONTROL_1, buffer[0] & !(1 << 5))
        } else {
            Ok(())
        }
    }

    pub fn set_time(&mut self, time: Time) -> Result<(), i2c::Error> {
        let mut buffer = [0u8; 7];
        buffer[0] = Self::bin_to_bcd(time.second);
        buffer[1] = Self::bin_to_bcd(time.minute);
        buffer[2] = Self::bin_to_bcd(time.hour);
        buffer[3] = Self::bin_to_bcd(time.day);
        buffer[4] = Self::bin_to_bcd(time.week);
        buffer[5] = Self::bin_to_bcd(time.month);
        buffer[6] = Self::bin_to_bcd(time.year);
        self.write_bytes(regs::PCF8563_VL_SECONDS, &mut buffer)
    }

    pub fn now(&self) -> Result<Time, i2c::Error> {
        println!("PCF8563 now()");
        let mut buffer = [0u8; 7];
        self.read_byte(regs::PCF8563_VL_SECONDS, &mut buffer)?;
        let second = Self::bcd_to_bin(buffer[0] & 0x7F); // 忽略最高位VL位
        let minute = Self::bcd_to_bin(buffer[1] & 0x7F);
        let hour = Self::bcd_to_bin(buffer[2] & 0x3F); // 忽略最高两位
        let day = Self::bcd_to_bin(buffer[3] & 0x3F); // 忽略最高位
        let week = Self::bcd_to_bin(buffer[4] & 0x07); // 只取最低3位
        let month = Self::bcd_to_bin(buffer[5] & 0x1F); // 忽略最高位
        let year = Self::bcd_to_bin(buffer[6]); // 读取年份并加上2000

        Ok(Time {
            year,
            month,
            day,
            hour,
            minute,
            second,
            week,
        })
    }

    pub fn check_alarm(&mut self) -> Result<bool, i2c::Error> {
        let mut alarm: u8;
        let mut buffer = [0u8; 1];
        self.read_byte(regs::PCF8563_CONTROL_2, &mut buffer)?;
        alarm = buffer[0];
        alarm = alarm & 8; // isolate the alarm flag bit
        return Ok(alarm == 8);
    }

    pub fn set_alarm(&mut self, week: u8, day: u8, hour: u8, minute: u8) -> Result<(), i2c::Error> {
        println!("set alarm");
        let mut buffer = [0u8; 4];
        buffer[0] = Self::bin_to_bcd(minute);
        buffer[1] = Self::bin_to_bcd(hour);
        buffer[2] = Self::bin_to_bcd(day) | 0x80;
        buffer[3] = Self::bin_to_bcd(week) | 0x80;
        self.write_bytes(regs::PCF8563_MINUTE_ALARM, &mut buffer)?;
        let mut ctlreg = [0u8];
        // self.read_byte(regs::PCF8563_CONTROL_2, &mut ctlreg)?;
        ctlreg[0] = 0x02 | 0x04;
        self.write_bytes(regs::PCF8563_CONTROL_2, &mut ctlreg)
    }

    pub fn clear_alarm(&mut self) -> Result<(), i2c::Error> {
        let mut buffer = [0u8; 1];
        self.read_byte(regs::PCF8563_CONTROL_2, &mut buffer)?;
        buffer[0] = buffer[0] - 0b00001000;
        self.write_bytes(regs::PCF8563_CONTROL_2, &mut buffer)
    }

    pub fn is_running(&mut self) -> Result<bool, i2c::Error> {
        let mut buf = [0u8; 1];
        self.read_byte(regs::PCF8563_CONTROL_1, &mut buf)?;
        return Ok(((buf[0] >> 5) & 1) != 0);
    }

    pub fn lost_power(&mut self) -> Result<bool, i2c::Error> {
        let mut buf = [0u8; 1];
        self.read_byte(regs::PCF8563_VL_SECONDS, &mut buf)?;
        return Ok((buf[0] >> 7) != 0);
    }

    pub fn stop(&mut self) -> Result<(), i2c::Error> {
        let mut buf = [0u8; 1];
        self.read_byte(regs::PCF8563_CONTROL_1, &mut buf)?;
        if (buf[0] & PCF8563_IS_RUNNING_FLAG) == 0 {
            return Err(i2c::Error::Bus);
        }
        if buf[0] & (1 << 5) != 0 {
            self.write_byte(regs::PCF8563_CONTROL_1, buf[0] | (1 << 5))
        } else {
            Ok(())
        }
    }

    fn bcd_to_bin(value: u8) -> u8 {
        ((value / 16) * 10) + (value % 16)
    }
    fn bin_to_bcd(value: u8) -> u8 {
        value + 6 * (value / 10)
    }

    fn dec_to_bcd(value: u8) -> u8 {
        value / 10 * 16 + value % 10
    }

    fn read_byte(&self, reg_addr: u8, buf: &mut [u8]) -> Result<(), i2c::Error> {
        println!("PCF8563 read_byte {:x?} from {:x}", buf, reg_addr);
        let mut i2c = self.i2c.borrow_mut();
        i2c.write_read(self.addr, &[reg_addr], buf)
    }

    fn write_bytes(&mut self, reg_addr: u8, bytes: &mut [u8]) -> Result<(), i2c::Error> {
        println!("PCF8563 write {:x?} into reg_addr {:x}", bytes, reg_addr);
        let mut i2c = self.i2c.borrow_mut();
        i2c.write_reg(self.addr, reg_addr, &bytes)
    }

    fn write_byte(&mut self, reg_addr: u8, byte: u8) -> Result<(), i2c::Error> {
        println!("PCF8563 write {:x} into reg_addr {:x}", byte, reg_addr);
        let mut i2c = self.i2c.borrow_mut();
        i2c.write(reg_addr, &[byte])
    }
}

use core::cell::UnsafeCell;

struct Singleton {
    inner: UnsafeCell<Option<&'static mut PCF8563>>,
}

unsafe impl Sync for Singleton {}

static PERIPHERALS: Singleton = Singleton {
    inner: UnsafeCell::new(None),
};

pub fn set_default_rtc(rtc: &'static mut PCF8563) {
    let peripherals = unsafe { &mut *PERIPHERALS.inner.get() };
    if peripherals.is_none() {
        *peripherals = Some(rtc);
    } else {
        panic!("RTC has already been set");
    }
}

pub fn take() -> &'static mut PCF8563 {
    let peripherals = unsafe { &mut *PERIPHERALS.inner.get() };
    peripherals.as_mut().expect("RTC is not set")
}
