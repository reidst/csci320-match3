#![no_std]
#![no_main]

mod vga_buffer;
mod serial;
mod match3game;

use lazy_static::lazy_static;
use match3game::{GameState, GameStateManager};
use pc_keyboard::DecodedKey;
use csci320_match3::HandlerTable;
use spin::Mutex;
use vga_buffer::{plot, plot_num_right_justified, plot_str, clear_row, ColorCode, Color};

lazy_static! {
    static ref TICK: Mutex<u64> = Mutex::new(0);
}
lazy_static! {
    static ref GAME: Mutex<GameStateManager> = Mutex::new(GameStateManager::new());
}


fn start() {
}

fn tick() {
    *TICK.lock() += 1;
    let gsm = &mut *GAME.lock();
    let tick = *TICK.lock();
    gsm.tick(tick);
    // draw
    match gsm.get_state() {
        GameState::EnteringCode => {
            const FLASH_PERIOD: u64 = 10;
            if tick % FLASH_PERIOD == 0 {
                draw_logo(tick / FLASH_PERIOD);
            }
            draw_code_menu(gsm);
        },
        GameState::Playing => draw_game(gsm)
    }
}

fn key(key: DecodedKey) {
    let gsm = &mut *GAME.lock();
    let old_state = gsm.get_state();
    gsm.key(key);
    if gsm.get_state() != old_state {
        vga_buffer::clear_screen();
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    HandlerTable::new()
        .keyboard(key)
        .timer(tick)
        .startup(start)
        .start()
}

fn draw_logo(tick: u64) {
    const LOGO_HEIGHT: usize = 4;
    const LOGO_LENGTH: usize = 41;
    const LOGO_DRAW_ROW: usize = 5;
    const LOGO_DRAW_COL: usize = (vga_buffer::BUFFER_WIDTH - LOGO_LENGTH) / 2;
    const LETTER_OFFSETS: [usize; 8] = [0, 8, 14, 19, 23, 31, 38, LOGO_LENGTH];
    const LETTER_COLORS: [u8; 7] = [9, 10, 11, 12, 13, 14, 15];
    // ASCII art generated from https://texteditor.com/ascii-art/ using the "Meh" font
    const LETTERS: &str = r" __  __        _        _       ____   _ |  \/  | __ _ | |_  __ | |_    |__ /  | || |\/| |/ _` ||  _|/ _||   \    |_ \  |_||_|  |_|\__/_| \__|\__||_||_|  |___/  (_)";
    for letter in 0..LETTER_OFFSETS.len()-1 {
        let chosen_color = LETTER_COLORS[(letter + tick as usize) % LETTER_COLORS.len()];
        for row in 0..LOGO_HEIGHT {
            let letter_start = LOGO_LENGTH * row + LETTER_OFFSETS[letter];
            let letter_end = LOGO_LENGTH * row + LETTER_OFFSETS[letter + 1];
            plot_str(
                &LETTERS[letter_start..letter_end], 
                LOGO_DRAW_COL + LETTER_OFFSETS[letter], 
                LOGO_DRAW_ROW + row, 
                ColorCode::new(Color::from(chosen_color), Color::Black)
            );
        }
    }
}

fn draw_code_menu(gsm: &GameStateManager) {
    const INPUT_HEIGHT: usize = 19;
    plot_str("Enter a code:", 33, INPUT_HEIGHT, ColorCode::new(Color::White, Color::Black));
    clear_row(INPUT_HEIGHT+1, Color::Black);
    let code = gsm.get_code();
    let mut write_pos = (vga_buffer::BUFFER_WIDTH - gsm.get_code_len()) / 2;
    for c in 0..gsm.get_code_len() {
        plot(code[c], write_pos, INPUT_HEIGHT+1, ColorCode::new(Color::Yellow, Color::Black));
        write_pos += 1;
    }
}

fn draw_game(gsm: &GameStateManager) {
    // board
    const DRAW_COL_OFFSET: usize = 20;
    const DRAW_ROW_OFFSET: usize = 0;
    const SELECT_BLINK_PERIOD: u64 = 4;
    let g = gsm.get_game();
    let board = g.get_board();
    for col in 0..match3game::BOARD_WIDTH {
        for row in 0..match3game::BOARD_HEIGHT {
            let current = board[col][row];
            let draw_col = col * 5 + DRAW_COL_OFFSET;
            let draw_row = row * 3 + DRAW_ROW_OFFSET;
            let highlight = if g.get_cursor().location() == (col, row) {
                Color::DarkGray
            } else {
                Color::Black
            };
            if current == 0 {
                draw_empty(draw_col, draw_row, highlight);
            } else {
                let color = Color::from(current + if g.is_alive() { 8 } else { 0 });
                let selected = g.get_cursor().location() == (col, row) 
                    && g.is_selected() 
                    && *TICK.lock() % (SELECT_BLINK_PERIOD * 2) < SELECT_BLINK_PERIOD;
                draw_gem(draw_col, draw_row, color, highlight, selected);
            }
        }
    }
    // score
    let ui_code = ColorCode::new(Color::White, Color::DarkGray);
    let msg = if g.is_alive() { "Score: " } else {"Game Over! Final Score:" };
    plot_num_right_justified(
        match3game::BOARD_WIDTH*5-1-msg.len(),  // width of baord in chars
        g.get_score() as isize * 100, 
        DRAW_COL_OFFSET+msg.len(),              // start at end of msg
        vga_buffer::BUFFER_HEIGHT-1,            // bottom row
        ui_code
    );
    plot_str(msg, DRAW_COL_OFFSET, vga_buffer::BUFFER_HEIGHT-1, ui_code);
    // outline
    for row in 0..vga_buffer::BUFFER_HEIGHT {
        plot(' ', DRAW_COL_OFFSET - 1, row, ui_code);
        plot(' ', DRAW_COL_OFFSET + 39, row, ui_code);
    }
}

fn draw_gem(c: usize, r: usize, color: Color, highlight: Color, selected: bool) {
    let code = ColorCode::new(color, highlight);
    let inverse_code = ColorCode::new(highlight, color);
    let center_char = if selected { '?' } else { ' ' };
    plot('/', c, r, code);
    plot('-', c+1, r, code);
    plot('-', c+2, r, code);
    plot('\\', c+3, r, code);
    plot('|', c, r+1, code);
    plot(center_char, c+1, r+1, inverse_code);
    plot(center_char, c+2, r+1, inverse_code);
    plot('|', c+3, r+1, code);
    plot('\\', c, r+2, code);
    plot('-', c+1, r+2, code);
    plot('-', c+2, r+2, code);
    plot('/', c+3, r+2, code);
}
fn draw_empty(c: usize, r: usize, color: Color) {
    for c in c..c+4 {
        for r in r..r+3 {
            plot(' ', c, r, ColorCode::new(color, color));
        }
    }
}
