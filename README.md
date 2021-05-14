# Rust nRF24L01 driver
This crate provides a platform agnostic Rust driver for the nRF24L01 single chip 2.4 GHz
transceiver by Nordic Semiconduct for communicating data wirelessly using the [`embedded-hal`](https://github.com/rust-embedded/embedded-hal) traits.


## Device
The nRF24L01 transceiver module, manufactured by [Nordic Semiconductor](https://www.nordicsemi.com), is designed to operate in 2.4 GHz worldwide ISM frequency band and uses GFSK modulation for data transmission.
The data transfer rate can be one of 250kbps, 1Mbps and 2Mbps.
#### [Datasheet](https://www.sparkfun.com/datasheets/Components/nRF24L01_prelim_prod_spec_1_2.pdf)

## Usage

This crate can be used by adding `nrf24-rs` to your dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
nrf24-rs = "0.1"
```

## Examples

## Feature-flags

- **micro-fmt:** provides a `uDebug` implementation from the [ufmt crate](https://docs.rs/ufmt) for all public structs and enums.

## Status
### Core functionality
- [x] initialization 
- [x] isChipConnected
- [x] startListening
- [x] stopListening 
- [x] available 
- [x] read 
- [x] write 
- [x] openWritingPipe 
- [x] openReadingPipe 
- [x] allow for multiple reading pipes

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
