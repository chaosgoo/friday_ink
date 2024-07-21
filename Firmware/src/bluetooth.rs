use crate::config;
use crate::regs;
use crate::rtc;
use crate::rtc::Time;
use ch58x::ch58x;
use ch58x_hal::ble::ffi::*;
use ch58x_hal::ble::gap::*;
use ch58x_hal::ble::MacAddress;
use ch58x_hal::gpio::Input;
use ch58x_hal::gpio::Pin;
use ch58x_hal::gpio::Pull;
use ch58x_hal::with_safe_access;
use ch58x_hal::{ble, peripherals, println};
use chrono::prelude::*;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use qingke::riscv;
use qingke_rt::highcode;

type CS = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

static EVENTS: Channel<CS, bool, 10> = Channel::new();

const DEFAULT_DISCOVERY_MODE: u8 = DEVDISC_MODE_ALL;
const DEFAULT_DISCOVERY_ACTIVE_SCAN: u8 = 1; // false
const DEFAULT_DISCOVERY_WHITE_LIST: u8 = 0;

static mut SCAN_RES_SIZE: u8 = 0;

#[repr(C)]
#[derive(Debug)]
pub struct AdStructure<'d> {
    ad_len: u8,
    ad_type: u8,
    ad_data: &'d [u8],
}

impl<'d> AdStructure<'d> {
    pub fn new(data: &'d [u8]) -> Option<Self> {
        if data.len() < 3 {
            // 数据长度不足，无法创建 AdStructure
            return None;
        }

        let ad_len = data[0] as usize;
        let ad_type = data[1];
        if data.len() < 1 + ad_len {
            // 数据长度不足以包含完整的 AD 数据
            return None;
        }

        let ad_data = &data[2..1 + ad_len];
        let ad_structure = AdStructure {
            ad_len: ad_len as u8,
            ad_type,
            ad_data,
        };
        Some(ad_structure)
    }

    pub fn parse2time(&self) -> Option<Time> {
        if self.ad_type == 0x09 || self.ad_type == 0x08 {
            // 校验是否符合时间规则
            // 'F' + 时间戳的16进制字符串 + 'R'
            match self.ad_data {
                [b'F', hex @ .., b'R'] => {
                    let mut result = 0i32;
                    for d in hex {
                        let digit = match d {
                            b'0'..=b'9' => *d as i32 - b'0' as i32,
                            b'A'..=b'F' => 10 + *d as i32 - b'A' as i32,
                            b'a'..=b'f' => 10 + *d as i32 - b'a' as i32,
                            _ => 0, // 如果字符不是十六进制字符，则默认为 0
                        };
                        result = result.checked_mul(16).unwrap_or(0) + digit;
                    }
                    let dt: DateTime<Utc> = DateTime::from_timestamp(result.into(), 0)?;
                    println!("adv time: {:?}", dt);
                    // 打印时间
                    return Some(Time {
                        year: (dt.year() - 1970) as u8,
                        month: dt.month() as u8,
                        day: dt.day() as u8,
                        hour: dt.hour() as u8,
                        minute: dt.minute() as u8,
                        second: dt.second() as u8,
                        // week: dt.iso_week().week() as u8,
                        week: dt.weekday() as u8,
                    });
                }
                data => {
                    // println!("ad_data not match {:02x?}", data);
                }
            }
        }
        None
    }
}

unsafe extern "C" fn observer_event_callback(event: &gapRoleEvent_t) {
    // println!("observer_event_callback: {:?}", event.gap);
    match event.gap.opcode {
        GAP_DEVICE_INIT_DONE_EVENT => {
            let ret = GAPRole_ObserverStartDiscovery(
                DEFAULT_DISCOVERY_MODE,
                DEFAULT_DISCOVERY_ACTIVE_SCAN,
                DEFAULT_DISCOVERY_WHITE_LIST,
            );
            println!("Discovering...{:?}", ret);
        }
        GAP_DEVICE_DISCOVERY_EVENT => {
            println!("Complete");
            EVENTS.try_send(true).unwrap();
        }
        GAP_DIRECT_DEVICE_INFO_EVENT => {
            println!("GAP_DIRECT_DEVICE_INFO_EVENT");
        }
        GAP_EXT_ADV_DEVICE_INFO_EVENT => {
            println!("GAP_EXT_ADV_DEVICE_INFO_EVENT");
        }
        GAP_DEVICE_INFO_EVENT => {
            // SCAN_RES_SIZE += 1;
            // if SCAN_RES_SIZE > 10 {
            //     SCAN_RES_SIZE = 0;
            //     EVENTS.try_send(true).unwrap();
            // }
            let event = event.deviceInfo;
            println!(
                "Device => {}, rssi={}",
                MacAddress::from_raw(event.addr),
                event.rssi
            );
            let data = core::slice::from_raw_parts(event.pEvtData, event.dataLen as _);
            let mut i = 0;
            // println!("data: {:02x?}", data);
            if data.len() == 0 {
                return;
            }
            while i < data.len() {
                let might_ad = AdStructure::new(&data[i..1 + (data[i] as usize + i)]);
                if let Some(ad) = might_ad {
                    if let Some(time) = ad.parse2time() {
                        let rtc = rtc::take();
                        rtc.set_time(time).unwrap();
                        // ch58x_hal::delay_ms(50u16);
                        unsafe {
                            ch58x_hal::reset();
                        }
                        break;
                    };
                };
                i += (data[i] + 1) as usize;
            }
        }
        _ => {
            println!("unknown event opcode: {}", event.gap.opcode);
        }
    }
}

