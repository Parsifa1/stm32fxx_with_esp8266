//! ```
//!      Display -> STM32
//! (black)  GND -> GND
//! (red)    +5V -> VCC
//! (yellow) SDA -> PB7
//! (green)  SCL -> PB6
//! ```

use defmt_rtt as _;
use embassy_stm32::{
    bind_interrupts,
    gpio::AnyPin,
    i2c,
    peripherals::{self, DMA1_CH6, DMA1_CH7, I2C1, PB6, PB7},
    time::Hertz,
    Peripherals,
};
use panic_probe as _;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306Async};

pub type I2cPins = (I2C1, PB6, PB7, DMA1_CH6, DMA1_CH7);

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

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

    /* Endless loop */
    loop {
        for c in 97..123 {
            let _ = display
                .write_str(unsafe { core::str::from_utf8_unchecked(&[c]) })
                .await;
        }
        for c in 65..91 {
            let _ = display
                .write_str(unsafe { core::str::from_utf8_unchecked(&[c]) })
                .await;
        }
    }
}
