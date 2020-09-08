#![no_std]
use core::fmt::Debug;
use embedded_hal as hal;
use hal::blocking::{
    delay::DelayMs,
    spi::{Transfer, Write},
};
use hal::{digital::v2::OutputPin, spi};

pub mod config;
pub mod register;
pub mod status;

use config::{DataPipe, DataRate, EncodingScheme, PALevel};
use register::Register;
use status::{FIFOStatus, Status};

/// SPI mode
pub const MODE: spi::Mode = spi::MODE_0;

const MAX_PAYLOAD_SIZE: u8 = 32;

/// Error
#[derive(Debug)]
pub enum TransmissionError<E, F> {
    /// TODO add error types
    ///
    /// SPI error
    Spi(E),
    /// Pin error
    Pin(F),
    /// Communication error with module
    CommunicationError(u8),
}

#[derive(Debug)]
pub struct NoDataAvailable;

pub struct Nrf24l01<SPI, CE, NCS> {
    spi: SPI,
    ncs: NCS,
    ce: CE,
    config_reg: u8,
    payload_size: u8,
}

type Result<T, E, F> = core::result::Result<T, TransmissionError<E, F>>;

impl<SPI, CE, NCS, SPIErr, PinErr> Nrf24l01<SPI, CE, NCS>
where
    SPI: Transfer<u8, Error = SPIErr> + Write<u8, Error = SPIErr>,
    NCS: OutputPin<Error = PinErr>,
    CE: OutputPin<Error = PinErr>,
{
    const MAX_ADDR_WIDTH: usize = 5;

    pub fn new<D>(
        spi: SPI,
        ce: CE,
        ncs: NCS,
        delay: &mut D,
        payload_size: u8,
    ) -> Result<Self, SPIErr, PinErr>
    where
        D: DelayMs<u8>,
    {
        let mut chip = Nrf24l01 {
            spi,
            ncs,
            ce,
            config_reg: 0,
            payload_size: 0,
        };

        chip.set_payload_size(payload_size);

        // Set the output pins to the correct levels
        chip.ce.set_low().map_err(TransmissionError::Pin)?;
        chip.ncs.set_high().map_err(TransmissionError::Pin)?;

        // Must allow the radio time to settle else configuration bits will not necessarily stick.
        // This is actually only required following power up but some settling time also appears to
        // be required after resets too. For full coverage, we'll always assume the worst.
        // Enabling 16b CRC is by far the most obvious case if the wrong timing is used - or skipped.
        // Technically we require 4.5ms + 14us as a worst case. We'll just call it 5ms for good measure.
        delay.delay_ms(5);

        // Set retries
        chip.set_retries(5, 15)?;
        // Set rf
        chip.setup_rf(DataRate::default(), PALevel::default())?;
        // Reset status
        chip.write_register(Register::STATUS, 0b01111110)?;
        // Set up default configuration.  Callers can always change it later.
        // This channel should be universally safe and not bleed over into adjacent spectrum.
        chip.set_channel(76)?;
        // flush buffers
        chip.flush_rx()?;
        chip.flush_tx()?;

        // clear CONFIG register, Enable PTX, Power Up & 16-bit CRC
        chip.enable_crc(EncodingScheme::R2Bytes)?;

        chip.config_reg = chip.read_register(Register::CONFIG)?;

        chip.power_up(delay)?;

        if chip.config_reg != 0b00001110 {
            Err(TransmissionError::CommunicationError(chip.config_reg))
        } else {
            Ok(chip)
        }
    }

    /// Power up now.
    ///
    /// # Examples
    /// ```rust
    /// chip.power_up(&mut delay)?;
    /// ```
    pub fn power_up<D>(&mut self, delay: &mut D) -> Result<(), SPIErr, PinErr>
    where
        D: DelayMs<u8>,
    {
        // if not powered up, power up and wait for the radio to initialize
        if !self.is_powered_up() {
            self.config_reg |= 1 << 1;
            self.write_register(Register::CONFIG, self.config_reg)?;

            delay.delay_ms(5);
        }
        Ok(())
    }

    /// Check whether there are any bytes available to be read.
    ///
    /// When data is available, this function returns the data pipe where the data can be read.
    /// If no data is available in any of the pipes, it returns `Err(NoDataAvailable)`
    pub fn available(
        &mut self,
    ) -> Result<core::result::Result<DataPipe, NoDataAvailable>, SPIErr, PinErr> {
        let fifo_status = self
            .read_register(Register::FIFO_STATUS)
            .map(FIFOStatus::from)?;

        if !fifo_status.rx_empty() {
            return self.status().map(|s| Ok(s.data_pipe()));
        }

        Ok(Err(NoDataAvailable))
    }

    /// Read the available payload
    pub fn read(&mut self, buf: &mut [u8], len: usize) -> Result<(), SPIErr, PinErr> {
        Ok(())
    }

    pub fn open_writing_pipe(&mut self, mut addr: &[u8]) -> Result<(), SPIErr, PinErr> {
        if addr.len() > Self::MAX_ADDR_WIDTH {
            addr = &addr[0..Self::MAX_ADDR_WIDTH - 1];
        }
        self.write_register(register, value)
        Ok(())
    }

    /// Setup of automatic retransmission.
    ///
    /// # Arguments
    /// * `delay` is the auto retransmit delay.
    /// Values can be between 0 and 15.
    /// The delay before a retransmit is initiated, is calculated according to the following formula:
    /// > ((**delay** + 1) * 250) + 86 µs
    ///
    /// * `count` is number of times there will be an auto retransmission.
    /// Must be a value between 0 and 15.
    ///
    /// # Examples
    /// ```rust
    /// // Set the auto transmit delay to (5 + 1) * 250) + 86 = 1586µs
    /// // and the retransmit count to 15.
    /// nrf24l01.set_retries(5, 15)?;
    /// ```
    pub fn set_retries(&mut self, delay: u8, count: u8) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::SETUP_RETR, (delay << 4) | (count))
    }

    /// Set the frequency channel nRF24L01 operates on.
    ///
    /// # Arguments
    ///
    /// * `channel` number between 0 and 127.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.set_channel(73)?;
    /// ```
    pub fn set_channel(&mut self, channel: u8) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::RF_CH, (0xf >> 1) & channel)
    }

    /// Flush transmission FIFO, used in TX mode.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.flush_tx()?;
    /// ```
    pub fn flush_tx(&mut self) -> Result<(), SPIErr, PinErr> {
        self.send_command(Instruction::FTX).map(|_| ())
    }

    /// Flush reciever FIFO, used in RX mode.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.flush_rx()?;
    /// ```
    pub fn flush_rx(&mut self) -> Result<(), SPIErr, PinErr> {
        self.send_command(Instruction::FRX).map(|_| ())
    }

    /// Enable CRC encoding scheme.
    ///
    /// **Note** that this configures the nrf24l01 in transmit mode.
    ///
    /// # Examples
    /// ```rust
    /// chip.enable_crc(EncodingScheme::R2Bytes)?;
    /// ```
    pub fn enable_crc(&mut self, scheme: EncodingScheme) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::CONFIG, (1 << 3) | (scheme.scheme() << 2))
    }

    /// Configure the data rate and PA level.
    pub fn configure(&mut self, data_rate: DataRate, level: PALevel) -> Result<(), SPIErr, PinErr> {
        self.setup_rf(data_rate, level)
    }

    /// Set the payload size.
    pub fn set_payload_size(&mut self, payload_size: u8) {
        self.payload_size = core::cmp::min(MAX_PAYLOAD_SIZE, payload_size);
    }

    /// Read status from device.
    pub fn status(&mut self) -> Result<Status, SPIErr, PinErr> {
        self.send_command(Instruction::NOP)
    }

    fn send_command(&mut self, instruction: Instruction) -> Result<Status, SPIErr, PinErr> {
        let mut buffer = [instruction.opcode()];
        self.ncs.set_low().map_err(TransmissionError::Pin)?;
        let r = self
            .spi
            .transfer(&mut buffer)
            .map_err(TransmissionError::Spi);
        self.ncs.set_high().map_err(TransmissionError::Pin)?;
        r.map(|s| Status::from(s[0]))
    }

    fn write_register(&mut self, register: Register, value: u8) -> Result<(), SPIErr, PinErr> {
        let buffer = [Instruction::WR.opcode() | register.addr(), value];
        self.ncs.set_low().map_err(TransmissionError::Pin)?;
        self.spi.write(&buffer).map_err(TransmissionError::Spi)?;
        self.ncs.set_high().map_err(TransmissionError::Pin)?;

        Ok(())
    }

    fn read_register(&mut self, register: Register) -> Result<u8, SPIErr, PinErr> {
        let mut buffer = [Instruction::RR.opcode() | register.addr(), 0];
        self.ncs.set_low().map_err(TransmissionError::Pin)?;
        self.spi
            .transfer(&mut buffer)
            .map_err(TransmissionError::Spi)?;
        self.ncs.set_high().map_err(TransmissionError::Pin)?;
        Ok(buffer[1])
    }

    fn setup_rf(&mut self, data_rate: DataRate, level: PALevel) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::RF_SETUP, data_rate.rate() | level.level())
    }

    fn is_powered_up(&self) -> bool {
        self.config_reg & (1 << 1) != 0
    }
}

#[derive(Clone, Copy)]
enum Instruction {
    /// Read registers
    RR = 0b0000_0000,
    /// Write registers
    /// Last 5 bits are the Memory Map Adress
    WR = 0b0010_0000,
    /// Read RX-payload, used in RX mode.
    RRX = 0b0110_0001,
    /// Write TX-payload, used in TX mode.
    WTX = 0b1010_0000,
    /// Flush TX FIFO, used in TX mode.
    FTX = 0b1110_0001,
    /// Flush RX FIFO, used in RX mode.
    FRX = 0b1110_0010,
    /// No operation. Might be used to read STATUS register.
    NOP = 0b1111_1111,
}

impl Instruction {
    pub(crate) fn opcode(&self) -> u8 {
        *self as u8
    }
}
