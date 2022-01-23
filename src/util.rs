use crate::types::*;

pub fn distance(x: u16, y: u16, width: u16) -> u16 {
    ((x/width).max(y/width) - (x/width).min(y/width)) + ((x%width).max(y%width) - (x%width).min(y%width))
}

pub fn is_in_direction(from: u16, to: u16, mv: Move, width: u16) -> bool {
    match mv {
        Move::Left => from % width > to % width,
        Move::Right => from % width < to % width,
        Move::Down => from / width > to / width,
        Move::Up => from / width < to / width,
    }
}

