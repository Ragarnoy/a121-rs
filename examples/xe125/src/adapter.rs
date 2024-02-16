use core::convert::Infallible;

use embassy_stm32::spi;
use embedded_hal::spi::{Error, ErrorKind, ErrorType, Operation, SpiDevice};
use embedded_hal_bus::spi::DeviceError;

pub struct SpiAdapter<SPI>
    where
        SPI: SpiDevice<u16>,
{
    spi: SPI,
}

impl<SPI> SpiAdapter<SPI>
    where
        SPI: SpiDevice<u16>,
{
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }
}

impl<SPI> ErrorType for SpiAdapter<SPI>
    where
        SPI: SpiDevice<u16>,
{
    type Error = ErrorKind;
}

impl<SPI> SpiDevice<u16> for SpiAdapter<SPI>
    where
        SPI: SpiDevice<u16, Error = DeviceError<spi::Error, Infallible>>,
{
    fn transaction(&mut self, operations: &mut [Operation<'_, u16>]) -> Result<(), Self::Error> {
        self.spi.transaction(operations).map_err(|e| e.kind())
    }

    fn read(&mut self, words: &mut [u16]) -> Result<(), ErrorKind> {
        self.spi.read(words).map_err(|e| e.kind())
    }

    fn write(&mut self, words: &[u16]) -> Result<(), ErrorKind> {
        self.spi.write(words).map_err(|e| e.kind())
    }

    fn transfer(&mut self, read: &mut [u16], write: &[u16]) -> Result<(), ErrorKind> {
        self.spi.transfer(read, write).map_err(|e| e.kind())
    }

    fn transfer_in_place(&mut self, words: &mut [u16]) -> Result<(), ErrorKind> {
        self.spi.transfer_in_place(words).map_err(|e| e.kind())
    }
}
