#![no_std]
#![no_main]

mod vga_buffer;
mod serial;
mod match3game;

use lazy_static::lazy_static;
use match3game::{Game, InputAction};
use pc_keyboard::{DecodedKey, KeyCode};
use csci320_match3::HandlerTable;
use spin::Mutex;
use vga_buffer::{plot, ColorCode, Color};

use crate::vga_buffer::{plot_num_right_justified, plot_str};

const TICK_PERIOD: u64 = 4;

lazy_static! {
    static ref TICK: Mutex<u64> = Mutex::new(0);
}
lazy_static! {
    static ref GAME: Mutex<Game> = Mutex::new(Game::new(1));
}

fn start() {
    draw_game(&mut *GAME.lock());
}

fn tick() {
    if *TICK.lock() % TICK_PERIOD == 0 {
        let drop = GAME.lock().drop_step();
        let fill = GAME.lock().fill_step();
        let settled = !drop && !fill;
        if settled {
            // only check for game over if board is settled and has no matches
            let old_score = GAME.lock().get_score();
            GAME.lock().score_matches();
            if GAME.lock().get_score() == old_score {
                GAME.lock().check_for_game_over();
            }
        }
    }
    draw_game(&mut *GAME.lock());
    *TICK.lock() += 1;
}

fn key(key: DecodedKey) {
    use DecodedKey::*;
    let action = match key {
        RawKey(KeyCode::ArrowUp)    | Unicode('w') => Some(InputAction::Up),
        RawKey(KeyCode::ArrowDown)  | Unicode('s') => Some(InputAction::Down),
        RawKey(KeyCode::ArrowLeft)  | Unicode('a') => Some(InputAction::Left),
        RawKey(KeyCode::ArrowRight) | Unicode('d') => Some(InputAction::Right),
        Unicode('\n') | Unicode(' ') => Some(InputAction::Select),
        _ => None
    };
    if let Some(action) = action { GAME.lock().do_action(action); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    HandlerTable::new()
        .keyboard(key)
        .timer(tick)
        .startup(start)
        .start()
}

fn draw_game(g: &Game) {
    // board
    const DRAW_COL_OFFSET: usize = 20;
    const DRAW_ROW_OFFSET: usize = 0;
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
                    && *TICK.lock() % (TICK_PERIOD * 2) < TICK_PERIOD;
                draw_gem(draw_col, draw_row, color, highlight, selected);
            }
        }
    }
    // score
    let ui_code = ColorCode::new(Color::White, Color::DarkGray);
    plot_num_right_justified(35, g.get_score() as isize * 100, 25, vga_buffer::BUFFER_HEIGHT-1, ui_code);
    let msg = if g.is_alive() { "Score: " } else {"Game Over! Final Score:" };
    plot_str(msg, 20, vga_buffer::BUFFER_HEIGHT-1, ui_code);
    // outline
    for row in 0..vga_buffer::BUFFER_HEIGHT {
        plot(' ', DRAW_COL_OFFSET - 1, row, ui_code);
        plot(' ', DRAW_COL_OFFSET + 40, row, ui_code);
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
    plot(' ', c+4, r, code);
    plot('|', c, r+1, code);
    plot(center_char, c+1, r+1, inverse_code);
    plot(center_char, c+2, r+1, inverse_code);
    plot('|', c+3, r+1, code);
    plot(' ', c+4, r+1, code);
    plot('\\', c, r+2, code);
    plot('-', c+1, r+2, code);
    plot('-', c+2, r+2, code);
    plot('/', c+3, r+2, code);
    plot(' ', c+4, r+2, code);
}
fn draw_empty(c: usize, r: usize, color: Color) {
    for c in c..c+5 {
        for r in r..r+3 {
            plot(' ', c, r, ColorCode::new(color, color));
        }
    }
}
