use std::env;
use std::path::PathBuf;
fn main() {
    let src = [
        // "u8g2/csrcmui_u8g2.c",
        // "u8g2/csrcmui.c",
        "u8g2/csrc/u8g2_arc.c",
        "u8g2/csrc/u8g2_bitmap.c",
        "u8g2/csrc/u8g2_box.c",
        "u8g2/csrc/u8g2_buffer.c",
        "u8g2/csrc/u8g2_button.c",
        "u8g2/csrc/u8g2_circle.c",
        "u8g2/csrc/u8g2_cleardisplay.c",
        "u8g2/csrc/u8g2_d_memory.c",
        "u8g2/csrc/u8g2_d_setup.c",
        "u8g2/csrc/u8g2_font.c",
        "u8g2/csrc/u8g2_fonts.c",
        "u8g2/csrc/u8g2_hvline.c",
        "u8g2/csrc/u8g2_input_value.c",
        "u8g2/csrc/u8g2_intersection.c",
        "u8g2/csrc/u8g2_kerning.c",
        "u8g2/csrc/u8g2_line.c",
        "u8g2/csrc/u8g2_ll_hvline.c",
        "u8g2/csrc/u8g2_message.c",
        "u8g2/csrc/u8g2_polygon.c",
        "u8g2/csrc/u8g2_selection_list.c",
        "u8g2/csrc/u8g2_setup.c",
        "u8g2/csrc/u8g2.h",
        "u8g2/csrc/u8log_u8g2.c",
        "u8g2/csrc/u8log_u8x8.c",
        "u8g2/csrc/u8log.c",
        "u8g2/csrc/u8x8_8x8.c",
        "u8g2/csrc/u8x8_byte.c",
        "u8g2/csrc/u8x8_cad.c",
        "u8g2/csrc/u8x8_capture.c",
        "u8g2/csrc/u8x8_d_ssd1607_200x200.c",
        "u8g2/csrc/u8x8_d_ssd1681_200x200.c",
        "u8g2/csrc/u8x8_debounce.c",
        "u8g2/csrc/u8x8_display.c",
        "u8g2/csrc/u8x8_fonts.c",
        "u8g2/csrc/u8x8_gpio.c",
        "u8g2/csrc/u8x8_input_value.c",
        "u8g2/csrc/u8x8_message.c",
        "u8g2/csrc/u8x8_selection_list.c",
        "u8g2/csrc/u8x8_setup.c",
        "u8g2/csrc/u8x8_string.c",
        "u8g2/csrc/u8x8_u8toa.c",
        "u8g2/csrc/u8x8_u16toa.c",
        "font/u8g2_font_fusion_pixel_16_mn.c",
    ];
    let stdint_path = "/mnt/c/MounRiver/MRS_Community/toolchain/toolchain/riscv-none-embed/include"; // 根据你的输出调整此路径
    let stdarg_path =
        "/mnt/c/MounRiver/MRS_Community/toolchain/toolchain/lib/gcc/riscv-none-embed/8.2.0/include"; // 根据你的输出调整此路径
    let limits_path = "/mnt/c/MounRiver/MRS_Community/toolchain/toolchain/lib/gcc/riscv-none-embed/8.2.0/include-fixed"; // 根据你的输出调整此路径
                                                                                                                         // C:\MounRiver\MRS_Community\toolchain\toolchain\lib\gcc\riscv-none-embed\8.2.0\include
    let mut builder = cc::Build::new();
    let build = builder
        .include("u8g2/csrc")
        .include("font")
        .files(src.iter())
        .static_flag(true);

    build.compile("u8g2_rs");
    let bindings = bindgen::Builder::default()
        .clang_arg(format!("-I{}", stdint_path))
        .clang_arg(format!("-I{}", stdarg_path))
        .clang_arg(format!("-I{}", limits_path))
        .clang_arg("--target=riscv32")
        .clang_arg("-march=rv32imac")
        .clang_arg("-mabi=ilp32")
        .clang_arg("-mcmodel=medany")
        .header("u8g2/csrc/u8g2.h")
        .header("font/font.h")
        .clang_arg("-I./u8g2/csrc")
        .clang_arg("-I./font")
        .use_core()
        .generate()
        .expect("Unable to generate bindings");

    // 3. 将绑定文件写入到指定位置
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

// riscv-none-embed-gcc -march=rv32imac -mabi=ilp32 -mcmodel=medany -msmall-data-limit=8 -Os -fmessage-length=0 -fsigned-char -ffunction-sections -fdata-sections  -g -DDEBUG=1 -I".StdPeriphDriver/inc" -I"E:\GitHub\WeActStudio.WCH-BLE-Core\Examples\CH582\FridayInk\LIB\Port" -I"E:\GitHub\WeActStudio.WCH-BLE-Core\Examples\CH582\FridayInk\HAL\include" -I"E:\GitHub\WeActStudio.WCH-BLE-Core\Examples\CH582\FridayInk\LIB\u8g2\csrc" -I"E:\GitHub\WeActStudio.WCH-BLE-Core\Examples\CH582\FridayInk\LIB\u8g2" -I"E:\GitHub\WeActStudio.WCH-BLE-Core\Examples\CH582\FridayInk\LIB\softwire" -I"E:\GitHub\WeActStudio.WCH-BLE-Core\Examples\CH582\FridayInk\LIB\easylogger\inc" -I"E:\GitHub\WeActStudio.WCH-BLE-Core\Examples\CH582\FridayInk\LIB" -I".RVMSIS" -std=gnu99 -MMD -MP -MF"APP/FridayInk.d" -MT"APP/FridayInk.o" -c -o"APP/FridayInk.o"".APP/FridayInk.c"
