use core::mem;
use stm32_metapac::gpio::regs::Odr;

use embedded_hal::delay::DelayNs;
use mipidsi::dcs::InterfaceExt;
use stm32_metapac as pac;

/* 常用画笔颜色 */
pub const WHITE: u16 = 0xFFFF; /* 白色 */
pub const BLACK: u16 = 0x0000; /* 黑色 */
pub const RED: u16 = 0xF800; /* 红色 */
pub const GREEN: u16 = 0x07E0; /* 绿色 */
pub const BLUE: u16 = 0x001F; /* 蓝色 */
pub const MAGENTA: u16 = 0xF81F; /* 品红色/紫红色 = BLUE + RED */
pub const YELLOW: u16 = 0xFFE0; /* 黄色 = GREEN + RED */
pub const CYAN: u16 = 0x07FF; /* 青色 = GREEN + BLUE */

// 方向常量 (Direction Constants)
const L2R_U2D: u8 = 0; // 从左到右,从上到下 (Left to Right, Top to Bottom)
const L2R_D2U: u8 = 1; // 从左到右,从下到上 (Left to Right, Bottom to Top)
const R2L_U2D: u8 = 2; // 从右到左,从上到下 (Right to Left, Top to Bottom)
const R2L_D2U: u8 = 3; // 从右到左,从下到上 (Right to Left, Bottom to Top)
const U2D_L2R: u8 = 4; // 从上到下,从左到右 (Top to Bottom, Left to Right)
const U2D_R2L: u8 = 5; // 从上到下,从右到左 (Top to Bottom, Right to Left)
const D2U_L2R: u8 = 6; // 从下到上,从左到右 (Bottom to Top, Left to Right)
const D2U_R2L: u8 = 7; // 从下到上,从右到左 (Bottom to Top, Right to Left)

// ST7789 常量
const ST7789: u16 = 0x7789;

/// LCD 设备状态结构体
pub struct LcdDev {
    pub width: u16,  // LCD 宽度
    pub height: u16, // LCD 高度
    pub dir: u8,     // 当前方向 (0: 竖屏 / Portrait, 1: 横屏 / Landscape)
    pub setxcmd: u8, // 设置 X 坐标范围的命令 (Command to set X address range, e.g., CASET 0x2A)
    pub setycmd: u8, // 设置 Y 坐标范围的命令 (Command to set Y address range, e.g., PASET 0x2B)
    pub wramcmd: u8, // 写入内存命令 (Command to write to memory, e.g., RAMWR 0x2C)
}

impl LcdDev {
    /// 创建一个新的ST7789 LCD设备
    pub fn new_st7789(width: u16, height: u16, dir: u8) -> Self {
        Self {
            width,
            height,
            dir,
            setxcmd: 0x2A, // CASET命令
            setycmd: 0x2B, // PASET命令
            wramcmd: 0x2C, // RAMWR命令
        }
    }
}

