extern crate u8g2_rs;
use core::cell::RefCell;
use core::ffi::{c_char, c_void};
use core::fmt::Write;
use core::slice;

use crate::assets;
use crate::rtc::Time;
use ch58x_hal::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull};
use ch58x_hal::println;
use ch58x_hal::spi::{BitOrder, Spi};
use ch58x_hal::{peripherals, prelude::*};
use embedded_hal_02::spi::Polarity;
use qingke::riscv;
use u8g2_rs::*;

extern crate core;

use core::fmt::{self};

// 在u8g2中,这些产量是宏定义的,没有被bindgen转换,所以需要手动补充一下
pub const U8X8_MSG_GPIO_CS: u32 = 73;
pub const U8X8_MSG_GPIO_DC: u32 = 74;
pub const U8X8_MSG_GPIO_RESET: u32 = 75;

pub enum DriverIC {
    SSD1607,
    SSD1681,
}

pub struct StringWriter {
    buffer: [u8; 32],
    pos: usize,
}

impl StringWriter {
    pub fn new() -> StringWriter {
        StringWriter {
            buffer: [0; 32],
            pos: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.pos]).unwrap()
    }
}

impl Write for StringWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let len = bytes.len();

        if self.pos + len > self.buffer.len() {
            return Err(fmt::Error);
        }

        self.buffer[self.pos..self.pos + len].copy_from_slice(bytes);
        self.pos += len;

        Ok(())
    }
}

pub unsafe extern "C" fn u8x8_byte_ch582f_hw_spi(
    u8x8: *mut u8x8_t,
    msg: u8,
    arg_int: u8,
    arg_ptr: *mut c_void,
) -> u8 {
    if u8x8.is_null() {
        println!("u8x8.is_null()");
        return 0;
    }

    let display_ptr = (*u8x8).user_ptr as *mut Display;
    if display_ptr.is_null() {
        println!("1display_ptr.is_null()");
        return 0;
    }
    let display = &mut *display_ptr;

    match msg as u32 {
        // 根据你的需要处理不同的消息
        U8X8_MSG_BYTE_SEND => {
            // 处理消息 0x00
            if arg_ptr.is_null() {
                // 处理空指针的情况
                return 0;
            }
            let data = slice::from_raw_parts(arg_ptr as *const u8, arg_int.into());
            let _ = display.spi_host.blocking_write(data);
        }
        U8X8_MSG_BYTE_INIT => {}
        U8X8_MSG_BYTE_SET_DC => {
            if arg_int == 0 {
                display.dc.set_low();
            } else {
                display.dc.set_high();
            }
        }
        U8X8_MSG_BYTE_START_TRANSFER => {
            display.cs.set_low();
        }
        U8X8_MSG_BYTE_END_TRANSFER => {
            display.cs.set_high();
        }
        _ => {}
    }
    0
}

pub unsafe extern "C" fn u8x8_gpio_and_delay_ch582f(
    u8x8: *mut u8x8_t,
    msg: u8,
    arg_int: u8,
    _arg_ptr: *mut c_void,
) -> u8 {
    if u8x8.is_null() {
        println!("u8x8.is_null()");
        return 0;
    }

    let display_ptr = (*u8x8).user_ptr as *mut Display;
    if display_ptr.is_null() {
        println!("2display_ptr.is_null()");
        return 0;
    }
    let display = &mut *display_ptr;

    match msg as u32 {
        // 根据你的需要处理不同的消息
        U8X8_MSG_GPIO_AND_DELAY_INIT => {}
        U8X8_MSG_DELAY_NANO => {
            for _ in 0..=20 {
                riscv::asm::nop();
            }
        }
        U8X8_MSG_DELAY_100NANO => {
            for _ in 0..=2000 {
                riscv::asm::nop();
            }
        }
        U8X8_MSG_DELAY_10MICRO => {
            ch58x_hal::delay_ms(1);
        }
        U8X8_MSG_DELAY_MILLI => {
            // TODO 理论上说1681需要借助Busy脚去检测状态
            let busy = Input::new(peripherals::PA4::steal().degrade(), Pull::Up);
            let mut max_delay: u16 = arg_int.into();
            let busy_level = match display.driver_ic {
                DriverIC::SSD1607 => Level::High,
                DriverIC::SSD1681 => Level::High,
            };
            while busy.get_level() == busy_level {
                println!("wait for busy low");
                if max_delay > 0 {
                    ch58x_hal::delay_ms(max_delay);
                    max_delay -= 1;
                } else {
                    break;
                }
            }
            // ch58x_hal::delay_ms(arg_int.into());
        }
        U8X8_MSG_GPIO_CS => {
            if arg_int == 0 {
                display.cs.set_low();
            } else {
                display.cs.set_high();
            }
        }
        U8X8_MSG_GPIO_DC => {
            if arg_int == 0 {
                display.dc.set_low();
            } else {
                display.dc.set_high();
            }
        }
        U8X8_MSG_GPIO_RESET => {
            if arg_int == 0 {
                display.res.set_low();
            } else {
                display.res.set_high();
            }
        }
        _ => {}
    }
    0
}

