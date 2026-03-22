#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::{Duration, Timer};
use embassy_usb::class::hid::{HidWriter, ReportId, RequestHandler, State};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config};
use core::sync::atomic::{AtomicU8, Ordering};
use usbd_hid::descriptor::{KeyboardReport, MediaKeyboardReport, SerializedDescriptor};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

static LEDS: AtomicU8 = AtomicU8::new(0);

struct KeyboardRequestHandler {}

impl RequestHandler for KeyboardRequestHandler {
    fn set_report(&mut self, _id: ReportId, data: &[u8]) -> OutResponse {
        if !data.is_empty() {
            LEDS.store(data[0], Ordering::Relaxed);
        }
        OutResponse::Accepted
    }
}

const PW1: &str = match option_env!("PW1") {
    Some(v) => v,
    None => "pwd1 goes here",
};
const PW2: &str = match option_env!("PW2") {
    Some(v) => v,
    None => "pwd2 goes here",
};
const PW3: &str = match option_env!("PW3") {
    Some(v) => v,
    None => "pwd3 goes here",
};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Pico Macro Keys starting...");

    // USB setup
    let driver = Driver::new(p.USB, Irqs);
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("Pico Macro Keys");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    static DEVICE_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> = static_cell::StaticCell::new();
    let device_descriptor = DEVICE_DESCRIPTOR.init([0; 256]);
    static CONFIG_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> = static_cell::StaticCell::new();
    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    static BOS_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> = static_cell::StaticCell::new();
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    static CONTROL_BUF: static_cell::StaticCell<[u8; 64]> = static_cell::StaticCell::new();
    let control_buf = CONTROL_BUF.init([0; 64]);

    static KBD_STATE: static_cell::StaticCell<State> = static_cell::StaticCell::new();
    let kbd_state = KBD_STATE.init(State::new());

    static MEDIA_STATE: static_cell::StaticCell<State> = static_cell::StaticCell::new();
    let media_state = MEDIA_STATE.init(State::new());

    let mut builder = Builder::new(
        driver,
        config,
        device_descriptor,
        config_descriptor,
        bos_descriptor,
        control_buf,
    );

    // Keyboard HID
    static KBD_HANDLER_CELL: static_cell::StaticCell<KeyboardRequestHandler> = static_cell::StaticCell::new();
    let kbd_handler = KBD_HANDLER_CELL.init(KeyboardRequestHandler {});
    let kbd_config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: Some(kbd_handler),
        poll_ms: 10,
        max_packet_size: 64,
    };
    let mut kbd_writer = HidWriter::<_, 8>::new(&mut builder, kbd_state, kbd_config);

    // Media Key HID
    let media_config = embassy_usb::class::hid::Config {
        report_descriptor: MediaKeyboardReport::desc(),
        request_handler: None,
        poll_ms: 10,
        max_packet_size: 64,
    };
    let mut media_writer = HidWriter::<_, 2>::new(&mut builder, media_state, media_config);

    let mut usb = builder.build();

    // Buttons
    let btn1 = Input::new(p.PIN_18, Pull::Up);
    let btn2 = Input::new(p.PIN_19, Pull::Up);
    let btn3 = Input::new(p.PIN_20, Pull::Up);
    let btn4 = Input::new(p.PIN_21, Pull::Up);
    let btn5 = Input::new(p.PIN_22, Pull::Up);

    // LED
    let mut led = Output::new(p.PIN_25, Level::Low);

    // Encoder
    let rot_a = Input::new(p.PIN_26, Pull::Up);
    let rot_b = Input::new(p.PIN_27, Pull::Up);

    let usb_fut = usb.run();

    let mut last_rot_a = rot_a.is_low();
    let mut last_rot_b = rot_b.is_low();

    let main_fut = async {
        loop {
            // Button 1
            if btn1.is_low() {
                led.set_high();
                send_string(&mut kbd_writer, PW1).await;
                led.set_low();
                Timer::after(Duration::from_millis(200)).await;
            }

            // Button 2
            if btn2.is_low() {
                send_string(&mut kbd_writer, PW2).await;
                Timer::after(Duration::from_millis(200)).await;
            }

            // Button 3
            if btn3.is_low() {
                send_string(&mut kbd_writer, PW3).await;
                Timer::after(Duration::from_millis(200)).await;
            }

            // Button 4: Ctrl+Shift+F9
            if btn4.is_low() {
                send_keys(&mut kbd_writer, 0x01 | 0x02, &[0x42]).await; // 0x01=Ctrl, 0x02=Shift, 0x42=F9
                Timer::after(Duration::from_millis(200)).await;
            }

            // Button 5: Ctrl+Shift+F8
            if btn5.is_low() {
                send_keys(&mut kbd_writer, 0x01 | 0x02, &[0x41]).await; // 0x41=F8
                Timer::after(Duration::from_millis(200)).await;
            }

            // Encoder logic
            let curr_rot_a = rot_a.is_low();
            let curr_rot_b = rot_b.is_low();
            if curr_rot_a != last_rot_a || curr_rot_b != last_rot_b {
                // Simple state machine for encoder
                if last_rot_a && !curr_rot_a {
                    if curr_rot_b {
                        // CW
                        send_media_key(&mut media_writer, 0xEA).await; // Volume Up
                    } else {
                        // CCW
                        send_media_key(&mut media_writer, 0xE9).await; // Volume Down
                    }
                }
                last_rot_a = curr_rot_a;
                last_rot_b = curr_rot_b;
            }

            Timer::after(Duration::from_millis(1)).await;
        }
    };

    embassy_futures::join::join(usb_fut, main_fut).await;
}

