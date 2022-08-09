use crate::bitboard::*;
use crate::bitboard::move_gen::*;

use std::env;
use std::time;
use arrayvec::ArrayVec;
use rand::seq::SliceRandom;
use tracing::{info, debug};

mod eval;
mod ttable;

pub use ttable::{init, get_tt_id};
#[cfg(feature = "training")]
pub use eval::set_training_weights;

lazy_static! {
    static ref FIXED_DEPTH: i8 = if let Ok(var) = env::var("FIXED_DEPTH") {
        var.parse().unwrap()
    } else {
        -1
    };
}

pub type Score = i16;

pub fn search<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, deadline: time::SystemTime) -> (Move, Score, u8)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    if *FIXED_DEPTH > 0 {
        fixed_depth_search(board, *FIXED_DEPTH as u8)
    } else {
        best_node_search(board, deadline)
    }
}

/// An iterative deepening MTD(f)
pub fn fixed_depth_search<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    target_depth: u8
) -> (Move, Score, u8)
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut node_counter = 0;
    let mut history = [[0; 4]; W*H];
    let my_moves = ordered_allowed_moves(board, 0, &history);
    let mut enemy_moves = ordered_limited_move_combinations(&board, 1, &history);
    let mut best_move = my_moves[0];
    let start_time = time::Instant::now(); // used to calculate nodes / second
    let deadline = time::SystemTime::now() + time::Duration::from_millis(500000);
    let mut best_score = Score::MIN+1;
    for mv in &my_moves {
        let mut guess = 0;
        for depth in 1..=target_depth {
            let mut bounds = [Score::MIN, Score::MAX];
            while bounds[0] < bounds[1] {
                let beta = guess + (guess == bounds[0]) as Score;
                guess = alphabeta(&board, &mut node_counter, deadline, *mv, &mut enemy_moves, &mut history, depth, beta-1, beta).unwrap();
                bounds[(guess < beta) as usize] = guess;
            }
        }
        if guess > best_score {
            best_score = guess;
            best_move = *mv;
        }
    }
    info!(
        game.turn = board.turn,
        game.mode = ?board.gamemode,
        search.nodes_total = node_counter,
        search.nodes_per_second = (node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos())) as u64,
        search.best_move = ?best_move,
        search.score = best_score,
        search.depth = target_depth,
        search.time_used = time::Instant::now().duration_since(start_time).as_millis() as u64,
        "fixed_depth_search_finished"
    );
    (best_move, best_score, target_depth)
}

/// An iterative deepening MTD(f)
pub fn mtdf<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    deadline: time::SystemTime
) -> (Move, Score, u8)
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let start_time = time::Instant::now(); // used to calculate nodes / second
    let mut node_counter = 0;
    let mut rng = rand::thread_rng();
    let mut depth = 1;
    let mut history = [[0; 4]; W*H];
    let mut my_moves = allowed_moves(board, 0);
    my_moves.shuffle(&mut rng);
    let mut enemy_moves = ordered_limited_move_combinations(&board, 1, &history);
    let mut best_move = my_moves[0];
    let mut best_score = Score::MIN+1;
    'outer_loop: loop {
        let mut best_move_candidate = my_moves[0];
        let mut best_score_candidate = Score::MIN+1;
        for mv in &my_moves {
            let mut guess = 0;
            let mut bounds = [Score::MIN, Score::MAX];
            while bounds[0] < bounds[1] {
                let beta = guess + (guess == bounds[0]) as Score;
                if let Some(score) = alphabeta(&board, &mut node_counter, deadline, *mv, &mut enemy_moves, &mut history, depth, beta-1, beta) {
                    guess = score;
                    bounds[(guess < beta) as usize] = guess;
                } else {
                    break 'outer_loop
                }
            }
            if guess > best_score_candidate {
                best_score_candidate = guess;
                best_move_candidate = *mv;
            }
        }
        // if best_score_candidate > best_score {
            best_score = best_score_candidate;
            best_move = best_move_candidate;
        // }
        if best_score > Score::MAX-1000 || best_score < Score::MIN+1000 || depth == u8::MAX {
            break // Our last best move resulted in a terminal state, so we don't need to search deeper
        }
        depth += 1;
    }
    info!(
        game.turn = board.turn,
        game.mode = ?board.gamemode,
        search.nodes_total = node_counter,
        search.nodes_per_second = (node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos())) as u64,
        search.best_move = ?best_move,
        search.score = best_score,
        search.depth = depth,
        search.time_used = time::Instant::now().duration_since(start_time).as_millis() as u64,
        "fixed_depth_search_finished"
    );
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

