//! ```
//!      ESP8266 -> STM32
//! (black)  GND -> GND
//! (red)    +5V -> VCC
//! (yellow) RXD -> PA10
//! (green)  TXD -> PA9
//! ```
use crate::{RX_PIPE, TX_PIPE};
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    mode::Async,
    peripherals::{self},
    usart::{self, Uart, UartRx},
};
use embassy_time::Timer;
use embedded_io_async::Write;

use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
});

const USART_BAUD: u32 = 115200;

pub type UartPins = (
    peripherals::USART1,
    peripherals::PA10,
    peripherals::PA9,
    peripherals::DMA1_CH4,
    peripherals::DMA1_CH5,
);

#[embassy_executor::task()]
pub async fn uart_task(p: UartPins, _spawner: Spawner) {
    let mut config = embassy_stm32::usart::Config::default();
    config.baudrate = USART_BAUD;

    let uart = Uart::new(
        p.0,  // 1. UART 外设
        p.1,  // 2. RX 引脚
        p.2,  // 3. TX 引脚
        Irqs, // 2. 中断
        p.3, p.4, config,
    )
    .expect("Create UART");

    let (mut tx, rx) = uart.split();
    unwrap!(_spawner.spawn(buffered_uart_reader(rx)));
    tx.write_all(b"ATE0\r\n").await.expect("开启回显失败");
    info!("Writing...");
    let mut buf = [0u8; 8];

    loop {
        let len = TX_PIPE.read(&mut buf).await;
        if len != 0 {
            tx.write_all(&buf[..len]).await.expect("Write failed");
            info!("write to uart: {}", buf)
        }
        Timer::after_micros(200).await;
    }
}

#[embassy_executor::task]
async fn buffered_uart_reader(mut rx: UartRx<'static, Async>) {
    info!("Reading...");
    loop {
        let mut buf = [0; 32];

        rx.read_until_idle(&mut buf).await.unwrap();

        // parse buf into utf8 string
        match core::str::from_utf8(&buf) {
            Ok(s) => {
                info!("raw byte: {:?}", s.as_bytes());
                info!("write {} bytes to PIPE", s.len());
                RX_PIPE.write(s.as_bytes()).await;
                info!("RX: {}", s);
            }
            Err(_e) => {
                info!("Utf8 error");
            }
        }
        Timer::after_secs(1).await;
    }
}
