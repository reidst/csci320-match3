pub const BOARD_HEIGHT: usize = 8;
pub const BOARD_WIDTH: usize = 8;
pub const GEM_COUNT: usize = 7;

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
        let x = match_len as isize;
        (x*x + x - 6) as usize / 2
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
    fn swap_cursor(&mut self, dir: InputAction) {
        match dir {
            InputAction::Up => {
                if self.cursor.1 > 0 {
                    self.swap_cursor_raw(self.cursor.0, self.cursor.1 - 1);
                }
            },
            InputAction::Down => {
                if self.cursor.1 < BOARD_HEIGHT - 1 {
                    self.swap_cursor_raw(self.cursor.0, self.cursor.1 + 1);
                }
            },
            InputAction::Left => {
                if self.cursor.0 > 0 {
                    self.swap_cursor_raw(self.cursor.0 - 1, self.cursor.1);
                }
            },
            InputAction::Right => {
                if self.cursor.0 < BOARD_WIDTH - 1 {
                    self.swap_cursor_raw(self.cursor.0 + 1, self.cursor.1);
                }
            },
            _ => {}
        }
    }

    /// Check if the swap that was just performed in the given direction makes any match.
    fn makes_match(&self, dir: InputAction) -> bool {
        let other_pos = match dir {
            InputAction::Up    if self.cursor.1 > 0                 => (self.cursor.0, self.cursor.1 - 1),
            InputAction::Down  if self.cursor.1 < BOARD_HEIGHT - 1  => (self.cursor.0, self.cursor.1 + 1),
            InputAction::Left  if self.cursor.0 > 0                 => (self.cursor.0 - 1, self.cursor.1),
            InputAction::Right if self.cursor.0 < BOARD_WIDTH - 1   => (self.cursor.0 + 1, self.cursor.1),
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
    pub fn check_for_game_over(&mut self) {
        self.alive = false;
        let loc = self.cursor.location();
        // search for vertical moves
        for col in 0..BOARD_WIDTH {
            for row in 0..BOARD_HEIGHT-1 {
                self.cursor.set_cursor(col, row);
                self.swap_cursor(InputAction::Down);
                if self.makes_match(InputAction::Down) {
                    self.alive = true;
                }
                self.swap_cursor(InputAction::Down);
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
                self.swap_cursor(InputAction::Right);
                if self.makes_match(InputAction::Right) {
                    self.alive = true;
                }
                self.swap_cursor(InputAction::Right);
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

    /// Handles actions performed on the game.
    pub fn do_action(&mut self, action: InputAction) {
        match action {
            InputAction::Select => self.selected = !self.selected,
            dir  => {
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
pub enum InputAction { Up, Down, Left, Right, Select }

pub struct GameCursor(usize, usize);

impl GameCursor {
    fn new() -> Self {
        Self(0, 0)
    }
    fn move_cursor(&mut self, dir: InputAction) {
        match dir {
            InputAction::Up => if self.1 > 0 { self.1 -= 1; },
            InputAction::Down => if self.1 < BOARD_HEIGHT - 1 { self.1 += 1; },
            InputAction::Left => if self.0 > 0 { self.0 -= 1; },
            InputAction::Right => if self.0 < BOARD_WIDTH - 1 { self.0 += 1; },
            _ => {}
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