pub fn lcd_write_data(gpio_port_ptr: *const pac::gpio::Gpio, data: u16) {
    let data = Odr(data as u32);
    unsafe { (*gpio_port_ptr).odr().write_value(data) }
}
/// 设置LCD的自动扫描方向 (ST7789专用)
///
/// Args:
/// * `dcs`: 实现 InterfaceExt trait 的可变引用，用于发送命令.
/// * `lcddev`: LcdDev 结构体的可变引用，包含 LCD 的状态信息.
/// * `dir`: 0~7, 代表8个方向 (具体定义见常量).
///
/// Returns:
/// * `Ok(())` 如果成功.
/// * `Err(T::Error)` 如果与 LCD 通信时发生错误.
pub fn lcd_scan_dir<T: InterfaceExt>(
    dcs: &mut T,
    lcddev: &mut LcdDev,
    mut dir: u8, // 使 dir 可变，因为后面可能重新赋值
) -> Result<(), T::Error> {
    // MADCTL 命令
    const MADCTL: u8 = 0x36;

    // 如果是横屏模式，需要转换方向
    if lcddev.dir == 1 {
        dir = match dir {
            0 => 6,
            1 => 7,
            2 => 4,
            3 => 5,
            4 => 1,
            5 => 0,
            6 => 3,
            7 => 2,
            _ => dir,
        };
    }

    // MADCTL 寄存器的位定义
    const MADCTL_MY: u8 = 1 << 7; // Row Address Order (0=Top to Bottom, 1=Bottom to Top)
    const MADCTL_MX: u8 = 1 << 6; // Column Address Order (0=Left to Right, 1=Right to Left)
    const MADCTL_MV: u8 = 1 << 5; // Row/Column Exchange (0=Normal, 1=Exchanged)
    const MADCTL_BGR: u8 = 1 << 3; // RGB/BGR Order (0=RGB, 1=BGR)

    // 根据方向设置 MADCTL 参数
    let mut madctl_param: u8 = 0;
    match dir {
        L2R_U2D => { /* 默认值 */ }
        L2R_D2U => madctl_param |= MADCTL_MY,
        R2L_U2D => madctl_param |= MADCTL_MX,
        R2L_D2U => madctl_param |= MADCTL_MY | MADCTL_MX,
        U2D_L2R => madctl_param |= MADCTL_MV,
        U2D_R2L => madctl_param |= MADCTL_MX | MADCTL_MV,
        D2U_L2R => madctl_param |= MADCTL_MY | MADCTL_MV,
        D2U_R2L => madctl_param |= MADCTL_MY | MADCTL_MX | MADCTL_MV,
        _ => { /* 无效方向，保持默认值 */ }
    }

    // ST7789 需要设置 BGR 位
    madctl_param |= MADCTL_BGR;

    // 写入 MADCTL 命令
    dcs.write_raw(MADCTL, &[madctl_param])?;

    // 检查是否需要交换宽高
    let xy_swapped = (madctl_param & MADCTL_MV) != 0;

    if xy_swapped {
        // 如果当前是竖屏但将要变为横屏
        if lcddev.width < lcddev.height {
            mem::swap(&mut lcddev.width, &mut lcddev.height);
        }
    } else {
        // 如果当前是横屏但将要变为竖屏
        if lcddev.width > lcddev.height {
            mem::swap(&mut lcddev.width, &mut lcddev.height);
        }
    }

    // 设置显示区域(开窗)大小为全屏

    let width = lcddev.width;
    let height = lcddev.height;

    // 坐标范围从0开始
    let x_start: u16 = 0;
    let y_start: u16 = 0;
    let x_end = width.saturating_sub(1);
    let y_end = height.saturating_sub(1);

    // CASET数据：起始X(16位，大端)，结束X(16位，大端)
    let x_data = [
        (x_start >> 8) as u8,
        (x_start & 0xFF) as u8,
        (x_end >> 8) as u8,
        (x_end & 0xFF) as u8,
    ];
    dcs.write_raw(lcddev.setxcmd, &x_data)?;

    // PASET数据：起始Y(16位，大端)，结束Y(16位，大端)
    let y_data = [
        (y_start >> 8) as u8,
        (y_start & 0xFF) as u8,
        (y_end >> 8) as u8,
        (y_end & 0xFF) as u8,
    ];
    dcs.write_raw(lcddev.setycmd, &y_data)?;

    Ok(())
}

