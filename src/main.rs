#![no_std]
#![no_main]

mod oled;
mod uart;

use crate::oled::I2cPins;
use crate::uart::UartPins;
use defmt::info;
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello, world!");
    let p = embassy_stm32::init(Default::default());
    let i2c_pin: I2cPins = (p.I2C1, p.PB6, p.PB7, p.DMA1_CH6, p.DMA1_CH7);
    let uart_pin: UartPins = (p.USART1, p.PA10, p.PA9);

    // _spawner.spawn(oled::oled_task(i2c_pin)).unwrap();
    _spawner.spawn(uart::uart_task(uart_pin, _spawner)).unwrap();
}