pub fn best_node_search<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    deadline: time::SystemTime
) -> (Move, Score, u8)
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut rng = rand::thread_rng();
    let start_time = time::Instant::now();
    let mut node_counter = 0;
    let mut history = [[0; 4]; W*H];

    let board = board.clone();
    let mut enemy_moves = ordered_limited_move_combinations(&board, 1, &history);
    let mut my_allowed_moves = allowed_moves(&board, 0);
    my_allowed_moves.shuffle(&mut rng);
    if my_allowed_moves.len() == 1 {
        debug!(
            game.turn = board.turn,
            game.mode = ?board.gamemode,
            search.best_move = ?my_allowed_moves[0],
            "returned_only_move"
        );
        return (my_allowed_moves[0], 0, 0)
    }

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
                if let Some(score) = alphabeta(&board, &mut node_counter, deadline, *mv, &mut enemy_moves, &mut history, depth, test-1, test) {
                    if score >= test {
                        better_moves.push(*mv);
                    }
                } else {
                    depth -= 1;
                    break 'outer_loop // time has run out
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
    info!(
        game.turn = board.turn,
        game.mode = ?board.gamemode,
        search.nodes_total = node_counter,
        search.nodes_per_second = (node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos())) as u64,
        search.best_move = ?best_move,
        search.score = best_score,
        search.depth = depth,
        search.time_used = time::Instant::now().duration_since(start_time).as_millis() as u64,
        "search_finished"
    );
    (best_move, best_score, depth)
}

