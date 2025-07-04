use defmt::info;
use embassy_stm32::{
    exti::ExtiInput,
    gpio::Pull,
    peripherals::{EXTI5, PC5},
};

use crate::TX_PIPE;

pub type CtrPins = (PC5, EXTI5);

#[embassy_executor::task]
pub async fn ctr_task(pin: CtrPins) {
    let mut button = ExtiInput::new(pin.0, pin.1, Pull::Up);
    // let time_now =
    loop {
        button.wait_for_any_edge().await;
        if button.is_low() {
            info!("Button low");
            TX_PIPE.write(b"pressed!\r\n").await;
        } else if button.is_high() {
            info!("Button high");
            // TX_PIPE.write(b"is_high\n").await;
        }
    }
}
