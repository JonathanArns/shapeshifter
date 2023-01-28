#![feature(test, generic_const_exprs, async_closure, let_chains)]

use shapeshifter::bitboard::*;
use shapeshifter::minimax::*;
use std::fs::File;
use std::io::BufWriter;
use std::io::{prelude::*, BufReader};
use indicatif::ProgressBar;

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        panic!("exactly 2 arguments, input path and output path are required")
    }
    let input_path = args[1].as_str();
    let output_path = args[2].as_str();
    
    let input_name = input_path.split("/").last().expect("empty input path");
    let output_name = output_path.split("/").last().expect("empty output path");

    let mut input_params = input_name.split("_");
    let mut output_params = output_name.split("_");

    let in_gamemode = input_params.next().expect("missing intput gamemode");
    let out_gamemode = output_params.next().expect("missing output gamemode");
    let in_type_params = input_params.next().expect("missing intput type params");
    let out_type_params = output_params.next().expect("missing output type params");
    let in_boards_or_features = input_params.next().expect("missing boards or features specifier");
    let out_boards_or_features = output_params.next().expect("missing boards or features specifier");

    // input path cannot equal output path
    if input_path == output_path {
        panic!("input and output paths need to be different");
    }

    // check if input matches output
    if in_gamemode != out_gamemode || in_type_params != out_type_params {
        panic!("gamemode and type params must match on input and output");
    }

    match (in_type_params, in_boards_or_features, out_boards_or_features) {
        ("2-11x11-NOWRAP-NOSTACK", "boards", "boards") => evaluate_stored_boards::<2, 11, 11, false, false, 0>(&input_path, &output_path, 5),
        ("2-11x11-NOWRAP-NOSTACK", "boards", "features") => transform_stored_board_to_features::<2, 11, 11, false, false, 0>(&input_path, &output_path),

        ("4-11x11-NOWRAP-NOSTACK", "boards", "boards") => evaluate_stored_boards::<4, 11, 11, false, false, 0>(&input_path, &output_path, 5),
        ("4-11x11-NOWRAP-NOSTACK", "boards", "features") => transform_stored_board_to_features::<4, 11, 11, false, false, 0>(&input_path, &output_path),

        ("4-11x11-WRAP-NOSTACK", "boards", "boards") => evaluate_stored_boards::<4, 11, 11, true, false, 0>(&input_path, &output_path, 5),
        ("4-11x11-WRAP-NOSTACK", "boards", "features") => transform_stored_board_to_features::<4, 11, 11, true, false, 0>(&input_path, &output_path),

        (_, _, _) => panic!("Board type parameters or operation not supported"),
    }
}

/// Reads in a file of json boards line by line and produces a new file with scores added.
pub fn evaluate_stored_boards<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(in_path: &str, out_path: &str, depth: u8)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let num_lines = BufReader::new(File::open(in_path).expect("coudln't open file")).lines().count();
    let bar = ProgressBar::new(num_lines as u64);
    let mut new_file = File::create(out_path).expect("coudln't create file");
    let mut writer = BufWriter::new(new_file);
    let file = File::open(in_path).expect("coudln't open file");
    for line in BufReader::new(file).lines() {
        let line = line.unwrap();
        let (_, board) = line.split_once(";").unwrap();
        let board = Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::from_str(&board).unwrap();
        let (_, score, _) = fixed_depth_search(&board, depth);
        if let Err(e) = writeln!(writer, "{};{}", score, board.to_string().unwrap()) {
            eprintln!("Couldn't write to file: {}", e);
        }
        bar.inc(1);
    }
}

/// Transforms the json boards in an evaluated boards file into neural net input features.
pub fn transform_stored_board_to_features<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(in_path: &str, out_path: &str)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized, [(); W*H*7]: Sized {
    let num_lines = BufReader::new(File::open(in_path).expect("coudln't open file")).lines().count();
    let bar = ProgressBar::new(num_lines as u64);
    let mut new_file = File::create(out_path).expect("coudln't create file");
    let mut writer = BufWriter::new(new_file);
    let file = File::open(in_path).expect("couldn't open file");
    for line in BufReader::new(file).lines() {
        let line = line.unwrap();
        let (score, board) = line.split_once(";").unwrap();
        let board = Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::from_str(&board).unwrap();
        if let Err(e) = writeln!(writer, "{};{:?}", score, board.get_nn_input()) {
            eprintln!("Couldn't write to file: {}", e);
        }
        bar.inc(1);
    }
}
