use pc_keyboard::{DecodedKey, KeyCode};

pub const BOARD_HEIGHT: usize = 8;
pub const BOARD_WIDTH: usize = 8;
pub const GEM_COUNT: usize = 7;
pub const REFRESH_PERIOD: u64 = 4;

#[derive(Clone, Copy)]
pub enum GameState { EnteringCode, Playing }

pub struct GameStateManager {
    state: GameState,
    game_code: GameCode,
    game: Game
}

impl GameStateManager {
    pub fn new() -> Self {
        Self { state: GameState::EnteringCode, game_code: GameCode::new(), game: Game::new(0) }
    }

    pub fn input_manager(&mut self, key: DecodedKey) {
        const K_BACKSPACE: char = 0x08 as char;
        const K_ESCAPE: char = 0x1b as char;
        match self.state {
            GameState::EnteringCode => {
                match key {
                    DecodedKey::Unicode('\n') => self.start_game(),
                    DecodedKey::Unicode(K_BACKSPACE) => self.game_code.backspace(),
                    DecodedKey::Unicode(c @ ' '..='~') => self.game_code.type_char(c),
                    _ => {}
                }
            },
            GameState::Playing => {
                match key {
                    DecodedKey::Unicode(K_ESCAPE) if !self.game.alive => self.return_to_code_menu(),
                    key => self.game.handle_input(key)
                }
            }
        }
    }

    pub fn tick(&mut self, current_tick: u64) {
        match self.state {
            GameState::EnteringCode => {},
            GameState::Playing => {
                if current_tick % REFRESH_PERIOD == 0 {
                    let drop = self.game.drop_step();
                    let fill = self.game.fill_step();
                    let settled = !drop && !fill;
                    if settled {
                        // only check for game over if board is settled and has no matches
                        let old_score = self.game.get_score();
                        self.game.score_matches();
                        if self.game.get_score() == old_score {
                            self.game.check_for_game_over();
                        }
                    }
                }
            },
        }
    }

    fn start_game(&mut self) {
        let seed = self.game_code.hash();
        self.game = Game::new(seed);
        self.state = GameState::Playing;
    }

    fn return_to_code_menu(&mut self) {
        self.state = GameState::EnteringCode;
    }

    pub fn get_state(&self) -> GameState { self.state }
    pub fn get_game(&self) -> &Game { &self.game }
    pub fn get_code(&self) -> [char; 80] { self.game_code.code }
    pub fn get_code_len(&self) -> usize { self.game_code.cursor }
}

struct GameCode {
    code: [char; 80],
    cursor: usize
}

impl GameCode {
    fn new() -> Self {
        Self { code: [0 as char; 80], cursor: 0 }
    }

    fn type_char(&mut self, c: char) {
        if self.cursor < self.code.len() {
            self.code[self.cursor] = c;
            self.cursor += 1;
        }
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.code[self.cursor] = 0 as char;
        }
    }

    fn hash(&self) -> u64 {
        let mut x = 5040;
        for i in 0..self.cursor {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x ^= self.code[i] as u64;
        }
        x
    }
}

pub struct Game {
    board: [[u8; BOARD_HEIGHT]; BOARD_WIDTH],
    marks: [[bool; BOARD_HEIGHT]; BOARD_WIDTH],
    rand: Random,
    cursor: GameCursor,
    selected: bool,
    alive: bool,
    score: usize
}

impl Game { // TODO: most of these shouldn't be public
    pub fn new(seed: u64) -> Self {
        Self{
            board: [[0; BOARD_HEIGHT]; BOARD_WIDTH],
            marks: [[false; BOARD_HEIGHT]; BOARD_WIDTH],
            rand: Random::new(seed),
            cursor: GameCursor::new(),
            selected: false,
            alive: true,
            score: 0
        }
    }

    /// Find, score, and remove all existing matches.
    pub fn score_matches(&mut self) {
        self.calculate_marks();
        self.remove_marked();
    }

    /// Find and mark any matches on the board; returns the score of those matches.
    fn calculate_marks(&mut self) {
        let mut points: usize = 0;
        // find vertical matches
        for col in 0..BOARD_WIDTH {
            for row in 0..BOARD_HEIGHT-2 {
                let current = self.board[col][row];
                if current == 0 || self.marks[col][row] { continue; }
                if self.board[col][row+1] == current && self.board[col][row+2] == current {
                    self.marks[col][row] = true;
                    self.marks[col][row+1] = true;
                    self.marks[col][row+2] = true;
                    let mut size = 3;
                    while row + size < BOARD_HEIGHT && self.board[col][row+size] == current {
                        self.marks[col][row+size] = true;
                        size += 1;
                    }
                    points += Self::calculate_score(size);
                }
            }
        }
        // find horizontal matches
        for row in 0..BOARD_HEIGHT {
            for col in 0..BOARD_WIDTH-2 {
                let current = self.board[col][row];
                if current == 0 || self.marks[col][row] { continue; }
                if self.board[col+1][row] == current && self.board[col+2][row] == current {
                    self.marks[col][row] = true;
                    self.marks[col+1][row] = true;
                    self.marks[col+2][row] = true;
                    let mut size = 3;
                    while col + size < BOARD_WIDTH && self.board[col+size][row] == current {
                        self.marks[col+size][row] = true;
                        size += 1;
                    }
                    points += Self::calculate_score(size);
                }
            }
        }
        self.score += points;
    }

