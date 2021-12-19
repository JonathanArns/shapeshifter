use rocket_contrib::json::JsonValue;

pub type Score = i16;
pub type Square = u8;

pub const FOOD: Square = 1;
pub const HAZARD: Square = 2;
pub const HEAD: Square = 4;
pub const BODY: Square = 8;
pub const TAIL: Square = 16;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Up = 0,
    Down = 1,
    Right = 2,
    Left = 3,
}

impl Move {
    pub fn to_int(&self) -> u8 {
        *self as u8
    }

    pub fn to_index(&self, board_width: u8) -> i8 {
        Self::int_to_index(self.to_int(), board_width)
    }

    pub fn to_json(&self) -> JsonValue {
        match self {
            Move::Up => json!({ "move": "up" }),
            Move::Down => json!({ "move": "down" }),
            Move::Left => json!({ "move": "left" }),
            Move::Right => json!({ "move": "right" }),
        }
    }

    /// Does not do a safety check, so only call with 0, 1, 2, 3 !
    pub fn from_int(x: u8) -> Self {
        unsafe { std::mem::transmute(x) }
    }
    
    pub fn int_to_index(x: u8, board_width: u8) -> i8 {
        [11, -11, 1, -1][x as usize]
    }
}

pub struct Game {
    pub move_time: std::time::Duration,
}

pub trait Board {
    fn alphabeta(&self, d: u8, alpha: Score, beta: Score) -> (Move, Score, u8);
    fn num_snakes(&self) -> usize;
}