pub struct Display<'d> {
    pub u8g2: RefCell<u8g2_t>,
    pub spi_host: Spi<'d, peripherals::SPI0>,
    pub dc: Output<'d, AnyPin>,
    pub res: Output<'d, AnyPin>,
    pub cs: Output<'d, AnyPin>,
    pub en: Output<'d, AnyPin>,
    driver_ic: DriverIC,
}

impl<'d> Display<'d> {
    pub fn new(driver_ic: DriverIC, byte_cb: u8x8_msg_cb, gpio_and_delay_cb: u8x8_msg_cb) -> Self {
        let mut u8g2: u8g2_t = unsafe { core::mem::zeroed() };
        unsafe {
            let rotation = &u8g2_rs::u8g2_cb_r3;
            match driver_ic {
                DriverIC::SSD1607 => {
                    u8g2_Setup_ssd1607_gd_200x200_f(
                        &mut u8g2,
                        rotation,
                        byte_cb,
                        gpio_and_delay_cb,
                    );
                }
                DriverIC::SSD1681 => {
                    u8g2_Setup_ssd1681_zjy_200x200_f(
                        &mut u8g2,
                        rotation,
                        byte_cb,
                        gpio_and_delay_cb,
                    );
                }
            }
        }

        let mut spi_config = ch58x_hal::spi::Config::default();
        spi_config.frequency = 20.MHz();
        spi_config.bit_order = BitOrder::MsbFirst;
        spi_config.clock_polarity = Polarity::IdleLow;
        let spi0 = unsafe { peripherals::SPI0::steal() };
        let sck = unsafe { peripherals::PB13::steal() };
        let mosi = unsafe { peripherals::PB14::steal() };
        let spi_host: Spi<peripherals::SPI0> = Spi::new_txonly(spi0, sck, mosi, spi_config);
        let dc = Output::new(
            unsafe { peripherals::PA15::steal() },
            Level::High,
            OutputDrive::_5mA,
        )
        .degrade();
        // active low
        let res = Output::new(
            unsafe { peripherals::PA5::steal() },
            Level::High,
            OutputDrive::_5mA,
        )
        .degrade();
        // SPI MODE_0, clk idle low, data valid on rising edge
        let cs = Output::new(
            unsafe { peripherals::PB12::steal() },
            Level::Low,
            OutputDrive::_5mA,
        )
        .degrade();
        let en = Output::new(
            unsafe { peripherals::PB7::steal() },
            Level::Low,
            OutputDrive::_5mA,
        )
        .degrade();
        Self {
            en,
            u8g2: RefCell::new(u8g2),
            spi_host,
            dc,
            res,
            cs,
            driver_ic,
        }
    }

    pub fn init(&mut self) {
        self.en.set_high();
        self.u8g2.borrow_mut().u8x8.user_ptr = self as *mut _ as *mut c_void;
        self.init_display();
    }

    fn init_display(&mut self) {
        let u8x8 = &mut self.u8g2.borrow_mut().u8x8;
        unsafe {
            u8x8_InitDisplay(u8x8);
        }
    }

