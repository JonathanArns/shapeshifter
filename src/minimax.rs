use crate::types::*;
use crate::bitboard::*;
use crate::move_gen::*;
use crate::eval::*;

use std::env;
use std::time;
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};

lazy_static! {
    static ref FIXED_DEPTH: i8 = if let Ok(var) = env::var("FIXED_DEPTH") {
        var.parse().unwrap()
    } else {
        -1
    };
}

pub fn search<const N: usize>(board: &Bitboard<N>, g: &mut Game) -> (Move, Score) {
    if *FIXED_DEPTH > 0 {
        fixed_depth_search(board, g, *FIXED_DEPTH as u8)
    } else {
        iterative_deepening_search(board, g)
    }
}

pub fn fixed_depth_search<const N: usize>(board: &Bitboard<N>, _g: &mut Game, depth: u8) -> (Move, Score) {
    let mut node_counter = 0;
    let start_time = time::Instant::now(); // only used to calculate nodes / second
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut enemy_moves = move_combinations(board, 1);
    let my_moves = allowed_moves(board, board.snakes[0].head);
    let mut best = Score::MIN+1;
    for mv in &my_moves {
        let score = alphabeta(board, &mut node_counter, *mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX);
        if score > best {
            best = score;
            best_move = *mv;
            best_score = best;
        }
    }
    println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    (best_move, best_score)
}

pub fn iterative_deepening_search<const N: usize>(board: &Bitboard<N>, g: &mut Game) -> (Move, Score) {
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut best_depth = 1;
    let start_time = time::Instant::now();
    let soft_deadline = start_time + g.move_time / 10;
    let hard_deadline = start_time + g.move_time / 2;

    let (stop_sender, stop_receiver) = unbounded();
    let (result_sender, result_receiver) : (Sender<(Move, Score, u8)>, Receiver<(Move, Score, u8)>) = unbounded();

    let board = board.clone();
    thread::spawn(move || {
        let mut node_counter = 0;
        let start_time = time::Instant::now(); // only used to calculate nodes / second
        let mut best_move = Move::Up;
        let mut best_score = Score::MIN+1;
        let mut depth = 1;
        let mut enemy_moves = move_combinations(&board, 1);
        let my_moves = allowed_moves(&board, board.snakes[0].head);
        loop {
            let mut best = Score::MIN+1;
            for mv in &my_moves {
                let score = alphabeta(&board, &mut node_counter, *mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX);
                if score > best {
                    best = score;
                    best_move = *mv;
                    best_score = best;
                }
            }
            result_sender.try_send((best_move, best_score, depth)).ok();
            if best == Score::MAX || best < Score::MIN+4 {
                break
            }
            if let Ok(_) = stop_receiver.try_recv() {
                break // stop thread because time is out and response has been sent
            }
            depth += 1;
        }
        println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    });

    // receive results
    while time::Instant::now() < soft_deadline {
        if let Ok(msg) = result_receiver.try_recv() {
            best_move = msg.0;
            best_score = msg.1;
            best_depth = msg.2
        } else {
            thread::sleep(time::Duration::from_millis(1));
        }
    }
    stop_sender.send(1).ok(); // Channel might be broken, if search returned early. We don't care.

    // wait for eventual results from still running search
    if let Ok(msg) = result_receiver.recv_timeout(hard_deadline - time::Instant::now()) {
        best_move = msg.0;
        best_score = msg.1;
        best_depth = msg.2
    }

    println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move, best_score, best_depth, time::Instant::now().duration_since(start_time).as_millis());
    (best_move, best_score)
}

pub fn alphabeta<const N: usize>(board: &Bitboard<N>, node_counter: &mut u64, mv: Move, enemy_moves: &mut Vec<[Move; N]>, depth: u8, alpha: Score, mut beta: Score) -> Score { // min call
    *node_counter += 1;
    if board.is_terminal() {
        return eval_terminal(board)
    }
    if depth <= 0 {
        return eval(board)
    }
    // // ProbCut heuristic
    // if depth == 4 {
    //     let a = 1.0;
    //     let b = 0.1;
    //     let percentile = 1.5;
    //     let sigma = 0.5;
    //     let mut bound = ((percentile * sigma + beta as f32 - b) / a).round() as Score;
    //     if self.alphabeta(mv, enemy_moves, 3, bound-1, bound) >= bound {
    //         print!("c");
    //         return beta 
    //     }
    //     bound = ((-percentile * sigma + alpha as f32 - b) / a).round() as Score;
    //     if self.alphabeta(mv, enemy_moves, 3, bound, bound+1) <= bound {
    //         print!("c");
    //         return alpha
    //     }
    // }

    // search
    for mvs in enemy_moves { // TODO: apply move ordering
        let score = { // max call
            let mut ialpha = alpha;
            let ibeta = beta;
            mvs[0] = mv;
            let child = board.apply_moves(mvs);
            let mut next_enemy_moves = move_combinations(&child, 1);
            for mv in allowed_moves(&child, child.snakes[0].head) { // TODO: apply move ordering
                let iscore = alphabeta(&child, node_counter, mv, &mut next_enemy_moves, depth-1, alpha, beta);
                if iscore > ibeta {
                    ialpha = ibeta;
                    break // same as return beta
                }
                if iscore > ialpha {
                    ialpha = iscore;
                }
            }
            ialpha
        };
        if score < alpha {
            return alpha
        }
        if score < beta {
            beta = score;
        }
    }
    beta
}

fn order_enemy_moves<const N: usize>(board: &Bitboard<N>, moves: &mut Vec<[Move; N]>) {
    let mut unique_moves_seen = Vec::<(Move, u8)>::with_capacity(N*N);
    moves.sort_by_cached_key(|x| {
        let me = board.snakes[0];
        let mut key = 0;
        for (i, snake) in board.snakes[1..].iter().enumerate() {
            if !snake.is_alive() {
                continue
            }
            let mv = x[i+1];
            // make sure to quickly cover all possible moves once
            if !unique_moves_seen.contains(&(mv, i as u8)) {
                unique_moves_seen.push((mv, i as u8));
                key += 100;
            }
            // if snake is longer, walk towards me, otherwise walk away from me
            key += (snake.length > me.length && is_in_direction(snake.head, me.head, mv)) as u8;
        }
        key
    });
}
