use crate::types::*;
use crate::bitboard::*;
use crate::move_gen::*;
use crate::eval::*;

use std::env;
use std::time;
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};
use arrayvec::ArrayVec;

lazy_static! {
    static ref FIXED_DEPTH: i8 = if let Ok(var) = env::var("FIXED_DEPTH") {
        var.parse().unwrap()
    } else {
        -1
    };
}

pub fn search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, g: &mut Game) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    if *FIXED_DEPTH > 0 {
        fixed_depth_search(board, g, *FIXED_DEPTH as u8)
    } else {
        iterative_deepening_search(board, g)
    }
}

pub fn fixed_depth_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, g: &mut Game, depth: u8) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    let mut node_counter = 0;
    let start_time = time::Instant::now(); // only used to calculate nodes / second
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut enemy_moves = limited_move_combinations(board, 1);
    let my_moves = allowed_moves(board, board.snakes[0].head);
    let mut best = Score::MIN+1;
    for mv in &my_moves {
        let (score, _) = alphabeta(board, g.ruleset, &mut node_counter, *mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX);
        if score > best {
            best = score;
            best_move = *mv;
            best_score = best;
        }
    }
    println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    (best_move, best_score, depth)
}

pub fn iterative_deepening_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, g: &mut Game) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut best_depth = 1;
    let start_time = time::Instant::now();
    let soft_deadline = start_time + g.move_time / 10;
    let hard_deadline = start_time + g.move_time / 2;

    let (stop_sender, stop_receiver) = unbounded();
    let (result_sender, result_receiver) : (Sender<(Move, Score, u8)>, Receiver<(Move, Score, u8)>) = unbounded();

    let ruleset = g.ruleset;
    let board = board.clone();
    thread::spawn(move || {
        let mut node_counter = 0;
        let start_time = time::Instant::now(); // only used to calculate nodes / second
        let mut best_move = Move::Up;
        let mut depth = 1;
        let mut enemy_moves = limited_move_combinations(&board, 1);
        let my_moves = allowed_moves(&board, board.snakes[0].head);
        loop {
            let mut best = Score::MIN+1;
            let mut best_unused_depth = depth;
            for mv in &my_moves {
                let (score, unused_depth) = alphabeta(&board, ruleset, &mut node_counter, *mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX);
                if score > best || (score == best && unused_depth < best_unused_depth) {
                    best = score;
                    best_move = *mv;
                    best_unused_depth = unused_depth;
                }
            }
            result_sender.try_send((best_move, best, depth)).ok();
            if best == Score::MAX || best < Score::MIN + 5 {
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
    (best_move, best_score, best_depth)
}

pub fn alphabeta<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>,
    ruleset: Ruleset,
    node_counter: &mut u64,
    mv: Move,
    enemy_moves: &mut ArrayVec<[Move; S], 4>,
    depth: u8,
    alpha: Score,
    mut beta: Score
) -> (Score, u8)
where [(); (W*H+127)/128]: Sized {  // min call
    // search
    for mvs in enemy_moves { // TODO: apply move ordering
        let score = { // max call
            let mut ialpha = alpha;
            let ibeta = beta;
            mvs[0] = mv;
            let mut child = board.clone();
            child.apply_moves(&mvs, ruleset);
            *node_counter += 1;

            // search stops
            if child.is_terminal() {
                ialpha = eval_terminal(&child);
            } else if depth == 1 {
                ialpha = eval(&child, ruleset);
            }
            // } else if let Some(entry) = ttable::get(&child) {
            //     if entry.get_depth() >= depth {
            //         ialpha = entry.get_score();
            //     }
            // }

            // continue search
            if depth > 1 {
                let mut next_enemy_moves = limited_move_combinations(&child, 1);
                for mv in allowed_moves(&child, child.snakes[0].head) { // TODO: apply move ordering
                    let (iscore, _) = alphabeta(&child, ruleset, node_counter, mv, &mut next_enemy_moves, depth-1, alpha, beta);
                    if iscore > ibeta {
                        ialpha = ibeta;
                        break // same as return beta
                    }
                    if iscore > ialpha {
                        ialpha = iscore;
                    }
                }
            }
            ialpha
        };
        if score < alpha {
            return (alpha, depth)
        }
        if score < beta {
            beta = score;
        }
    }
    (beta, depth)
}

fn order_enemy_moves<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, moves: &mut Vec<[Move; S]>)
where [(); (W*H+127)/128]: Sized {
    let mut unique_moves_seen = Vec::<(Move, u8)>::with_capacity(S*S);
    moves.sort_by_cached_key(|x| {
        let me = board.snakes[0];
        let mut key = 0;
        for (i, snake) in board.snakes[1..].iter().enumerate() {
            if snake.is_dead() {
                continue
            }
            let mv = x[i+1];
            // make sure to quickly cover all possible moves once
            if !unique_moves_seen.contains(&(mv, i as u8)) {
                unique_moves_seen.push((mv, i as u8));
                key += 100;
            }
            // if snake is longer, walk towards me, otherwise walk away from me
            key += (snake.length > me.length && board.is_in_direction(snake.head, me.head, mv)) as u8;
        }
        key
    });
}