/// ST7789 显示控制器初始化
pub fn st7789_init<T: InterfaceExt>(
    dcs: &mut T,
    delay: &mut impl DelayNs,
) -> Result<LcdDev, T::Error> {
    dcs.write_raw(0x11, &[])?;
    delay.delay_ms(120);
    dcs.write_raw(0x36, &[0x00])?;
    dcs.write_raw(0x3A, &[0x05])?;
    dcs.write_raw(0xB2, &[0x0C, 0x0C, 0x00, 0x33, 0x33])?;
    dcs.write_raw(0xB7, &[0x35])?;
    dcs.write_raw(0xBB, &[0x32])?;
    dcs.write_raw(0xC0, &[0x0C])?;
    dcs.write_raw(0xC2, &[0x01])?;
    dcs.write_raw(0xC3, &[0x10])?;
    dcs.write_raw(0xC4, &[0x20])?;
    dcs.write_raw(0xC6, &[0x0f])?;
    dcs.write_raw(0xD0, &[0xA4, 0xA1])?;
    dcs.write_raw(
        0xE0,
        &[
            0xD0, 0x00, 0x02, 0x07, 0x0A, 0x28, 0x32, 0x44, 0x42, 0x06, 0x0E, 0x12, 0x14, 0x17,
        ],
    )?;
    dcs.write_raw(
        0xE1,
        &[
            0xD0, 0x00, 0x02, 0x07, 0x0A, 0x28, 0x31, 0x54, 0x47, 0x0E, 0x1C, 0x17, 0x1B, 0x1E,
        ],
    )?;
    dcs.write_raw(0x2A, &[0x00, 0x00, 0x00, 0xEF])?;
    dcs.write_raw(0x2B, &[0x00, 0x00, 0x01, 0x3F])?;
    delay.delay_us(120_000);

    dcs.write_raw(0x29, &[])?;

    // 创建并初始化LCD设备结构
    let mut lcd = LcdDev::new_st7789(240, 320, 0); // 默认竖屏

    // 设置默认扫描方向（从左到右，从上到下）
    lcd_scan_dir(dcs, &mut lcd, L2R_U2D)?;

    Ok(lcd)
}

pub fn lcd_clear<T: InterfaceExt>(
    dcs: &mut T,
    lcddev: &LcdDev,
    color: u16,
) -> Result<(), T::Error> {
    // 1. 设置光标/窗口到全屏 (0,0) 到 (width-1, height-1)
    let width = lcddev.width;
    let height = lcddev.height;
    let x_start: u16 = 0;
    let y_start: u16 = 0;
    let x_end = width.saturating_sub(1);
    let y_end = height.saturating_sub(1);
    // 发送 CASET (Column Address Set)
    let x_data = [
        (x_start >> 8) as u8,
        (x_start & 0xFF) as u8,
        (x_end >> 8) as u8,
        (x_end & 0xFF) as u8,
    ];
    dcs.write_raw(lcddev.setxcmd, &x_data)?;
    // 发送 PASET (Page Address Set)
    let y_data = [
        (y_start >> 8) as u8,
        (y_start & 0xFF) as u8,
        (y_end >> 8) as u8,
        (y_end & 0xFF) as u8,
    ];
    dcs.write_raw(lcddev.setycmd, &y_data)?;
    // 2. 写入颜色数据到整个屏幕 (通过重复发送 RAMWR + 数据块)
    let total_pixels = width as u32 * height as u32;
    if total_pixels == 0 {
        return Ok(());
    }
    // 将 16 位颜色值转换为 Big Endian 的字节数组
    let pixel_bytes = color.to_be_bytes(); // [MSB, LSB]
                                           // --- 分块写入 ---
                                           // 定义缓冲区大小 (例如 256 个像素 = 512 字节)
                                           // 调整此值以平衡内存使用和 `write_raw` 调用次数
    const BUFFER_PIXELS: usize = 256;
    const BUFFER_SIZE: usize = BUFFER_PIXELS * 2;
    let mut buffer = [0u8; BUFFER_SIZE];
    // 填充整个缓冲区
    for i in 0..BUFFER_PIXELS {
        buffer[i * 2] = pixel_bytes[0]; // MSB
        buffer[i * 2 + 1] = pixel_bytes[1]; // LSB
    }
    let full_chunks = (total_pixels as usize) / BUFFER_PIXELS;
    let remaining_pixels = (total_pixels as usize) % BUFFER_PIXELS;
    // 获取写 RAM 命令字节
    let write_cmd = lcddev.wramcmd; // 通常是 0x2C
                                    // 发送完整的缓冲区块
                                    // **注意**: 每个 write_raw 调用都发送一次 write_cmd (0x2C)
    for _ in 0..full_chunks {
        dcs.write_raw(write_cmd, &buffer)?;
    }
    // 发送剩余不足一个缓冲区的像素数据
    if remaining_pixels > 0 {
        let remaining_bytes = remaining_pixels * 2;
        // **注意**: 这里也发送一次 write_cmd (0x2C)
        dcs.write_raw(write_cmd, &buffer[0..remaining_bytes])?;
    }
    Ok(())
}
