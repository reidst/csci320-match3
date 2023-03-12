#![no_std]
#![no_main]

mod vga_buffer;
mod serial;

use lazy_static::lazy_static;
use pc_keyboard::DecodedKey;
use csci320_match3::HandlerTable;
use spin::Mutex;
use vga_buffer::{plot, ColorCode, Color};

const COLUMN_GAP: usize = 2;
const GREEN_SHIFT_SPEED: usize = 4;

lazy_static! {
    static ref RAND: Mutex<Random> = Mutex::new(Random::new(1234));
}
lazy_static! {
    static ref TICK: Mutex<u64> = Mutex::new(0);
}
lazy_static! {
    static ref COLOR_BARS: Mutex<[usize; vga_buffer::BUFFER_WIDTH]> = 
        Mutex::new([(vga_buffer::BUFFER_HEIGHT/2) as usize; vga_buffer::BUFFER_WIDTH]);
}

fn start() {
    // println!("Hello, world!");
}

fn tick() {
    for row in 0..vga_buffer::BUFFER_HEIGHT {
        for col in 0..vga_buffer::BUFFER_WIDTH {
            let character = RAND.lock().range(48,58) as u8 as char;
            let number_color = if col % COLUMN_GAP != 0 {
                Color::Black
            } else if row < COLOR_BARS.lock()[col] {
                Color::Green
            } else {
                Color::DarkGray
            };
            plot(character, col, row, ColorCode::new(number_color, Color::Black))
        }
    }
    for _ in 0..GREEN_SHIFT_SPEED {
        let col = RAND.lock().range(0, vga_buffer::BUFFER_WIDTH as u64) as usize;
        let old = COLOR_BARS.lock()[col];
        if RAND.lock().next() %2 == 0 {
            COLOR_BARS.lock()[col] = if old >= vga_buffer::BUFFER_HEIGHT - 1 { old } else { old + 1};
        } else {
            COLOR_BARS.lock()[col] = if old <= 1 { old } else { old - 1};
        }
    }
    *TICK.lock() += 1;
}

fn key(_key: DecodedKey) {
    // match key {
    //     DecodedKey::Unicode(character) => print!("{}", character),
    //     DecodedKey::RawKey(key) => print!("{:?}", key),
    // }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    HandlerTable::new()
        .keyboard(key)
        .timer(tick)
        .startup(start)
        .start()
}


/// A simple 64-bit xorshift. Source: https://en.wikipedia.org/wiki/Xorshift#Example_implementation
struct Random {
    _state: u64
}

impl Random {
    fn new(seed: u64) -> Self {
        Self { _state: seed }
    }
    fn next(&mut self) -> u64 {
        let mut x = self._state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self._state = x;
        x
    }
    fn range(&mut self, min: u64, max: u64) -> u64 {
        let range = max - min;
        let limit = u64::MAX / range * range;
        let mut candidate = self.next();
        while candidate >= limit {
            candidate = self.next();
        }
        candidate - (candidate / range * range) + min
    }
}
