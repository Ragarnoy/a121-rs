# STM32WBA65RI bring-up

Minimal Embassy example: enable sensor, register RSS HAL, read ASIC ID register.

## Wiring (example — adjust for your board)

| Signal  | MCU pin |
|---------|---------|
| SPI_SCK | PB3     |
| SPI_MOSI| PB5     |
| SPI_MISO| PB4     |
| SPI_CS  | PA8     |
| SEN_EN  | PA9     |
| SEN_INT | PB10    |

## Build

Requires Acconeer static libraries for Cortex-M33:

```bash
export A121_RSS_LIB=/path/to/rss/lib
cargo build --release
```

Headers come from the sibling `a121-sys` crate (`rss/include/`).
