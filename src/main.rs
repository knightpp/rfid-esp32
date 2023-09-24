use anyhow::Context;
use esp_idf_hal::{
    delay,
    prelude::Peripherals,
    spi::{self, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
    units::MegaHertz,
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use mfrc522::comm::eh02::spi::SpiInterface;

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

    let spi = peripherals.spi2;
    let sda = peripherals.pins.gpio2;
    let sclk = peripherals.pins.gpio18;
    let mosi = peripherals.pins.gpio23;
    let miso = peripherals.pins.gpio19;

    log::debug!("create driver...");
    let driver = SpiDriver::new(spi, sclk, miso, Some(mosi), &SpiDriverConfig::new())
        .context("create spi driver")?;
    let config = spi::config::Config::new().baudrate(MegaHertz(1).into());
    let device = SpiDeviceDriver::new(&driver, Some(sda), &config)?;

    log::debug!("create mfrc522...");
    let spi = SpiInterface::new(device).with_delay(|| delay::Ets::delay_us(50));
    let mut mfrc522 = mfrc522::Mfrc522::new(spi).init().context("init mfrc522")?;

    log::debug!("get mfrc522 version...");
    let version = mfrc522.version()?;

    log::info!("mfrc522 reported version is 0x{:X}", version);
    if !(version == 0x91 || version == 0x92) {
        log::error!("version mismatch!")
    }

    Ok(())
}
