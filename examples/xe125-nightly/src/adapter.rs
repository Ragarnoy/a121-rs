use core::convert::Infallible;

use embassy_stm32::spi;
use embedded_hal::spi::{Error, ErrorKind, ErrorType, Operation, SpiDevice};
use embedded_hal_bus::spi::DeviceError;

pub struct SpiAdapter<SPI>
    where
        SPI: SpiDevice<u8>,
{
    spi: SPI,
}

impl<SPI> SpiAdapter<SPI>
    where
        SPI: SpiDevice<u8>,
{
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }
}

impl<SPI> ErrorType for SpiAdapter<SPI>
    where
        SPI: SpiDevice<u8>,
{
    type Error = ErrorKind;
}

impl<SPI> SpiDevice<u8> for SpiAdapter<SPI>
    where
        SPI: SpiDevice<u8, Error = DeviceError<spi::Error, Infallible>>,
{
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        self.spi.transaction(operations).map_err(|e| e.kind())
    }

    fn read(&mut self, words: &mut [u8]) -> Result<(), ErrorKind> {
        self.spi.read(words).map_err(|e| e.kind())
    }

    fn write(&mut self, words: &[u8]) -> Result<(), ErrorKind> {
        self.spi.write(words).map_err(|e| e.kind())
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), ErrorKind> {
        self.spi.transfer(read, write).map_err(|e| e.kind())
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), ErrorKind> {
        self.spi.transfer_in_place(words).map_err(|e| e.kind())
    }
}
