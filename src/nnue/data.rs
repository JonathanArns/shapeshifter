use crate::bitboard::Bitboard;
use crate::bitboard::Bitset;
use crate::minimax::eval::area_control;
use crate::minimax::Score;

use std::fs::OpenOptions;
use std::io::prelude::*;

pub fn write_datapoint<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>, score: Score
) where
    [(); (W * H + 127) / 128]: Sized,
{
    let ((my_area, enemy_area), (_, _), _) = area_control(board);
    let score = my_area.count_ones() as Score - enemy_area.count_ones() as Score;
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

pub fn bitboard_to_active_features<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Vec<usize>
where [(); (W*H+127)/128]: Sized, [(); W*H*4]: Sized {
    let mut res = Vec::with_capacity(64);
    let mut j = 0;
    for i in 0..(W as u16 * H as u16) {
        // bodies
        if board.bodies[0].get_bit(i as usize) {
            res.push(j);
        }
        j += 1;
        // my head
        if board.snakes[0].head == i {
            res.push(j);
        }
        j += 1;
        // enemy heads
        if board.snakes[1].head == i
            || (board.snakes.len() > 2 && board.snakes[2].head == i)
            || (board.snakes.len() > 3 && board.snakes[3].head == i)
        {
            res.push(j);
        }
        j += 1;
        // tails
        if board.snakes[0].tail == i
            || board.snakes[1].tail == i
            || (board.snakes.len() > 2 && board.snakes[2].head == i)
            || (board.snakes.len() > 3 && board.snakes[3].head == i)
        {
            res.push(j);
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
