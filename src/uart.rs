//! ```
//!      ESP8266 -> STM32
//! (black)  GND -> GND
//! (red)    +5V -> VCC
//! (yellow) RXD -> PA10
//! (green)  TXD -> PA9
//! ```
use crate::PIPE;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::peripherals;
use embassy_stm32::usart;
use embassy_stm32::usart::BufferedUartRx;
use embassy_stm32::{bind_interrupts, usart::BufferedUart};
use embassy_time::Timer;
use embedded_io_async::{Read, Write};
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART1 => usart::BufferedInterruptHandler<peripherals::USART1>;
});

const USART_BAUD: u32 = 9600;

pub type UartPins = (peripherals::USART1, peripherals::PA10, peripherals::PA9);

#[embassy_executor::task()]
pub async fn uart_task(p: UartPins, _spawner: Spawner) {
    info!("Hello World!");
    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 16])[..];
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; 16])[..];

    let mut config = embassy_stm32::usart::Config::default();
    config.baudrate = USART_BAUD;

    let uart = BufferedUart::new(
        p.0,  // 1. UART 外设
        Irqs, // 2. 中断
        p.1,  // 2. RX 引脚
        p.2,  // 3. TX 引脚
        tx_buf, rx_buf, config,
    )
    .expect("Create UART");

    let (mut tx, rx) = uart.split();
    unwrap!(_spawner.spawn(buffered_uart_reader(rx)));
    info!("Writing...");
    loop {
        let data = b"AT\r\n";
        info!("TX {:?}", data);
        tx.write_all(data).await.unwrap();
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn buffered_uart_reader(mut rx: BufferedUartRx<'static>) {
    info!("Reading...");
    loop {
        let mut buf = [0; 10];

        rx.read_exact(&mut buf).await.unwrap();

        // parse buf into utf8 string
        match core::str::from_utf8(&buf) {
            Ok(s) => {
                info!("raw byte: {:?}", s.as_bytes());
                PIPE.write(s.as_bytes()).await;
                info!("RX: {}", s);
            }
            Err(_e) => {
                info!("Utf8 error");
            }
        }
        Timer::after_secs(1).await;
    }
}
