use rocket_contrib::json::JsonValue;
use rand::Rng;

pub type Score = i16;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Move {
    Up = 0,
    Down = 1,
    Right = 2,
    Left = 3,
}

impl Move {
    pub const fn to_int(&self) -> u8 {
        *self as u8
    }

    pub const fn to_index(&self, width: usize) -> i16 {
        Self::int_to_index(self.to_int(), width)
    }

    pub const fn to_index_wrapping(&self, width: usize, height: usize, from: u16) -> i16 {
        Self::int_to_index_wrapping(self.to_int(), width, height, from)
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
    pub const fn from_int(x: u8) -> Self {
        unsafe { std::mem::transmute(x) }
    }
    
    pub const fn int_to_index(x: u8, width: usize) -> i16 {
        match x {
            0 => width as i16,
            1 => -(width as i16),
            2 => 1,
            3 => -1,
            _ => panic!("Bad move int")
        }
    }

    pub const fn int_to_index_wrapping(x: u8, width: usize, height: usize, from: u16) -> i16 {
        let h = height as i16;
        let w = width as i16;
        if x == 0 {
            if (from as i16) < (h - 1) * w {
                w
            } else {
                -(w * (h - 1))
            }
        } else if x == 1 {
            if (from as i16) >= w {
                -w
            } else {
                w * (h - 1)
            }
        } else if x == 2 {
            if (from as i16) % w < w-1 {
                1
            } else {
                -(w - 1)
            }
        } else {
            if (from as i16) % w > 0 {
                -1
            } else {
                w - 1
            }
        }
    }

    pub fn random() -> Self {
        Move::from_int(rand::thread_rng().gen_range(0..4))
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Ruleset {
    Standard,
    Royale,
    Wrapped,
    WrappedSpiral,
    Constrictor,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_int_conversions() {
        assert!(Move::Up == Move::from_int(Move::Up.to_int()));
        assert!(Move::Down == Move::from_int(Move::Down.to_int()));
        assert!(Move::Left == Move::from_int(Move::Left.to_int()));
        assert!(Move::Right == Move::from_int(Move::Right.to_int()));
    }
}