async fn send_string<'d, W: embassy_usb::driver::Driver<'d>>(writer: &mut HidWriter<'d, W, 8>, s: &str) {
    let leds = LEDS.load(Ordering::Relaxed);
    for c in s.chars() {
        let (mods, key) = char_to_keycode(c, leds);
        send_keys(writer, mods, &[key]).await;
        Timer::after(Duration::from_millis(10)).await;
    }
}

async fn send_keys<'d, W: embassy_usb::driver::Driver<'d>>(writer: &mut HidWriter<'d, W, 8>, modifier: u8, keys: &[u8]) {
    let mut keycodes = [0u8; 6];
    for (i, &key) in keys.iter().enumerate().take(6) {
        keycodes[i] = key;
    }
    let report = KeyboardReport {
        modifier,
        reserved: 0,
        leds: 0,
        keycodes,
    };
    let _ = writer.write_serialize(&report).await;

    // Release keys
    let report = KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0u8; 6],
    };
    let _ = writer.write_serialize(&report).await;
}

async fn send_media_key<'d, W: embassy_usb::driver::Driver<'d>>(writer: &mut HidWriter<'d, W, 2>, usage_id: u16) {
    let report = MediaKeyboardReport { usage_id };
    let _ = writer.write_serialize(&report).await;

    // Release
    let report = MediaKeyboardReport { usage_id: 0 };
    let _ = writer.write_serialize(&report).await;
}

fn char_to_keycode(c: char, leds: u8) -> (u8, u8) {
    let caps_lock = (leds & 0x02) != 0;
    match c {
        'a'..='z' => {
            let modifier = if caps_lock { 0x02 } else { 0 };
            (modifier, 0x04 + (c as u8 - b'a'))
        }
        'A'..='Z' => {
            let modifier = if caps_lock { 0 } else { 0x02 };
            (modifier, 0x04 + (c as u8 - b'A'))
        }
        '1'..='9' => (0, 0x1E + (c as u8 - b'1')),
        '0' => (0, 0x27),
        ' ' => (0, 0x2C),
        '!' => (0x02, 0x1E), // Shift + 1
        '@' => (0x02, 0x1F), // Shift + 2
        '#' => (0x02, 0x20), // Shift + 3
        '$' => (0x02, 0x21), // Shift + 4
        '%' => (0x02, 0x22), // Shift + 5
        '^' => (0x02, 0x23), // Shift + 6
        '&' => (0x02, 0x24), // Shift + 7
        '*' => (0x02, 0x25), // Shift + 8
        '(' => (0x02, 0x26), // Shift + 9
        ')' => (0x02, 0x27), // Shift + 0
        '-' => (0, 0x2D),
        '_' => (0x02, 0x2D),
        '=' => (0, 0x2E),
        '+' => (0x02, 0x2E),
        '[' => (0, 0x2F),
        '{' => (0x02, 0x2F),
        ']' => (0, 0x30),
        '}' => (0x02, 0x30),
        '\\' => (0, 0x31),
        '|' => (0x02, 0x31),
        ';' => (0, 0x33),
        ':' => (0x02, 0x33),
        '\'' => (0, 0x34),
        '"' => (0x02, 0x34),
        '`' => (0, 0x35),
        '~' => (0x02, 0x35),
        ',' => (0, 0x36),
        '<' => (0x02, 0x36),
        '.' => (0, 0x37),
        '>' => (0x02, 0x37),
        '/' => (0, 0x38),
        '?' => (0x02, 0x38),
        _ => (0, 0),
    }
}
