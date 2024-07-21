#![no_std]
#![no_main]
#![feature(stmt_expr_attributes)]

extern crate u8g2_rs;

use ch58x_hal::ble::ffi::TMOS_SystemProcess;
use ch58x_hal::gpio::{Input, Level, Output, OutputDrive, Pin, Pull};
use ch58x_hal::peripherals;
use ch58x_hal::{println, uart::UartTx};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use friday_rs::bluetooth::{observer_task, observer_task_init, observer_timeout_task};
use friday_rs::display::{u8x8_byte_ch582f_hw_spi, u8x8_gpio_and_delay_ch582f, Display};
use friday_rs::rtc::set_default_rtc;
use friday_rs::softwire::SoftwareI2C;
use friday_rs::{config, power, rtc};
use qingke::riscv;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;

    let pa9 = unsafe { peripherals::PA9::steal() };
    let uart1 = unsafe { peripherals::UART1::steal() };
    let mut serial = UartTx::new(uart1, pa9, Default::default()).unwrap();

    let _ = writeln!(&mut serial, "panic\n\n\n{}", info);

    loop {}
}

enum FridayMode {
    Normal,
    TimePair,
}

fn print_embassy_logo() {
    println!("\n\nHello World from ch58x-hal!");
    println!(
        r#"
    ______          __
   / ____/___ ___  / /_  ____ _____________  __
  / __/ / __ `__ \/ __ \/ __ `/ ___/ ___/ / / /
 / /___/ / / / / / /_/ / /_/ (__  |__  ) /_/ /
/_____/_/ /_/ /_/_.___/\__,_/____/____/\__, /
                                      /____/   on CH582F"#
    );
    println!("System Clocks: {}", ch58x_hal::sysctl::clocks().hclk);
    println!("ChipID: 0x{:02x}", ch58x_hal::signature::get_chip_id());
}

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let mut config = ch58x_hal::Config::default();
    config.clock.use_pll_48mhz().enable_lse();
    let p = ch58x_hal::init(config);
    ch58x_hal::embassy::init();
    let uart = UartTx::new(p.UART1, p.PA9, Default::default()).unwrap();
    unsafe {
        ch58x_hal::set_default_serial(uart);
    }
    print_embassy_logo();

    let wake_up_btn = Input::new(unsafe { peripherals::PB4::steal().degrade() }, Pull::Up);
    let scl: u8 = p.PA13.pin() + p.PA13.port() * 32;
    let sda_pin: u8 = p.PA12.pin() + p.PA13.port() * 32;

    static mut SOFTI2C: Option<SoftwareI2C> = None;
    static mut RTC_INSTANCE: Option<rtc::PCF8563> = None;

    unsafe {
        SOFTI2C = Some(SoftwareI2C::new(sda_pin, scl));
        if let Some(ref mut i2c) = SOFTI2C {
            RTC_INSTANCE = Some(rtc::PCF8563::new(i2c));
        }
        set_default_rtc(RTC_INSTANCE.as_mut().unwrap());
    }

    let mut display = unsafe {
        Display::new(
            &u8g2_rs::u8g2_cb_r3,
            Some(u8x8_byte_ch582f_hw_spi),
            Some(u8x8_gpio_and_delay_ch582f),
        )
    };

    display.init();
    display.set_power_save(false);
    let rtc = rtc::take();
    if cfg!(feature = "power_measure") {
        let mut now = rtc.now().unwrap();
        if now.minute == 59 {
            rtc::take().set_alarm(0, 0, now.hour + 1, 0).unwrap();
        } else {
            rtc::take()
                .set_alarm(0, 0, now.hour, now.minute + 1)
                .unwrap();
        }
    } else {
        rtc.set_alarm(0, 0, config::ALARM_HOUR, config::ALARM_MINUTE)
            .unwrap();
    }
    let alarm = rtc.check_alarm();
    if let Ok(alarming) = alarm {
        if alarming {
            rtc.clear_alarm().unwrap();
            println!("wake up by ALARM!");
        } else {
            println!("wake up by PIN!");
        }
    }
    let mut boot_mode: FridayMode = FridayMode::Normal;
    if wake_up_btn.is_low() {
        Timer::after(Duration::from_millis(config::DEBOUNCE_TIME)).await;
        boot_mode = if wake_up_btn.is_low() {
            FridayMode::TimePair
        } else {
            FridayMode::Normal
        }
    }

    let now = rtc.now().unwrap();

    match boot_mode {
        FridayMode::TimePair => {
            println!("FridayMode::TimePair @ {:?}", now);
            display.embassy_logo();
        }
        FridayMode::Normal => {
            println!("FridayMode::Normal @ {:?}", now);
            display.is_friday(now);
        }
    }
    // waiting for epd draw done.
    ch58x_hal::delay_ms(1000u16);
    display.set_power_save(true);
    match boot_mode {
        FridayMode::Normal => {
            println!("Enter SLEEP");
            power::wake_up_cfg();
            power::low_power_shutdown(0);
        }
        _ => {
            observer_task_init();
            let _ = spawner.spawn(observer_timeout_task());
            let _ = spawner.spawn(observer_task());
        }
    }

    loop {
        Timer::after(Duration::from_micros(300)).await;
        unsafe {
            TMOS_SystemProcess();
        }
    }
}
