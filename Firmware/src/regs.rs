#[allow(unused)]
pub const SAFE_ACCESS_SIG1: u8 = 0x57; // WO: safe accessing sign value step 1
pub const SAFE_ACCESS_SIG2: u8 = 0xA8; // WO: safe accessing sign value step 2
pub const RB_CLK_XT32K_PON: u8 = 0x01; // RWA, external 32KHz oscillator power on
pub const RB_CLK_INT32K_PON: u8 = 0x02; // RWA, internal 32KHz oscillator power on
pub const RB_CLK_OSC32K_XT: u8 = 0x04; // RWA, 32KHz oscillator source selection: 0=RC, 1=XT
pub const RB_CLK_OSC32K_FILT: u8 = 0x08; // RWA, internal 32KHz oscillator low noise mode disable: 0=enable, 1=disable
pub const RB_32K_CLK_PIN: u8 = 0x80; // RO, 32KHz oscillator clock pin status
pub const RB_CLK_XT32M_PON: u8 = 0x04; // RWA, external 32MHz oscillator power control: 0=power down, 1-power on
pub const RB_CLK_SYS_MOD: u8 = 0xC0; // RWA, system clock source mode: 00=divided from 32MHz, 01=divided from PLL-480MHz, 10=directly from 32MHz, 11=directly from 32KHz
pub const RB_PWR_DCDC_EN: u16 = 0x0200u16; // RWA, DC/DC converter enable: 0=DC/DC disable and bypass, 1=DC/DC enable
pub const RB_PWR_DCDC_PRE: u16 = 0x0400u16; // RWA, DC/DC converter pre-enable
pub const RB_PWR_PLAN_EN: u16 = 0x8000u16; // RWA/WZ, power plan enable, auto clear after sleep executed
pub const RB_PWR_MUST_0010: u16 = 0x1000u16; // RWA, must write 0010
pub const RB_SLP_GPIO_WAKE: u8 = 0x10; // RWA, enable GPIO waking
pub const RB_WAKE_EV_MODE: u8 = 0x40; // RWA, event wakeup mode: 0=event keep valid for long time, 1=short pulse event

