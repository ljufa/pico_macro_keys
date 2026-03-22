# Pico Macro Keys

A custom USB macro keyboard firmware for the Raspberry Pi Pico (RP2040), built with Rust and the Embassy asynchronous framework. It acts as a USB HID keyboard and media controller.

## Features

- **USB HID Keyboard:** Acts as a standard USB keyboard.
- **USB HID Media Controller:** Acts as a standard USB media controller for volume control.
- **5 Configurable Buttons:**
  - **Button 1 (Pin 18):** Types a string (configured via `PW1`), flashes LED.
  - **Button 2 (Pin 19):** Types a string (configured via `PW2`).
  - **Button 3 (Pin 20):** Types a string (configured via `PW3`).
  - **Button 4 (Pin 21):** Sends `Ctrl+Shift+F9`.
  - **Button 5 (Pin 22):** Sends `Ctrl+Shift+F8`.
- **Rotary Encoder (Pins 26/27):** Controls system volume (Volume Up/Down).
- **Caps Lock Awareness:** Correctly parses typed strings based on host Caps Lock state.

## Hardware Setup

* **Raspberry Pi Pico (RP2040)**
* **Buttons:** Connect to Pins 18, 19, 20, 21, and 22. Connect the other side of the buttons to Ground. Internal pull-ups are used.
* **Rotary Encoder:** Connect Pin A to Pin 26, Pin B to Pin 27. Connect the common pin to Ground. Internal pull-ups are used.
* **LED:** Connect an LED (with a suitable resistor) to Pin 25.

## Configuration

Passwords or macro strings for Buttons 1-3 are configured at compile-time using environment variables.

Create a `.env` file in the root of the project:

```sh
PW1="your_first_string"
PW2="your_second_string"
PW3="your_third_string"
```

If not provided, they default to fallback strings in the code.

## Building and Flashing

1. **Install Prerequisites:**
   Make sure you have Rust installed and the appropriate target for the Pico:
   ```sh
   rustup target add thumbv6m-none-eabi
   cargo install probe-rs --features cli
   ```

2. **Build the project:**
   ```sh
   cargo build --release
   ```

3. **Flash:**
   You can flash it using `probe-rs` if you have a debug probe connected:
   ```sh
   probe-rs download --chip RP2040 target/thumbv6m-none-eabi/release/pico-macro-keys
   ```
   Or convert to `.uf2` and drag-and-drop onto the Pico in BOOTSEL mode.

## License

MIT License