    //000101
    pub fn set_power_save(&mut self, enable: bool) {
        if enable {
            self.en.set_low();
            unsafe { u8x8_SetPowerSave(&mut self.u8g2.borrow_mut().u8x8, 1) };
            self.cs.set_low();
            match self.driver_ic {
                DriverIC::SSD1607 => {
                    self.res.set_low();
                    let _ = Input::new(unsafe { peripherals::PA4::steal().degrade() }, Pull::None);
                }
                DriverIC::SSD1681 => {
                    self.res.set_high();
                    let _ = Input::new(unsafe { peripherals::PA4::steal().degrade() }, Pull::Up);
                }
            }
        } else {
            self.en.set_high();
            unsafe { u8x8_SetPowerSave(&mut self.u8g2.borrow_mut().u8x8, 0) };
            self.cs.set_low();
        }
    }

    pub fn clear_buffer(&mut self) {
        unsafe {
            u8g2_ClearBuffer(&mut *self.u8g2.borrow_mut());
        }
    }

    pub fn send_buffer(&mut self) {
        unsafe { u8g2_SendBuffer(&mut *self.u8g2.borrow_mut()) };
    }

    pub fn set_font_mode(&mut self, is_transparent: u8) {
        unsafe {
            u8g2_SetFontMode(&mut *self.u8g2.borrow_mut(), is_transparent);
        }
    }

    pub fn set_font_direction(&mut self, dir: u8) {
        unsafe { u8g2_SetFontDirection(&mut *self.u8g2.borrow_mut(), dir) }
    }

    pub fn set_font(&mut self, font: &[u8]) {
        unsafe {
            u8g2_SetFont(&mut *self.u8g2.borrow_mut(), font.as_ptr());
        }
    }

    pub fn draw_utf8(&mut self, x: u16, y: u16, str_: &str) {
        unsafe {
            u8g2_DrawUTF8(
                &mut *self.u8g2.borrow_mut(),
                x,
                y,
                str_.as_ptr() as *const i8,
            );
        }
    }

    pub fn draw_str(&mut self, x: u16, y: u16, str_: &str) {
        unsafe {
            u8g2_DrawStr(
                &mut *self.u8g2.borrow_mut(),
                x,
                y,
                str_.as_ptr() as *const c_char,
            );
        }
    }

    pub fn draw_xbm(
        &mut self,
        x: i16,
        y: i16,
        w: u16,
        h: u16,
        bitmap: *const ::core::ffi::c_uchar,
    ) {
        unsafe {
            u8g2_DrawXBM(
                &mut *self.u8g2.borrow_mut(),
                x as u16,
                y as u16,
                w,
                h,
                bitmap,
            )
        }
    }

    pub fn set_draw_color(&mut self, color: u8) {
        unsafe {
            u8g2_SetDrawColor(&mut *self.u8g2.borrow_mut(), color);
        }
    }

    pub fn draw_box(&mut self, x: u16, y: u16, w: u16, h: u16) {
        unsafe { u8g2_DrawBox(&mut *self.u8g2.borrow_mut(), x, y, w, h) };
    }

    pub fn draw_frame(&mut self, x: u16, y: u16, w: u16, h: u16) {
        unsafe { u8g2_DrawFrame(&mut *self.u8g2.borrow_mut(), x, y, w, h) };
    }

    #[rustfmt::skip]
    pub fn embassy_logo(&mut self) {
        self.clear_buffer();
        self.draw_xbm(0, 0, 200, 200, assets::img::PAIR_IMG.as_ptr());
        self.send_buffer();
    }

    pub fn is_friday(&mut self, time: Time) {
        self.clear_buffer();
        self.set_font_mode(1);
        self.set_font_direction(0);
        unsafe {
            self.set_font(&u8g2_font_fusion_pixel_16_mn);
        }
        let mut time_label = StringWriter::new();

        write!(
            &mut time_label,
            "{:04}年{:02}月{:02}日\0",
            time.year as u16 + 1970,
            time.month,
            time.day
        )
        .unwrap();
        self.draw_utf8(20, 32, time_label.as_str());
        self.draw_utf8(20, 56, "今天是周五吗\0");
        if time.week == 4 {
            self.draw_xbm(
                12 - 88 + 44,
                100,
                176,
                88,
                assets::img::IMG_NOPE_ANSWER.as_ptr(),
            );
            self.set_draw_color(0);
            self.draw_box(0, 100, 56, 88);
            self.set_draw_color(1);
        } else {
            self.draw_xbm(12, 100, 176, 88, assets::img::IMG_NOPE_ANSWER.as_ptr());
        }
        self.send_buffer();
    }

    pub fn scan_mode(&mut self) {}
}
