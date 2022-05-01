use crate::bitboard::{Bitboard, Move};
use crate::minimax::Score;

use std::fs::OpenOptions;
use std::io::prelude::*;

pub fn write_datapoint<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>, score: Score
) where
    [(); (W * H + 127) / 128]: Sized,
{
    let features = bitboard_to_active_features(board);
    

    let mut datapoint = score.to_string();
    datapoint.push_str(&format_board(board));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("nnue-data.csv")
        .unwrap();
    if let Err(e) = writeln!(file, "{}", datapoint) {
        eprintln!("Couldn't write to file: {}", e);
    }
}

// uses 4 * 121 * 121 + 4 features
pub fn bitboard_to_active_features<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Vec<(usize, u16)>
where [(); (W*H+127)/128]: Sized {
    let mut res = Vec::with_capacity(256);
    let mut j = 0;

    // health
    res.push((j, board.snakes[j].health as u16));
    j += 1;
    if board.snakes[j].is_alive() {
        res.push((j, board.snakes[j].health as u16));
    }
    j += 1;
    if S > 2 && board.snakes[j].is_alive() {
        res.push((j, board.snakes[j].health as u16));
    }
    j += 1;
    if S > 3 && board.snakes[j].is_alive() {
        res.push((j, board.snakes[j].health as u16));
    }
    j += 1 + board.snakes[0].head as usize * 121 * 4; // features from here on are relative to my head

    // bodies
    for snake in board.snakes {
        if snake.is_dead() {
            continue
        }
        let mut x = snake.curled_bodyparts;
        let mut tail_pos = snake.tail;
        if x == 0 {
            let move_int = board.bodies[1].get_bit(tail_pos as usize) as u8 | (board.bodies[2].get_bit(tail_pos as usize) as u8) << 1;
            tail_pos = if WRAP {
                tail_pos as i16 + Move::int_to_index_wrapping(move_int, W, H, tail_pos)
            } else {
                tail_pos as i16 + Move::int_to_index(move_int, W)
            } as u16;
            x += 1;
        }
        while snake.head != tail_pos {
            let move_int = board.bodies[1].get_bit(tail_pos as usize) as u8 | (board.bodies[2].get_bit(tail_pos as usize) as u8) << 1;
            res.push((j + tail_pos as usize, x as u16));
            tail_pos = if WRAP {
                tail_pos as i16 + Move::int_to_index_wrapping(move_int, W, H, tail_pos)
            } else {
                tail_pos as i16 + Move::int_to_index(move_int, W)
            } as u16;
            x += 1;
        }
    }
    j += 121;

    // enemy heads
    for snake in board.snakes {
        if snake.is_dead() {
            continue
        }
        res.push((j + snake.head as usize, 1));
    }
    j += 121;

    for i in 0..(W as u16 * H as u16) {
        // food
        if board.food.get_bit(i as usize) {
            res.push((j, 1));
        }
        j += 1;

        // hazards
        if board.hazards.get_bit(i as usize) {
            res.push((j, 1));
        }
        j += 1;
    }
    res
}

pub fn bitboard_to_slice<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> [f32; W*H*4]
where [(); (W*H+127)/128]: Sized, [(); W*H*4]: Sized {
    let mut slice = [0_f32; W*H*4];
    let mut j = 0;
    for i in 0..(W as u16 * H as u16) {
        // bodies
        if board.bodies[0].get_bit(i as usize) {
            slice[j] = 1.0;
        }
        j += 1;
        // my head
        if board.snakes[0].head == i {
            slice[j] = 1.0;
        }
        j += 1;
        // enemy heads
        if board.snakes[1].head == i
            || (board.snakes.len() > 2 && board.snakes[2].head == i)
            || (board.snakes.len() > 3 && board.snakes[3].head == i)
        {
            slice[j] = 1.0;
        }
        j += 1;
        // tails
        if board.snakes[0].tail == i
            || board.snakes[1].tail == i
            || (board.snakes.len() > 2 && board.snakes[2].head == i)
            || (board.snakes.len() > 3 && board.snakes[3].head == i)
        {
            slice[j] = 1.0;
        }
        j += 1;
    }
    return slice
}

pub fn format_board<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>,
) -> String
where
    [(); (W * H + 127) / 128]: Sized,
{
    let mut res = "".to_string();
    for i in 0..(W as u16 * H as u16) {
        // bodies
        if board.bodies[0].get_bit(i as usize) {
            res.push_str(",1");
        } else {
            res.push_str(",0");
        }
        // my head
        if board.snakes[0].head == i {
            res.push_str(",1")
        } else {
            res.push_str(",0")
        }
        // enemy heads
        if board.snakes[1].head == i
            || (board.snakes.len() > 2 && board.snakes[2].head == i)
            || (board.snakes.len() > 3 && board.snakes[3].head == i)
        {
            res.push_str(",1")
        } else {
            res.push_str(",0")
        }
        // tails
        if board.snakes[0].tail == i
            || board.snakes[1].tail == i
            || (board.snakes.len() > 2 && board.snakes[2].head == i)
            || (board.snakes.len() > 3 && board.snakes[3].head == i)
        {
            res.push_str(",1")
        } else {
            res.push_str(",0")
        }
    }
    return res;
}
