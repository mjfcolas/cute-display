# The hardware

The Habity bedside clock, as recovered by reverse engineering (pin matrix read over the
USB JTAG from the running stock firmware, then confirmed by probing) and as measured on
this project's unit.

## Pin map

| Function                    | GPIO                    | Peripheral / notes                                              |
| --------------------------- | ----------------------- | --------------------------------------------------------------- |
| Panel SCLK / MOSI / CS      | 12 / 11 / 10            | SPI2, mode 0, 10 MHz, no MISO                                   |
| Panel DC / RST / BUSY       | 9 / 16 / 4              | BUSY is low while the controller works                          |
| Wheel A / B                 | 47 / 48                 | PCNT quadrature, 2 counts per detent                            |
| Wheel press                 | 21                      | switch to ground, pull-up                                       |
| Yellow button               | 15                      | switch to ground, pull-up                                       |
| Long button                 | 44                      | switch to ground, pull-up. Also UART0 RX: never use UART0       |
| I2C SDA / SCL               | 45 / 46                 | 100 kHz. Only device: DS3231 at 0x68                            |
| DS3231 INT                  | 3                       | open drain, pulled up on the board                              |
| I2S BCLK / WS / DOUT        | 40 / 39 / 38            | Philips, 16-bit stereo, 44.1 kHz, no MCLK                       |
| Amplifier enable            | 41                      | high = on                                                       |
| Reading lamp                | 17                      | LEDC, 1 kHz, 10-bit                                             |
| Front light (over the panel)| 18                      | LEDC, 1 kHz, 10-bit                                             |
| SD CMD / CLK / D0–D3        | 5 / 6 / 7, 8, 14, 13    | SDMMC 4-bit, FAT. The card does not come out of the case. It holds the stock sounds |
| USB present                 | 1                       | ~3.2 V plugged, tens of mV not, through ~340 kΩ: read floating  |
| Battery sense               | 2                       | ADC1 channel 1, divider with ~31 nF; ratio unknown (~0.9 V seen) |

The console is the USB Serial/JTAG (`303a:1001`); there is no reachable UART.

## Parts

- **MCU**: ESP32-S3, 16 MB flash, 8 MB octal PSRAM in the package (eFuse `PSRAM_CAP`).
- **Panel**: GDEY037T03, 416 × 240, controller **UC8253** (a UC8xxx, not an SSD16xx).
  The case hides columns 398 to 415.
- **RTC**: DS3231, battery-backed. The stock firmware arms alarm 1 daily from settings it
  keeps in NVS; once it fires, INT (GPIO 3) stays low until the flag is cleared. Its die
  temperature sensor is the board's only thermometer.
- **Amplifier**: part unknown, fixed gain, loud: a pure tone at 25 % of full scale is
  already unpleasant. Volume is digital scaling.
- **Battery**: the device runs for hours unplugged; no fuel gauge, only the sense pad.

## The panel controller

Measured, no datasheet. What the driver relies on is written beside the code, in
[`src/drivers/src/uc8253/`](../src/drivers/src/uc8253/). Clean full refresh ≈ 2.6 s (up
to 6.9 s cold); fast partial ≈ 350 ms.

## Flash layout of this unit

| Partition | Offset   | Size    | Content                                             |
| --------- | -------- | ------- | --------------------------------------------------- |
| nvs       | 0x9000   | 64 KB   | stock Wi-Fi credentials                             |
| otadata   | 0x19000  | 8 KB    | which app boots; stock copy in `backup/`            |
| phy_init  | 0x1b000  | 20 KB   |                                                     |
| factory   | 0x20000  | 4 MB    | stock Habity v1.1.0                                 |
| app0      | 0x420000 | 4 MB    | stock Habity v1.1.1 (OTA), what the device ran      |
| app1      | 0x820000 | 4 MB    | **cute-display**                                    |
| coredump  | 0xc20000 | 3.9 MB  |                                                     |
