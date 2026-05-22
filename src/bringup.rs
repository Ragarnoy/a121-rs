//! Low-level ASIC register access over SPI (bring-up / diagnostics).
//!
//! Framing matches the Acconeer RSS library (`addr | 0x3000` read, `addr | 0x1000` write).
//! This is intentionally small and does not depend on `device-driver`.

/// SPI read command flag (OR with 16-bit register address).
pub const READ_FLAG: u16 = 0x3000;
/// SPI write command flag (OR with 16-bit register address).
pub const WRITE_FLAG: u16 = 0x1000;

/// ASIC identity register (communication / bring-up test).
pub const REG_ASIC_ID: u16 = 0x004D;
/// Run status register.
pub const REG_RUN_STATUS: u16 = 0x0062;
/// Wakeup status 0.
pub const REG_WAKEUP_STATUS0: u16 = 0x007C;
/// Interrupt status register.
pub const REG_INTERRUPT_STATUS: u16 = 0x0098;
/// Scratchpad configuration 0.
pub const REG_SCRATCHPAD_CONFIG0: u16 = 0x0037;
/// Stack level status.
pub const REG_STACK_LEVEL_STATUS: u16 = 0x001C;

/// Errors during register-level SPI access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// SPI transfer failed.
    Spi,
}

/// In-place SPI transfer (chip select held for the whole buffer).
pub trait SpiTransfer {
    /// Full-duplex transfer used for register transactions.
    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Error>;
}

/// Read a 16-bit ASIC register.
pub fn read_register<S: SpiTransfer>(spi: &mut S, address: u16) -> Result<u16, Error> {
    let header = address | READ_FLAG;
    let mut buf = [0u8; 6];
    buf[0..2].copy_from_slice(&header.to_le_bytes());
    spi.transfer_in_place(&mut buf)?;
    Ok(u16::from_le_bytes([buf[4], buf[5]]))
}

/// Write a 16-bit ASIC register.
pub fn write_register<S: SpiTransfer>(spi: &mut S, address: u16, value: u16) -> Result<(), Error> {
    let header = address | WRITE_FLAG;
    let mut buf = [0u8; 4];
    buf[0..2].copy_from_slice(&header.to_le_bytes());
    buf[2..4].copy_from_slice(&value.to_le_bytes());
    spi.transfer_in_place(&mut buf)
}

/// Read the ASIC ID register (sanity check after enable + SPI wiring).
pub fn read_asic_id<S: SpiTransfer>(spi: &mut S) -> Result<u16, Error> {
    read_register(spi, REG_ASIC_ID)
}

/// Adapter for `fn(&mut [u8]) -> Result<(), E>` callbacks (e.g. shared with [`crate::hal::AccHalImpl::new_with_transfer`]).
pub struct FnSpiTransfer<F>(pub F);

impl<F> SpiTransfer for FnSpiTransfer<F>
where
    F: FnMut(&mut [u8]) -> Result<(), Error>,
{
    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Error> {
        (self.0)(words)
    }
}
