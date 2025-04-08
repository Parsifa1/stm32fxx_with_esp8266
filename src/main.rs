#![no_std]
#![no_main]

mod oled;
mod uart;

use crate::oled::I2cPins;
use crate::uart::UartPins;
use core::sync::atomic::Ordering;
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{AnyPin, Level, Output, Pin, Speed};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::pipe::Pipe;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

pub static PIPE: Pipe<ThreadModeRawMutex, 10> = Pipe::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello, world!");
    let p = embassy_stm32::init(Default::default());
    let i2c_pin: I2cPins = (p.I2C1, p.PB6, p.PB7, p.DMA1_CH6, p.DMA1_CH7);
    let uart_pin: UartPins = (p.USART1, p.PA10, p.PA9);

    _spawner.spawn(uart::uart_task(uart_pin, _spawner)).unwrap();
    _spawner.spawn(oled::oled_task(i2c_pin, _spawner)).unwrap();
    _spawner.spawn(led(p.PA8.degrade())).unwrap();
}

#[embassy_executor::task]
pub async fn led(pin: AnyPin) {
    let mut led = Output::new(pin, Level::Low, Speed::Low);
    loop {
        if oled::LED_FLAG.load(Ordering::Relaxed) {
            info!("led toggled");
            led.toggle();
            oled::LED_FLAG.store(false, Ordering::Relaxed);
            Timer::after_secs(1).await;
        } else {
            Timer::after_millis(10).await;
        }
    }
}
