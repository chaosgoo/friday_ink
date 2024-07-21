use ch58x::ch58x;
use ch58x_hal::{isp::flash_rom_reset, with_safe_access};
use qingke::riscv;
use qingke_rt::highcode;

use crate::{gpio::{GPIO_PIN_22, GPIO_PIN_23, GPIO_PIN_4}, regs};
#[derive(Clone, Copy)]
pub enum SysClk {
    ClkSourceLsi = 0x00,
    ClkSourceLse = 0x01,
    ClkSourceHse16mhz = 0x22,
    ClkSourceHse8mhz = 0x24,
    ClkSourceHse6_4mhz = 0x25,
    ClkSourceHse4mhz = 0x28,
    ClkSourceHse2mhz = (0x20 | 16),
    ClkSourceHse1mhz = (0x20 | 0),
    ClkSourcePll80mhz = 0x46,
    ClkSourcePll60mhz = 0x48,
    ClkSourcePll48mhz = (0x40 | 10),
    ClkSourcePll40mhz = (0x40 | 12),
    ClkSourcePll36_9mhz = (0x40 | 13),
    ClkSourcePll32mhz = (0x40 | 15),
    ClkSourcePll30mhz = (0x40 | 16),
    ClkSourcePll24mhz = (0x40 | 20),
    ClkSourcePll20mhz = (0x40 | 24),
    ClkSourcePll15mhz = (0x40 | 0),
}

fn set_sys_clock(sysclk: SysClk) {
    /*
       R8_SAFE_ACCESS_SIG = SAFE_ACCESS_SIG1;
       R8_SAFE_ACCESS_SIG = SAFE_ACCESS_SIG2;
       SAFEOPERATE;
       R8_PLL_CONFIG &= ~(1 << 5); //
       R8_SAFE_ACCESS_SIG = 0;
    */
    with_safe_access(|| unsafe {
        let sys = ch58x::SYS::steal();
        sys.pll_config().modify(|r, w| w.bits(r.bits() & !(1 << 5)));
    });

    if (sysclk as u8 & 0x20) != 0 {
        let sys = unsafe { ch58x::SYS::steal() };
        if sys.hfck_pwr_ctrl().read().clk_xt32m_pon().bit_is_clear() {
            with_safe_access(|| unsafe {
                sys.hfck_pwr_ctrl()
                    .modify(|_r, w| w.clk_xt32m_pon().set_bit());
                for _ in 0..=1200 {
                    riscv::asm::nop();
                    riscv::asm::nop();
                }
            })
        }
        with_safe_access(|| unsafe {
            let sys = ch58x::SYS::steal();
            sys.clk_sys_cfg()
                .modify(|_r, w| w.bits((0 << 6) | sysclk as u16 & 0x1f));
            riscv::asm::nop();
            riscv::asm::nop();
            riscv::asm::nop();
            riscv::asm::nop();
        });
        with_safe_access(|| unsafe {
            let sys = ch58x::SYS::steal();
            sys.flash_cfg().write(|w| w.bits(0x51))
        });
    } else if (sysclk as u8 & 0x40) != 0 {
        let sys = unsafe { ch58x::SYS::steal() };
        if sys.hfck_pwr_ctrl().read().clk_pll_pon().bit_is_clear() {
            with_safe_access(|| unsafe {
                sys.hfck_pwr_ctrl()
                    .modify(|_r, w| w.clk_pll_pon().set_bit());
                for _ in 0..=2000 {
                    riscv::asm::nop();
                    riscv::asm::nop();
                }
            })
        }
        with_safe_access(|| unsafe {
            sys.clk_sys_cfg()
                .modify(|_r, w| w.bits((1 << 6) | sysclk as u16 & 0x1f));
            riscv::asm::nop();
            riscv::asm::nop();
            riscv::asm::nop();
            riscv::asm::nop();
        });
        with_safe_access(|| unsafe {
            match sysclk {
                SysClk::ClkSourcePll80mhz => sys.flash_cfg().write(|w| w.bits(0x02)),
                _ => sys.flash_cfg().write(|w| w.bits(0x52)),
            }
        });
    } else {
        let sys = unsafe { ch58x::SYS::steal() };
        with_safe_access(|| unsafe {
            sys.clk_sys_cfg()
                .modify(|r, w| w.bits(r.bits() | regs::RB_CLK_SYS_MOD as u16));
        })
    }
    with_safe_access(|| unsafe {
        let sys = ch58x::SYS::steal();
        sys.pll_config().modify(|r, w| w.bits(r.bits() | 1 << 7));
    })
}

