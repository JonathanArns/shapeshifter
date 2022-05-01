use crate::bitboard::*;
use crate::bitboard::move_gen::*;

use std::env;
use std::time;
use arrayvec::ArrayVec;
use rand::seq::SliceRandom;

mod eval;
mod ttable;

pub use ttable::{init, get_tt_id};

const QUIESCENCE_DEPTH: u8 = 3;

lazy_static! {
    static ref FIXED_DEPTH: i8 = if let Ok(var) = env::var("FIXED_DEPTH") {
        var.parse().unwrap()
    } else {
        -1
    };
}

pub type Score = i16;

pub fn search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, deadline: time::Instant) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    if *FIXED_DEPTH > 0 {
        fixed_depth_search(board, *FIXED_DEPTH as u8)
    } else {
        best_node_search(board, deadline)
    }
}

pub fn fixed_depth_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, depth: u8) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    let mut node_counter = 0;
    let start_time = time::Instant::now(); // only used to calculate nodes / second
    let deadline = start_time + time::Duration::from_secs(5);
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut enemy_moves = ordered_limited_move_combinations(board, 1);
    let my_moves = allowed_moves(board, board.snakes[0].head);
    let mut best = Score::MIN+1;
    for mv in &my_moves {
        let score = alphabeta(board, &mut node_counter, deadline, *mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX).unwrap();
        if score > best {
            best = score;
            best_move = *mv;
            best_score = best;
        }
    }
    println!("Move: {:?}, Score: {}", best_move, best_score);
    println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    (best_move, best_score, depth)
}

fn next_bns_guess(prev_guess: Score, alpha: Score, beta: Score) -> Score {
    if prev_guess > alpha && prev_guess < beta {
        return prev_guess
    }
    let test = alpha / 2 + beta / 2;
    if test == beta {
        test - 1
    } else if test == alpha {
        test + 1
    } else {
        test
    }
}

pub fn best_node_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, deadline: time::Instant) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    let mut rng = rand::thread_rng();
    let start_time = time::Instant::now();
    let mut node_counter = 0;

    let board = board.clone();
    let mut enemy_moves = ordered_limited_move_combinations(&board, 1);
    let mut my_allowed_moves = allowed_moves(&board, board.snakes[0].head);
    my_allowed_moves.shuffle(&mut rng);
    let mut best_move = my_allowed_moves[0];
    let mut best_score = Score::MIN+1;
    let mut depth = 1;

    let mut last_test = 0;
    'outer_loop: loop {
        let mut my_moves = my_allowed_moves.clone();
        let mut alpha = Score::MIN;
        let mut beta = Score::MAX;
        loop {
            let test = next_bns_guess(last_test, alpha, beta);
            let mut better_moves = ArrayVec::<Move, 4>::new();
            for mv in &my_moves {
                if let Some(score) = alphabeta(&board, &mut node_counter, deadline, *mv, &mut enemy_moves, depth, test-1, test) {
                    // println!("test: {}, score: {}, move: {:?}, alpha: {}, beta: {}", test, score, *mv, alpha, beta);
                    if score >= test {
                        better_moves.push(*mv);
                    }
                } else {
                    depth -= 1;
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
                best_score = test;
                best_move = my_moves[0];
                break
            }
        }
        if best_score > Score::MAX-1000 || best_score < Score::MIN+1000 || depth == u8::MAX {
            break // Our last best move resulted in a terminal state, so we don't need to search deeper
        }
        depth += 1;
    }
    #[cfg(not(feature = "detect_hash_collisions"))]
    println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    #[cfg(not(feature = "detect_hash_collisions"))]
    println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move, best_score, depth, time::Instant::now().duration_since(start_time).as_millis());
    (best_move, best_score, depth)
}

