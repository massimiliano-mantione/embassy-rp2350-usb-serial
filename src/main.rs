#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::Timer;
use embassy_usb::UsbDevice;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcAcmState};
use panic_probe as _;
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

type UsbDriver = Driver<'static, USB>;
type StaticUsbDevice = UsbDevice<'static, UsbDriver>;

#[embassy_executor::task]
async fn usb_task(mut usb: StaticUsbDevice) -> ! {
    usb.run().await
}

static CDC_ACM_STATE: StaticCell<CdcAcmState> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let config = {
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("Embassy");
        config.product = Some("USB-serial example");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };

    // Create CDC ACM class on the builder.
    let cdc_acm_state = CDC_ACM_STATE.init(CdcAcmState::new());
    // Comment this out to avoid the crash
    let usb_serial = CdcAcmClass::new(&mut builder, cdc_acm_state, 64);

    // Build the builder.
    let usb = builder.build();
    spawner.spawn(usb_task(usb)).unwrap();

    let mut led = Output::new(p.PIN_25, Level::Low);

    let mut counter = 0;
    loop {
        counter += 1;
        //log::info!("Tick {}", counter);
        led.set_level(if counter % 2 == 0 {
            Level::High
        } else {
            Level::Low
        });
        Timer::after_secs(1).await;
    }
}
