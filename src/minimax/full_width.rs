use crate::bitboard::*;
use crate::bitboard::move_gen::*;
use crate::minimax::{Score, ttable, eval::eval_terminal};

use std::time;
use arrayvec::ArrayVec;

/// Runs a full width paranoid search to identify possible losses to opponent coordination
pub fn paranoid_loss_prevention<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    deadline: time::SystemTime,
    start_depth: u8,
) -> [Option<u8>; 4]
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    // let start_time = time::Instant::now();
    let mut node_counter = 0;
    let mut history = [[0; 4]; W*H];

    let board = board.clone();
    let mut enemy_moves = ordered_move_combinations(&board, 1, &history);
    let mut my_moves = allowed_moves(&board, 0);

    let mut depth = start_depth;
    let mut results = [None; 4];
    let test_val = 0;

    'outer_loop: loop {
        let mut better_moves = ArrayVec::<Move, 4>::new();
        for mv in &my_moves {
            if let Some(score) = full_width_win_loss_alphabeta(&board, &mut node_counter, deadline, *mv, &mut enemy_moves, &mut history, depth, test_val-1, test_val) {
                if score >= test_val {
                    better_moves.push(*mv);
                } else {
                    // this move is a loss after depth turns
                    results[mv.to_int() as usize] = Some(depth);
                }
            } else {
                break 'outer_loop // time has run out
            }
        }
        if better_moves.len() == 0 {
            break 'outer_loop
        } else {
            my_moves = better_moves; // update subtrees left to search
        }
        depth += 1;
    }
    // info!(
    //     game.turn = board.turn,
    //     game.mode = ?board.gamemode,
    //     search.nodes_total = node_counter,
    //     search.nodes_per_second = (node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos())) as u64,
    //     search.best_move = ?best_move,
    //     search.score = best_score,
    //     search.depth = depth,
    //     search.time_used = time::Instant::now().duration_since(start_time).as_millis() as u64,
    //     "search_finished"
    // );
    results
}

/// An iterative deepening MTD(f) without speculative pruning
pub fn fixed_depth_full_width_search<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    target_depth: u8,
    timeout_millis: u64,
) -> ((Move, Score, u8), u64)
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut node_counter = 0;
    let mut history = [[0; 4]; W*H];
    let my_moves = ordered_allowed_moves(board, 0, &history);
    let mut enemy_moves = ordered_move_combinations(&board, 1, &history);
    let mut best_move = my_moves[0];
    let start_time = time::Instant::now(); // used to calculate nodes / second
    let deadline = time::SystemTime::now() + time::Duration::from_millis(timeout_millis);
    let mut best_score = Score::MIN+1;
    for mv in &my_moves {
        let mut guess = 0;
        for depth in 1..target_depth {
            let mut bounds = [Score::MIN, Score::MAX];
            while bounds[0] < bounds[1] {
                let beta = guess + (guess == bounds[0]) as Score;
                guess = full_width_win_loss_alphabeta(&board, &mut node_counter, deadline, *mv, &mut enemy_moves, &mut history, depth, beta-1, beta).unwrap();
                bounds[(guess < beta) as usize] = guess;
            }
        }
        if guess > best_score {
            best_score = guess;
            best_move = *mv;
        }
    }
    println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    println!("Move: {:?}, Score: {}, Time: {}", best_move, best_score, time::Instant::now().duration_since(start_time).as_millis());
    ((best_move, best_score, target_depth), node_counter)
}

fn win_draw_loss_eval<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>
) -> Score
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    if board.is_terminal() {
        eval_terminal(board)
    } else {
        0
    }
}

pub fn full_width_win_loss_alphabeta<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    node_counter: &mut u64,
    deadline: time::SystemTime,
    mv: Move,
    enemy_moves: &mut Vec<[Move; S]>,
    history: &mut [[u64; 4]; W*H],
    depth: u8,
    mut alpha: Score,
    mut beta: Score
) -> Option<Score>
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {  // min call
    if time::SystemTime::now() > deadline {
        return None
    }
    // let tt_key = ttable::hash(&(board, mv));
    // let tt_entry = ttable::get(tt_key, board.tt_id);
    // let mut tt_move = None;
    // if let Some(entry) = tt_entry {
    //     if entry.get_depth() >= depth {
    //         let tt_score = entry.get_score();
    //         if entry.is_lower_bound() {
    //             alpha = alpha.max(tt_score);
    //         } else if entry.is_upper_bound() {
    //             beta = beta.min(tt_score);
    //         } else {
    //             return Some(tt_score) // got exact score
    //         }
    //         if alpha >= beta {
    //             return Some(tt_score);
    //         }
    //     }
    //     if let Some(x) = entry.get_best_moves::<S>() {
    //         // sanity check for tt_move
    //         if board.is_legal_enemy_moves(x) {
    //             tt_move = Some(x);
    //         }
    //     }
    // }

    // search
    let mut best_score = Score::MAX;
    let mut best_moves = [Move::Up; S];
    // for mvs in tt_move.iter_mut().chain(enemy_moves.iter_mut()) {
    for mvs in enemy_moves.iter_mut() {
        mvs[0] = mv;
        let score = 'max_call: { // max call
            let mut ialpha = alpha;
            let mut ibeta = beta;
            let mut ibest_score = Score::MIN;
            let mut ibest_move = Move::Up;
            let mut child = board.clone();
            (child.apply_moves.clone())(&mut child, &mvs);
            *node_counter += 1;

            // search stops
            if child.is_terminal() || depth == 1 {
                break 'max_call win_draw_loss_eval(&child);
            }
            // check TT
            // let itt_key = ttable::hash(&child);
            // let itt_entry = ttable::get(itt_key, child.tt_id);
            // let mut itt_move = None;
            // if let Some(entry) = itt_entry {
            //     if entry.get_depth() >= depth {
            //         let tt_score = entry.get_score();
            //         if entry.is_lower_bound() {
            //             ialpha = ialpha.max(tt_score);
            //         } else if entry.is_upper_bound() {
            //             ibeta = ibeta.min(tt_score);
            //         } else {
            //             break 'max_call tt_score; // got exact score
            //         }
            //         if ialpha >= ibeta {
            //             break 'max_call tt_score;
            //         }
            //     }
            //     if let Some(x) = entry.get_best_moves::<1>() {
            //         // sanity check for itt_move
            //         if board.is_legal_move(board.snakes[0].head, x[0]) {
            //             itt_move = Some(x[0]);
            //         }
            //     }
            // }

            // continue search
            let mut iseen_moves = ArrayVec::<Move, 4>::default();
            let mut next_enemy_moves = ordered_move_combinations(&child, 1, history);
            // for mv in itt_move.iter().chain(ordered_allowed_moves(&child, 0, history).iter()) {
            for mv in ordered_allowed_moves(&child, 0, history).iter() {
                if iseen_moves.contains(mv) {
                    continue
                } else {
                    iseen_moves.push(*mv);
                }
                let iscore = full_width_win_loss_alphabeta(&child, node_counter, deadline, *mv, &mut next_enemy_moves, history, depth-1, ialpha, ibeta)?;
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
            // ttable::insert(itt_key, child.tt_id, ibest_score, ibest_score >= ibeta, ibest_score <= ialpha, depth, [ibest_move; 1]);
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
    // ttable::insert(tt_key, board.tt_id, best_score, best_score >= beta, best_score <= alpha, depth, best_moves);
    // update history heuristic
    for i in 1..S {
        if board.snakes[i].is_alive() {
            history[board.snakes[i].head as usize][best_moves[i].to_int() as usize] += depth as u64;
        }
    }
    Some(best_score)
}