    /// Scoring calculator:
    /// match-3 = 3 points
    /// match-4 = 3 + 4 = 7 points
    /// match-n = 3 + 4 + ... + n points
    fn calculate_score(match_len: usize) -> usize {
        let mut x = 0;
        for i in 3..=match_len { x += i; }
        x
    }

    /// Erase all marked gems, then reset the markings
    fn remove_marked(&mut self) {
        for col in 0..BOARD_WIDTH {
            for row in 0..BOARD_HEIGHT {
                if self.marks[col][row] {
                    self.board[col][row] = 0;
                    self.marks[col][row] = false;
                }
            }
        }
    }

    /// Move all suspended gems down one space; returns whether any gems were moved.
    pub fn drop_step(&mut self) -> bool {
        let mut ongoing = false;
        for col in 0..BOARD_WIDTH {
            let mut row = BOARD_HEIGHT - 2;
            loop {
                let current = self.board[col][row];
                if current != 0 && self.board[col][row+1] == 0 {
                    self.board[col][row+1] = current;
                    self.board[col][row] = 0;
                    ongoing = true;
                } else if row == 0 {
                    break;
                } else {
                    row -= 1;
                }
            }
        }
        ongoing
    }

    /// Drop at most one gem into the top of all available columns; returns whether any gems were dropped.
    pub fn fill_step(&mut self) -> bool {
        let mut any: bool = false;
        for col in 0..BOARD_WIDTH {
            if self.board[col][0] == 0 {
                self.board[col][0] = self.rand.range(0, GEM_COUNT as u64) as u8 + 1;
                any = true;
            }
        }
        any
    }

    /// Swaps the gem at (c,r) with the gem at the cursor's location.
    fn swap_cursor_raw(&mut self, c: usize, r: usize) {
        let temp = self.board[self.cursor.0][self.cursor.1];
        self.board[self.cursor.0][self.cursor.1] = self.board[c][r];
        self.board[c][r] = temp;
    }

    /// Swaps the piece under the cursor and the piece in the `dir` direction from the cursor.
    fn swap_cursor(&mut self, dir: Direction) {
        match dir {
            Direction::Up => {
                if self.cursor.1 > 0 {
                    self.swap_cursor_raw(self.cursor.0, self.cursor.1 - 1);
                }
            },
            Direction::Down => {
                if self.cursor.1 < BOARD_HEIGHT - 1 {
                    self.swap_cursor_raw(self.cursor.0, self.cursor.1 + 1);
                }
            },
            Direction::Left => {
                if self.cursor.0 > 0 {
                    self.swap_cursor_raw(self.cursor.0 - 1, self.cursor.1);
                }
            },
            Direction::Right => {
                if self.cursor.0 < BOARD_WIDTH - 1 {
                    self.swap_cursor_raw(self.cursor.0 + 1, self.cursor.1);
                }
            }
        }
    }

    /// Check if the swap that was just performed in the given direction makes any match.
    fn makes_match(&self, dir: Direction) -> bool {
        let other_pos = match dir {
            Direction::Up    if self.cursor.1 > 0                 => (self.cursor.0, self.cursor.1 - 1),
            Direction::Down  if self.cursor.1 < BOARD_HEIGHT - 1  => (self.cursor.0, self.cursor.1 + 1),
            Direction::Left  if self.cursor.0 > 0                 => (self.cursor.0 - 1, self.cursor.1),
            Direction::Right if self.cursor.0 < BOARD_WIDTH - 1   => (self.cursor.0 + 1, self.cursor.1),
            _ => (self.cursor.0, self.cursor.1),
        };
        self.check_for_match(self.cursor.0, self.cursor.1) || self.check_for_match(other_pos.0, other_pos.1)
    }