#[highcode]
pub fn low_power_shutdown(rm: u8) {
    let sys = unsafe { ch58x::SYS::steal() };
    let pfic = unsafe { ch58x::PFIC::steal() };

    flash_rom_reset();

    with_safe_access(|| unsafe {
        // 关闭电压监控
        sys.bat_det_ctrl().write(|w| w.bits(0));
        if sys.rtc_cnt_32k().read().bits() > 0x3fff {
            // 超过500ms
            // x32Kpw = (x32Kpw & 0xfc) | 0x01; // LSE驱动电流降低到额定电流
            sys.xt32k_tune()
                .modify(|r, w| w.bits((r.bits() & 0xfc) | 0x01));
        }
        sys.xt32m_tune()
            .modify(|r, w| w.bits((r.bits() & 0xfc) | 0x03));
        // x32Mpw = (x32Mpw & 0xfc) | 0x03; // 150%额定电流
    });

    set_sys_clock(SysClk::ClkSourceHse6_4mhz);

    // deep sleep
    pfic.sctlr()
        .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 2)) });
    with_safe_access(|| {
        sys.slp_power_ctrl()
            .modify(|_r, w| w.ram_ret_lv().set_bit());
        sys.power_plan().modify(|_r, w| unsafe {
            w.bits(regs::RB_PWR_PLAN_EN as u16 | regs::RB_PWR_MUST_0010 as u16 | rm as u16)
        });
    });
    with_safe_access(|| {
        sys.ck32k_config()
            .modify(|_r, w| w.clk_int32k_pon().set_bit())
    });
    unsafe {
        pfic.sctlr().modify(|r, w| w.bits(r.bits() & !(1 << 3)));
        riscv::asm::wfi();
        riscv::asm::nop();
        riscv::asm::nop();
    }
    flash_rom_reset();
    with_safe_access(|| {
        sys.rst_wdog_ctrl()
            .modify(|_r, w| w.software_reset().set_bit())
    });
}

pub fn wake_up_cfg() {
    let sys = unsafe { ch58x::SYS::steal() };
    let pfic = unsafe { ch58x::PFIC::steal() };
    // 0x00000010
    let pin: u16 = (GPIO_PIN_4 | ((GPIO_PIN_4 & (GPIO_PIN_22 | GPIO_PIN_23)) >> 14)) as u16;
    // GPIOB_ITModeCfg(GPIO_Pin_1, GPIO_ITMode_FallEdge); /* 下降沿唤醒 */
    sys.pb_int_mode()
        .modify(|r, w| unsafe { w.bits(r.bits() | pin) });
    sys.pb_clr()
        .modify(|r, w| unsafe { w.bits(r.bits() | pin as u32) });
    sys.pb_int_if().modify(|r, w| unsafe { w.bits(pin) });
    sys.pb_int_en().modify(|r, w| unsafe { w.bits(pin) });
    pfic.ienr2()
        .write(|w| unsafe { w.bits(1 << ((18u32) & 0x1F)) });
    with_safe_access(|| unsafe {
        let m = 0x00;
        riscv::asm::nop();
        sys.slp_wake_ctrl()
            .modify(|r, w| w.bits(r.bits() | regs::RB_WAKE_EV_MODE | regs::RB_SLP_GPIO_WAKE));
        sys.slp_power_ctrl().modify(|r, w| w.bits(r.bits() | m));
    });
}
