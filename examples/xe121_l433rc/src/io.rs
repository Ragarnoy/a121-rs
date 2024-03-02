use embassy_stm32::exti::ExtiInput;
use embassy_stm32::peripherals::PC13;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

pub static LED_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[embassy_executor::task]
pub async fn button_task(mut button: ExtiInput<'static, PC13>) {
    // Await the button press
    loop {
        button.wait_for_high().await;
        LED_SIGNAL.signal(());
    }
}