/// Returns None if it received a timeout from stop_receiver.
pub fn alphabeta<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>,
    node_counter: &mut u64,
    deadline: time::Instant,
    mv: Move,
    enemy_moves: &mut ArrayVec<[Move; S], 4>,
    depth: u8,
    mut alpha: Score,
    mut beta: Score
) -> Option<Score>
where [(); (W*H+127)/128]: Sized {  // min call
    if time::Instant::now() > deadline {
        return None
    }
    let tt_key = ttable::hash(&(board, mv));
    let tt_entry = ttable::get(tt_key, board.tt_id);
    let mut tt_move = None;
    if let Some(entry) = tt_entry {
        if entry.get_depth() >= depth {
            let tt_score = entry.get_score();
            if entry.is_lower_bound() {
                alpha = alpha.max(tt_score);
            } else if entry.is_upper_bound() {
                beta = beta.min(tt_score);
            } else {
                return Some(tt_score) // got exact score
            }
            if alpha >= beta {
                return Some(tt_score);
            }
        }
        tt_move = entry.get_best_moves::<S>();
    }

    // search
    let mut best_score = Score::MAX;
    let mut best_moves = [Move::Up; S];
    for mvs in tt_move.iter_mut().chain(enemy_moves.iter_mut()) {
        let score = 'max_call: { // max call
            let mut ialpha = alpha;
            let mut ibeta = beta;
            let mut ibest_score = Score::MIN;
            let mut ibest_move = Move::Up;
            mvs[0] = mv;
            let mut child = board.clone();
            child.apply_moves(&mvs);
            *node_counter += 1;

            // search stops
            if child.is_terminal() {
                break 'max_call eval::eval_terminal(&child);
            } else if depth == 1 && is_stable(&child) {
                // TODO: insert into TT and move TT check to before?
                break 'max_call eval::eval(&child);
            }
            // check TT
            let itt_key = ttable::hash(&child);
            let itt_entry = ttable::get(itt_key, child.tt_id);
            let mut itt_move = None;
            if let Some(entry) = itt_entry {
                if entry.get_depth() >= depth {
                    let tt_score = entry.get_score();
                    if entry.is_lower_bound() {
                        ialpha = ialpha.max(tt_score);
                    } else if entry.is_upper_bound() {
                        ibeta = ibeta.min(tt_score);
                    } else {
                        break 'max_call tt_score; // got exact score
                    }
                    if ialpha >= ibeta {
                        break 'max_call tt_score;
                    }
                }
                if let Some(x) = entry.get_best_moves::<1>() {
                    itt_move = Some(x[0]);
                }
            }

            // continue search
            let mut next_enemy_moves = ordered_limited_move_combinations(&child, 1);
            for mv in itt_move.iter().chain(allowed_moves(&child, child.snakes[0].head).iter()) {
                let iscore = if depth == 1 {
                    quiescence(&child, node_counter, deadline, *mv, &mut next_enemy_moves, QUIESCENCE_DEPTH, ialpha, ibeta)?
                } else {
                    alphabeta(&child, node_counter, deadline, *mv, &mut next_enemy_moves, depth-1, ialpha, ibeta)?
                };
                if iscore > ibeta {
                    ibest_score = iscore;
                    ibest_move = *mv;
                    break;
                }
                if iscore > ibest_score {
                    ibest_score = iscore;
                    ibest_move = *mv;
                    if iscore > ialpha {
                        ialpha = iscore;
                    }
                }
            }
            ttable::insert(itt_key, child.tt_id, ibest_score, ibest_score >= ibeta, ibest_score <= ialpha, depth, [ibest_move; 1]);
            ibest_score
        };
        if score < alpha {
            best_score = score;
            best_moves = *mvs;
            break;
        }
        if score < best_score {
            best_score = score;
            best_moves = *mvs;
            if score < beta {
                beta = score;
            }
        }
    }
    ttable::insert(tt_key, board.tt_id, best_score, best_score >= beta, best_score <= alpha, depth, best_moves);
    Some(best_score)
}

/// Used for quiescence search, to determine, if the position is stable and can be evaluated, or if
/// search must continue.
fn is_stable<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> bool
where [(); (W*H+127)/128]: Sized {
    for snake in board.snakes {
        if snake.curled_bodyparts != 0 {
            return false
        }
        for i in 0..4 {
            if let Some(pos) = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[snake.head as usize][i] {
                if board.food.get_bit(pos as usize) {
                    return false
                }
            }
        }
        
    }
    true
}

/// Returns None if it received a timeout from stop_receiver.
pub fn quiescence<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>,
    node_counter: &mut u64,
    deadline: time::Instant,
    mv: Move,
    enemy_moves: &mut ArrayVec<[Move; S], 4>,
    depth: u8,
    alpha: Score,
    mut beta: Score
) -> Option<Score>
where [(); (W*H+127)/128]: Sized {  // min call
    if time::Instant::now() > deadline {
        return None
    }

    let mut best_score = Score::MAX;
    for mvs in enemy_moves.iter_mut() {
        let score = 'max_call: { // max call
            let mut ialpha = alpha;
            let ibeta = beta;
            let mut ibest_score = Score::MIN;
            mvs[0] = mv;
            let mut child = board.clone();
            child.apply_moves(&mvs);
            *node_counter += 1;

            // search stops
            if child.is_terminal() {
                break 'max_call eval::eval_terminal(&child);
            } else if depth == 1 || is_stable(&child) {
                break 'max_call eval::eval(&child);
            }

            // continue search
            let mut next_enemy_moves = ordered_limited_move_combinations(&child, 1); // No idea why, but using unordered movegen here is a big improvement
            for mv in &allowed_moves(&child, child.snakes[0].head) {
                let iscore = quiescence(&child, node_counter, deadline, *mv, &mut next_enemy_moves, depth-1, ialpha, ibeta)?;
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
            ibest_score
        };
        if score < alpha {
            best_score = score;
            break;
        }
        if score < best_score {
            best_score = score;
            if score < beta {
                beta = score;
            }
        }
    }
    Some(best_score)
}