#[embassy_executor::task]
pub async fn observer_task() {
    static CALLBACK: gapRoleObserverCB_t = gapRoleObserverCB_t {
        eventCB: Some(observer_event_callback),
    };
    unsafe {
        println!("GAPRole_ObserverStartDevice");
        GAPRole_ObserverStartDevice(&CALLBACK).unwrap();
    }
    loop {
        EVENTS.receiver().receive().await;
        println!("Restarting discovery...");
        unsafe {
            let ret = GAPRole_CancelSync();
            println!("Restarting GAPRole_CancelSync...{:?}", ret);
            let ret = GAPRole_ObserverStartDiscovery(
                DEFAULT_DISCOVERY_MODE,
                DEFAULT_DISCOVERY_ACTIVE_SCAN,
                DEFAULT_DISCOVERY_WHITE_LIST,
            );
            println!("Restarting discovery...{:?}", ret);
        }
    }
}

#[embassy_executor::task]
pub async fn observer_timeout_task() {
    let wake_up_btn = Input::new(unsafe { peripherals::PB4::steal().degrade() }, Pull::Up);
    let rtc = rtc::take();
    let pair_begin = rtc.now().unwrap();
    let mut delta: u8 = 0;
    println!("pair_begin={:?}", pair_begin);
    loop {
        Timer::after(Duration::from_millis(500)).await;
        let now = rtc.now().unwrap();
        println!("now={:?}", now);
        let new_delta = if now.second < pair_begin.second {
            (60 - pair_begin.second) as u8 + now.second
        } else {
            now.second - pair_begin.second
        };
        if new_delta != delta {
            println!("delta={}", delta);
            delta = new_delta;
        }
        if delta > config::PAIR_MODE_TIME_OUT {
            unsafe {
                ch58x_hal::reset();
            }
        } else if delta > 5 {
            // 5s 之后才可以使用按钮重置
            if wake_up_btn.is_low() {
                Timer::after(Duration::from_millis(config::DEBOUNCE_TIME)).await;
                if wake_up_btn.is_low() {
                    unsafe {
                        ch58x_hal::reset();
                    }
                }
            }
        }
    }
}

#[highcode]
pub fn observer_task_init() {
    let pfic = unsafe { ch58x::PFIC::steal() };
    pfic.irer1().write(|w| unsafe { w.bits(12 & 0x1F) });
    unsafe {
        riscv::asm::nop();
        riscv::asm::nop();
    }
    let sys = unsafe { ch58x::SYS::steal() };
    with_safe_access(|| unsafe {
        sys.ck32k_config()
            .modify(|r, w| w.bits(r.bits() & !(regs::RB_CLK_OSC32K_XT | regs::RB_CLK_XT32K_PON)))
    });
    with_safe_access(|| unsafe {
        sys.ck32k_config()
            .modify(|r, w| w.bits(r.bits() | regs::RB_CLK_INT32K_PON))
    });
    println!("System Clocks: {}", ch58x_hal::sysctl::clocks().hclk);
    println!("ChipID: 0x{:02x}", ch58x_hal::signature::get_chip_id());

    let (task_id, _) = ble::init(ble::Config::default()).unwrap();
    println!("BLE init task id: {:?}", task_id);
    println!("MemFree: {}K", ch58x_hal::stack_free() / 1024);

    unsafe {
        let observer_init = GAPRole_ObserverInit();
        println!("GAPRole_ObserverInit: {:?}", observer_init);
    }
    // Observer_Init
    unsafe {
        // 4800 * 0.625ms = 3s
        let scan_res = 8u8;
        GAPRole_SetParameter(GAPROLE_MAX_SCAN_RES, 1, &scan_res as *const _ as _).unwrap();
        GAP_SetParamValue(TGAP_DISC_SCAN, 4800).unwrap();
        GAP_SetParamValue(TGAP_FILTER_ADV_REPORTS, 1).unwrap();
        GAP_SetParamValue(TGAP_DISC_SCAN_PHY, GAP_PHY_BIT_LE_1M).unwrap();
    }
}
