use anyhow::Context;
// use embedded_graphics::{
//     mono_font::{ascii::FONT_6X10, MonoTextStyle},
//     pixelcolor::BinaryColor,
//     prelude::Point,
//     text::Text,
//     Drawable,
// };
use esp_idf_hal::{
    delay::Delay,
    // units::KiloHertz,
    // i2c::{I2cConfig, I2cDriver},
    prelude::Peripherals,
    spi::{self, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use mfrc522::{
    comm::{eh02::spi::SpiInterface, Interface},
    Initialized, Mfrc522,
};
// use ssd1306::{
//     prelude::{Brightness, DisplayConfig},
//     rotation::DisplayRotation,
//     size::DisplaySize128x64,
// };

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let logger = esp_idf_svc::log::EspLogger;
    logger.set_target_level("rfid_esp32", log::LevelFilter::Trace)?;

    log::info!("STARTING...");

    let peripherals = Peripherals::take().context("no peripherals")?;

    // {
    //     let i2c = peripherals.i2c0;
    //     let scl = peripherals.pins.gpio22;
    //     let sda = peripherals.pins.gpio21;
    //     let config = I2cConfig::new().baudrate(KiloHertz(100).into());
    //     let i2c_driver = I2cDriver::new(i2c, sda, scl, &config)?;
    //     let i2c = ssd1306::I2CDisplayInterface::new(i2c_driver);
    //     let mut display = ssd1306::Ssd1306::new(i2c, DisplaySize128x64, DisplayRotation::Rotate180)
    //         .into_buffered_graphics_mode();
    //     display.init().map_err(|err| anyhow::anyhow!("{:?}", err))?;
    //     display
    //         .set_brightness(Brightness::DIM)
    //         .map_err(|err| anyhow::anyhow!("{:?}", err))?;
    //     display.clear_buffer();

    //     let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    //     Text::new("Hello Rust!", Point::new(20, 30), style)
    //         .draw(&mut display)
    //         .map_err(|err| anyhow::anyhow!("{:?}", err))?;
    //     display.flush().unwrap();
    // }

    let spi = peripherals.spi3;
    let sda = peripherals.pins.gpio5;
    let sclk = peripherals.pins.gpio18;
    let miso = peripherals.pins.gpio19;
    let mosi = peripherals.pins.gpio23;

    let driver = SpiDriver::new(spi, sclk, mosi, Some(miso), &SpiDriverConfig::new())
        .context("create spi driver")?;
    let config = spi::config::Config::new();
    let device = SpiDeviceDriver::new(&driver, Some(sda), &config)?;

    let spi = SpiInterface::new(device);
    let mut mfrc522 = mfrc522::Mfrc522::new(spi).init()?;

    let version = mfrc522.version()?;

    log::info!("mfrc522 reported version is 0x{:X}", version);
    if !(version == 0x91 || version == 0x92) {
        anyhow::bail!("version mismatch")
    }

    loop {
        const CARD_UID: [u8; 4] = [34, 246, 178, 171];
        const TAG_UID: [u8; 4] = [128, 170, 179, 76];

        if let Ok(atqa) = mfrc522.reqa() {
            if let Ok(uid) = mfrc522.select(&atqa) {
                log::info!("UID: {:?}", uid.as_bytes());

                if uid.as_bytes() == CARD_UID {
                    log::info!("CARD");
                } else if uid.as_bytes() == TAG_UID {
                    log::info!("TAG");
                }

                handle_authenticate(&mut mfrc522, &uid, |m| {
                    let data = m.mf_read(1)?;
                    log::info!("read {:?}", data);
                    Ok(())
                })
                .ok();
            }
        }

        Delay::delay_ms(1000u32);
    }
}

fn handle_authenticate<E, COMM: Interface<Error = E>, F>(
    mfrc522: &mut Mfrc522<COMM, Initialized>,
    uid: &mfrc522::Uid,
    action: F,
) -> anyhow::Result<()>
where
    F: FnOnce(&mut Mfrc522<COMM, Initialized>) -> anyhow::Result<()>,
    E: std::fmt::Debug + std::marker::Sync + std::marker::Send + 'static,
{
    // Use *default* key, this should work on new/empty cards
    let key = [0xFF; 6];
    if mfrc522.mf_authenticate(uid, 1, &key).is_ok() {
        action(mfrc522)?;
    } else {
        log::warn!("Could not authenticate");
    }

    mfrc522.hlta()?;
    mfrc522.stop_crypto1()?;
    Ok(())
}