pub fn alphabeta<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    node_counter: &mut u64,
    deadline: time::SystemTime,
    mv: Move,
    enemy_moves: &mut ArrayVec<[Move; S], 4>,
    history: &mut [[u64; 4]; W*H],
    depth: u8,
    mut alpha: Score,
    mut beta: Score
) -> Option<Score>
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {  // min call
    if time::SystemTime::now() > deadline {
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
        if let Some(x) = entry.get_best_moves::<S>() {
            // sanity check for tt_move
            if board.is_legal_enemy_moves(x) {
                tt_move = Some(x);
            }
        }
    }

    // search
    let mut best_score = Score::MAX;
    let mut best_moves = [Move::Up; S];
    let mut seen_moves = ArrayVec::<[Move; S], 4>::default();
    for mvs in tt_move.iter_mut().chain(enemy_moves.iter_mut()) {
        mvs[0] = mv;
        if seen_moves.contains(&mvs) {
            continue
        } else {
            seen_moves.push(mvs.clone());
        }
        let score = 'max_call: { // max call
            let mut ialpha = alpha;
            let mut ibeta = beta;
            let mut ibest_score = Score::MIN;
            let mut ibest_move = Move::Up;
            let mut child = board.clone();
            (child.apply_moves.clone())(&mut child, &mvs);
            *node_counter += 1;

            // search stops
            if child.is_terminal() {
                break 'max_call eval::eval_terminal(&child);
            } else if depth == 1 && (get_quiescence_params(board.gamemode).1)(&child) { // calls is_stable
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
                    // sanity check for itt_move
                    if board.is_legal_move(board.snakes[0].head, x[0]) {
                        itt_move = Some(x[0]);
                    }
                }
            }

            // continue search
            let mut iseen_moves = ArrayVec::<Move, 4>::default();
            let mut next_enemy_moves = ordered_limited_move_combinations(&child, 1, history);
            for mv in itt_move.iter().chain(ordered_allowed_moves(&child, 0, history).iter()) {
                if iseen_moves.contains(mv) {
                    continue
                } else {
                    iseen_moves.push(*mv);
                }
                let iscore = if depth == 1 {
                    let (q_depth, is_stable) = get_quiescence_params(board.gamemode);
                    quiescence(&child, node_counter, deadline, is_stable, *mv, &mut next_enemy_moves, history, q_depth, ialpha, ibeta)?
                } else {
                    alphabeta(&child, node_counter, deadline, *mv, &mut next_enemy_moves, history, depth-1, ialpha, ibeta)?
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
            // update history heuristic
            history[board.snakes[0].head as usize][ibest_move.to_int() as usize] += depth as u64;
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
    // update history heuristic
    for i in 1..S {
        if board.snakes[i].is_alive() {
            history[board.snakes[i].head as usize][best_moves[i].to_int() as usize] += depth as u64;
        }
    }
    Some(best_score)
}

/// Used for quiescence search, to determine, if the position is stable and can be evaluated, or if
/// search must continue.
fn get_quiescence_params<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    mode: Gamemode
) -> (u8, fn(&Bitboard<S, W, H, WRAP, HZSTACK>) -> bool)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    match mode {
        Gamemode::WrappedArcadeMaze => (20, |board| {
            let mut moves = 1;
            for (i, snake) in board.snakes.iter().enumerate() {
                if snake.is_dead() {
                    continue
                }
                moves *= allowed_moves(board, i).len();
            }
            moves > 2
        }),
        Gamemode::Constrictor => (0, |_| true),
        _ => (3, |board| {
            for snake in board.snakes {
                if snake.is_dead() {
                    continue
                }
                if snake.curled_bodyparts != 0 {
                    return false
                }
                for i in 0..4 {
                    if let Some(pos) = Bitboard::<S, W, H, WRAP, HZSTACK>::MOVES_FROM_POSITION[snake.head as usize][i] {
                        if board.food.get_bit(pos as usize) {
                            return false
                        }
                    }
                }
            }
            true
        }),
    }
}

/// Returns None if it received a timeout from stop_receiver.
pub fn quiescence<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    node_counter: &mut u64,
    deadline: time::SystemTime,
    is_stable: fn (&Bitboard<S, W, H, WRAP, HZSTACK>) -> bool,
    mv: Move,
    enemy_moves: &mut ArrayVec<[Move; S], 4>,
    history: &mut [[u64; 4]; W*H],
    depth: u8,
    alpha: Score,
    mut beta: Score
) -> Option<Score>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {  // min call
    if time::SystemTime::now() > deadline {
        return None
    }

    let mut best_score = Score::MAX;
    let mut best_moves = [Move::Up; S];
    for mvs in enemy_moves.iter_mut() {
        let score = 'max_call: { // max call
            let mut ialpha = alpha;
            let ibeta = beta;
            let mut ibest_score = Score::MIN;
            let mut ibest_move = Move::Up;
            mvs[0] = mv;
            let mut child = board.clone();
            (child.apply_moves.clone())(&mut child, &mvs);
            *node_counter += 1;

            // search stops
            if child.is_terminal() {
                break 'max_call eval::eval_terminal(&child);
            } else if depth == 1 || is_stable(&child) {
                break 'max_call eval::eval(&child);
            }

            // continue search
            let mut next_enemy_moves = ordered_limited_move_combinations(&child, 1, history);
            for mv in &ordered_allowed_moves(&child, 0, history) {
                let iscore = quiescence(&child, node_counter, deadline, is_stable, *mv, &mut next_enemy_moves, history, depth-1, ialpha, ibeta)?;
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
            // update history heuristic
            history[board.snakes[0].head as usize][ibest_move.to_int() as usize] += depth as u64;
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
    // update history heuristic
    for i in 1..S {
        if board.snakes[i].is_alive() {
            history[board.snakes[i].head as usize][best_moves[i].to_int() as usize] += depth as u64;
        }
    }
    Some(best_score)
}
