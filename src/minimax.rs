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
        // iterative_deepening_search(board, g)
        best_node_search(board, g)
    }
}

pub fn fixed_depth_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, g: &mut Game, depth: u8) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    let mut node_counter = 0;
    let start_time = time::Instant::now(); // only used to calculate nodes / second
    let (_, stop_receiver) = unbounded(); // only used for alphabeta type signature
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut enemy_moves = limited_move_combinations(board, 1);
    let my_moves = allowed_moves(board, board.snakes[0].head);
    let mut best = Score::MIN+1;
    for mv in &my_moves {
        let (score, _) = alphabeta(board, g.ruleset, &mut node_counter, &stop_receiver, *mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX).unwrap();
        if score > best {
            best = score;
            best_move = *mv;
            best_score = best;
        }
    }
    println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    (best_move, best_score, depth)
}

fn next_bns_guess(alpha: Score, beta: Score, subtree_count: usize) -> Score {
    alpha / 2
    + beta / 2
    + 1
    // * (subtree_count as Score - 1) / subtree_count as Score
}

pub fn best_node_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, g: &mut Game) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut best_depth = 1;
    let start_time = time::Instant::now();
    let deadline = start_time + g.move_time / 2;

    let (stop_sender, stop_receiver) = unbounded();
    let (result_sender, result_receiver) : (Sender<(Move, Score, u8)>, Receiver<(Move, Score, u8)>) = unbounded();

    let ruleset = g.ruleset;
    let board = board.clone();
    thread::spawn(move || {
        let mut node_counter = 0;
        let start_time = time::Instant::now(); // only used to calculate nodes / second
        let mut depth = 1;
        let mut enemy_moves = limited_move_combinations(&board, 1);
        let mut last_test = 0;
        'outer_loop: loop {
            let mut my_moves = allowed_moves(&board, board.snakes[0].head);
            let mut alpha = Score::MIN+1;
            let mut beta = Score::MAX;
            let best_move;
            loop {
                let test = if last_test > alpha && last_test < beta {
                    last_test
                } else {
                    next_bns_guess(alpha, beta, my_moves.len())
                };
                let mut better_moves = ArrayVec::<Move, 4>::new();
                for mv in &my_moves {
                    if let Some((score, _)) = alphabeta(&board, ruleset, &mut node_counter, &stop_receiver, *mv, &mut enemy_moves, depth, test-1, test) {
                        if score >= test {
                            better_moves.push(*mv);
                            best_score = score;
                        }
                    } else {
                        break 'outer_loop // stop thread because time is out and response has been sent
                    }
                }
                if better_moves.len() == 0 {
                    beta = test; // update beta
                } else {
                    alpha = test; // update alpha
                    my_moves = better_moves; // update subtrees left to search
                }
                if (beta as i32 - alpha as i32) < 2 || my_moves.len() == 1 {
                    last_test = test;
                    best_move = my_moves[0];
                    break
                }
            }
            result_sender.try_send((best_move, best_score, depth)).ok();
            if best_score == Score::MAX || best_score < Score::MIN+5 {
                break // Our last best move resulted in a terminal state, so we don't need to search deeper
            }
            depth += 1;
        }
        println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    });

    // receive results
    while time::Instant::now() < deadline {
        if let Ok(msg) = result_receiver.try_recv() {
            best_move = msg.0;
            best_score = msg.1;
            best_depth = msg.2
        } else {
            thread::sleep(time::Duration::from_millis(1));
        }
    }
    stop_sender.send(1).ok(); // Channel might be broken, if search returned early. We don't care.

    println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move, best_score, best_depth, time::Instant::now().duration_since(start_time).as_millis());
    (best_move, best_score, best_depth)
}

pub fn iterative_deepening_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, g: &mut Game) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut best_depth = 1;
    let start_time = time::Instant::now();
    let deadline = start_time + g.move_time / 2;

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
        'outer_loop: loop {
            let mut best = Score::MIN+1;
            let mut best_unused_depth = depth;
            for mv in &my_moves {
                if let Some((score, unused_depth)) = alphabeta(&board, ruleset, &mut node_counter, &stop_receiver, *mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX) {
                    if score > best || (score == best && unused_depth < best_unused_depth) {
                        best = score;
                        best_move = *mv;
                        best_unused_depth = unused_depth;
                    }
                } else {
                    break 'outer_loop
                }
            }
            result_sender.try_send((best_move, best, depth)).ok();
            if best == Score::MAX || best < Score::MIN + 5 {
                break
            }
            depth += 1;
        }
        println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    });

    // receive results
    while time::Instant::now() < deadline {
        if let Ok(msg) = result_receiver.try_recv() {
            best_move = msg.0;
            best_score = msg.1;
            best_depth = msg.2
        } else {
            thread::sleep(time::Duration::from_millis(1));
        }
    }
    stop_sender.send(1).ok(); // Channel might be broken, if search returned early. We don't care.

    println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move, best_score, best_depth, time::Instant::now().duration_since(start_time).as_millis());
    (best_move, best_score, best_depth)
}

/// Returns None if it received a timeout from stop_receiver.
pub fn alphabeta<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>,
    ruleset: Ruleset,
    node_counter: &mut u64,
    stop_receiver: &Receiver<u8>,
    mv: Move,
    enemy_moves: &mut ArrayVec<[Move; S], 4>,
    depth: u8,
    alpha: Score,
    mut beta: Score
) -> Option<(Score, u8)>
where [(); (W*H+127)/128]: Sized {  // min call
    if let Ok(_) = stop_receiver.try_recv() {
        return None
    }
    // search
    let mut best_score = Score::MAX;
    for mvs in enemy_moves { // TODO: apply move ordering
        let score = { // max call
            let mut ibest_score = Score::MIN;
            let mut ialpha = alpha;
            let ibeta = beta;
            mvs[0] = mv;
            let mut child = board.clone();
            child.apply_moves(&mvs, ruleset);
            *node_counter += 1;

            // search stops
            if child.is_terminal() {
                ibest_score = eval_terminal(&child);
            } else if depth == 1 {
                ibest_score = eval(&child, ruleset);
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
                    let (iscore, _) = alphabeta(&child, ruleset, node_counter, stop_receiver, mv, &mut next_enemy_moves, depth-1, alpha, beta)?;
                    if iscore > ibeta {
                        ibest_score = iscore;
                        break;
                    }
                    if iscore > ibest_score {
                        ibest_score = iscore;
                        if iscore > ialpha {
                            ialpha = iscore;
                        }
                    }
                }
            }
            ibest_score
        };
        if score < alpha {
            return Some((score, depth))
        }
        if score < best_score {
            best_score = score;
            if score < beta {
                beta = score;
            }
        }
    }
    Some((best_score, depth))
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
