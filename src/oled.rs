//! ```
//!      Display -> STM32
//! (black)  GND -> GND
//! (red)    +5V -> VCC
//! (yellow) SDA -> PB7
//! (green)  SCL -> PB6
//! ```

use core::sync::atomic::{AtomicBool, Ordering::Relaxed};
use defmt::*;
use defmt_rtt as _;
use embassy_stm32::{
    bind_interrupts, i2c,
    peripherals::{self, DMA1_CH6, DMA1_CH7, I2C1, PB6, PB7},
    time::Hertz,
};
use heapless::String;
use panic_probe as _;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306Async};

use crate::RX_PIPE;

pub type I2cPins = (I2C1, PB6, PB7, DMA1_CH6, DMA1_CH7);

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

pub static LED_FLAG: AtomicBool = AtomicBool::new(false);
pub static PIPE_LEN: usize = 32;

#[embassy_executor::task]
pub async fn oled_task(p: I2cPins) {
    let i2c = embassy_stm32::i2c::I2c::new(
        p.0,
        p.1,
        p.2,
        Irqs,
        p.3,
        p.4,
        Hertz::khz(400),
        Default::default(),
    );

    let interface: I2CInterface<_> = I2CDisplayInterface::new(i2c);
    let mut display: Ssd1306Async<_, _, _> =
        Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_terminal_mode();
    display.init().await.unwrap();
    let _ = display.clear().await;

    // _spawner.spawn(oled_clear()).unwrap();
    loop {
        let mut buf = [0u8; PIPE_LEN];
        let len = RX_PIPE.read(&mut buf).await;
        let len2 = RX_PIPE.try_read(&mut buf[len..]).unwrap_or(0);

        info!("len: {}, len2: {}", len, len2);
        let data = core::str::from_utf8(&buf[..len])
            .expect("Invalid UTF-8")
            .trim_end_matches("\0");

        let mut string: String<32> = heapless::String::from(data);
        let data = if data.ends_with('\n') {
            string.as_str()
        } else {
            string.push('\n').unwrap();
            string.as_str()
        };

        info!("buf: {:?}", data);
        if data.contains("clear") {
            info!("clear display");
            display.clear().await.unwrap();
            continue;
        }
        if data.contains("toggle") {
            LED_FLAG.store(true, Relaxed);
            continue;
        }

        info!("write asyncly");
        display.write_str(data).await.unwrap();
        // if CLEAR_FLAG.load(Relaxed) {
        //     info!("clear display");
        //     display.clear().await.unwrap();
        //     CLEAR_FLAG.store(false, Relaxed);
        // }
    }
}

// #[embassy_executor::task]
// pub async fn oled_clear() {
//     loop {
//         Timer::after_secs(10).await;
//         CLEAR_FLAG.store(true, Relaxed);
//     }
// }
