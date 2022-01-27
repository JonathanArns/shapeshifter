use rocket_contrib::json::JsonValue;

pub type Score = i16;

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

    pub fn to_index(&self, board_width: usize) -> i16 {
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
    
    pub fn int_to_index(x: u8, width: usize) -> i16 {
        [width as i16, -(width as i16), 1, -1][x as usize]
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Ruleset {
    Standard,
    Royale,
    Constrictor,
}

pub struct Game {
    pub move_time: std::time::Duration,
    pub ruleset: Ruleset,
}