    /// Check if the piece at (c,r) is part of a match.
    fn check_for_match(&self, c: usize, r: usize) -> bool {
        if c >= BOARD_WIDTH || r >= BOARD_HEIGHT { return false }
        let current = self.board[c][r];
        // vertical check
        if r >= 2 && self.board[c][r-2] == current && self.board[c][r-1] == current {
            return true
        } else if r >= 1 && r + 1 < BOARD_HEIGHT && self.board[c][r-1] == current && self.board[c][r+1] == current {
            return true
        } else if r + 2 < BOARD_HEIGHT && self.board[c][r+1] == current && self.board[c][r+2] == current {
            return true
        }
        // horizontal check
        if c >= 2 && self.board[c-2][r] == current && self.board[c-1][r] == current {
            return true
        } else if c >= 1 && c + 1 < BOARD_WIDTH && self.board[c-1][r] == current && self.board[c+1][r] == current {
            return true
        } else if c + 2 < BOARD_WIDTH && self.board[c+1][r] == current && self.board[c+2][r] == current {
            return true
        }
        false
    }

    /// Check if there are any valid moves left
    pub fn check_for_game_over(&mut self) { // TODO: rewrite without mutating self
        self.alive = false;
        let loc = self.cursor.location();
        // search for vertical moves
        for col in 0..BOARD_WIDTH {
            for row in 0..BOARD_HEIGHT-1 {
                self.cursor.set_cursor(col, row);
                self.swap_cursor(Direction::Down);
                if self.makes_match(Direction::Down) {
                    self.alive = true;
                }
                self.swap_cursor(Direction::Down);
                if self.alive {
                    self.cursor.set_cursor(loc.0, loc.1);
                    return
                }
            }
        }
        // search for horizontal matches
        self.cursor = GameCursor::new();
        for row in 0..BOARD_HEIGHT {
            for col in 0..BOARD_WIDTH-1 {
                self.cursor.set_cursor(col, row);
                self.swap_cursor(Direction::Right);
                if self.makes_match(Direction::Right) {
                    self.alive = true;
                }
                self.swap_cursor(Direction::Right);
                if self.alive {
                    self.cursor.set_cursor(loc.0, loc.1);
                    return
                }
            }
        }
        self.cursor.set_cursor(loc.0, loc.1);
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }

    pub fn get_board(&self) -> [[u8; BOARD_HEIGHT]; BOARD_WIDTH] {
        self.board
    }

    pub fn get_cursor(&self) -> &GameCursor {
        &self.cursor
    }

    pub fn get_score(&self) -> usize {
        self.score
    }

    fn handle_input(&mut self, key: DecodedKey) {
        use DecodedKey::*;
        let action = match key {
            RawKey(KeyCode::ArrowUp)    | Unicode('w') => Some(InputAction::Move(Direction::Up)),
            RawKey(KeyCode::ArrowDown)  | Unicode('s') => Some(InputAction::Move(Direction::Down)),
            RawKey(KeyCode::ArrowLeft)  | Unicode('a') => Some(InputAction::Move(Direction::Left)),
            RawKey(KeyCode::ArrowRight) | Unicode('d') => Some(InputAction::Move(Direction::Right)),
            Unicode('\n') | Unicode(' ') => Some(InputAction::Select),
            _ => None
        };
        if let Some(action) = action {
            self.do_action(action);
        }
    }

    /// Handles actions performed on the game.
    fn do_action(&mut self, action: InputAction) {
        match action {
            InputAction::Select => self.selected = !self.selected,
            InputAction::Move(dir)  => {
                if self.selected {
                    self.swap_cursor(dir);
                    if !self.makes_match(dir) {
                        self.swap_cursor(dir)
                    } else {
                        self.cursor.move_cursor(dir);
                        self.selected = false;
                    }
                } else {
                    self.cursor.move_cursor(dir);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum Direction { Up, Down, Left, Right }

#[derive(Clone, Copy)]
pub enum InputAction { Move(Direction), Select }

pub struct GameCursor(usize, usize);

impl GameCursor {
    fn new() -> Self {
        Self(0, 0)
    }
    fn move_cursor(&mut self, dir: Direction) {
        match dir {
            Direction::Up    => if self.1 > 0                { self.1 -= 1; },
            Direction::Down  => if self.1 < BOARD_HEIGHT - 1 { self.1 += 1; },
            Direction::Left  => if self.0 > 0                { self.0 -= 1; },
            Direction::Right => if self.0 < BOARD_WIDTH - 1  { self.0 += 1; }
        }
    }
    fn set_cursor(&mut self, c: usize, r: usize) {
        self.0 = c;
        self.1 = r;
    }
    pub fn location(&self) -> (usize, usize) {
        (self.0, self.1)
    }
}

/// A simple 64-bit xorshift. Source: https://en.wikipedia.org/wiki/Xorshift#Example_implementation
struct Random {
    state: u64
}

impl Random {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
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