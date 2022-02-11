use crate::types::*;
use crate::bitboard::*;
use crate::move_gen::*;
use crate::eval::*;
use crate::ttable;

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

pub fn search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, deadline: time::Instant) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    if *FIXED_DEPTH > 0 {
        fixed_depth_search(board, *FIXED_DEPTH as u8)
    } else {
        // iterative_deepening_search(board, deadline)
        best_node_search(board, deadline)
    }
}

pub fn fixed_depth_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, depth: u8) -> (Move, Score, u8)
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
        let score = alphabeta(board, &mut node_counter, &stop_receiver, *mv, &mut enemy_moves, depth, 0, Score::MIN+1, Score::MAX).unwrap();
        if score > best {
            best = score;
            best_move = *mv;
            best_score = best;
        }
    }
    println!("{} nodes total, {} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    (best_move, best_score, depth)
}

fn next_bns_guess(prev_guess: Score, alpha: Score, beta: Score, subtree_count: usize) -> Score {
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
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut best_depth = 1;
    let start_time = time::Instant::now();

    let (stop_sender, stop_receiver) = unbounded();
    let (result_sender, result_receiver) : (Sender<(Move, Score, u8)>, Receiver<(Move, Score, u8)>) = unbounded();

    let board = board.clone();
    thread::spawn(move || {
        let mut node_counter = 0;
        let start_time = time::Instant::now(); // only used to calculate nodes / second
        let mut depth = 1;
        let mut enemy_moves = limited_move_combinations(&board, 1);
        let mut last_test = 0;
        'outer_loop: loop {
            let mut my_moves = allowed_moves(&board, board.snakes[0].head);
            // let mut alpha = Score::MIN+1;
            // let mut beta = Score::MAX;
            let mut alpha = -1000;
            let mut beta = 1000;
            let best_move;
            loop {
                let test = next_bns_guess(last_test, alpha, beta, my_moves.len());
                let mut better_moves = ArrayVec::<Move, 4>::new();
                for mv in &my_moves {
                    if let Some(score) = alphabeta(&board, &mut node_counter, &stop_receiver, *mv, &mut enemy_moves, depth, 0, test-1, test) {
                        // println!("test: {}, score: {}, move: {:?}, alpha: {}, beta: {}", test, score, *mv, alpha, beta);
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
            if best_score > Score::MAX-20 || best_score < Score::MIN+20 || depth == u8::MAX {
                break // Our last best move resulted in a terminal state, so we don't need to search deeper
            }
            depth += 1;
        }
        #[cfg(not(feature = "detect_hash_collisions"))]
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

    #[cfg(not(feature = "detect_hash_collisions"))]
    println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move, best_score, best_depth, time::Instant::now().duration_since(start_time).as_millis());
    (best_move, best_score, best_depth)
}

pub fn iterative_deepening_search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, deadline: time::Instant) -> (Move, Score, u8)
where [(); (W*H+127)/128]: Sized {
    let mut best_move = Move::Up;
    let mut best_score = Score::MIN+1;
    let mut best_depth = 1;
    let start_time = time::Instant::now();

    let (stop_sender, stop_receiver) = unbounded();
    let (result_sender, result_receiver) : (Sender<(Move, Score, u8)>, Receiver<(Move, Score, u8)>) = unbounded();

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
            for mv in &my_moves {
                if let Some(score) = alphabeta(&board, &mut node_counter, &stop_receiver, *mv, &mut enemy_moves, depth, 0, Score::MIN+1, Score::MAX) {
                    if score > best {
                        best = score;
                        best_move = *mv;
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
        #[cfg(not(feature = "detect_hash_collisions"))]
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

    #[cfg(not(feature = "detect_hash_collisions"))]
    println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move, best_score, best_depth, time::Instant::now().duration_since(start_time).as_millis());
    (best_move, best_score, best_depth)
}

/// Returns None if it received a timeout from stop_receiver.
pub fn alphabeta<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>,
    node_counter: &mut u64,
    stop_receiver: &Receiver<u8>,
    mv: Move,
    enemy_moves: &mut ArrayVec<[Move; S], 4>,
    depth: u8,
    depth_searched: u8,
    mut alpha: Score,
    mut beta: Score
) -> Option<Score>
where [(); (W*H+127)/128]: Sized {  // min call
    if let Ok(_) = stop_receiver.try_recv() {
        return None
    }
    // let tt_entry = ttable::get(&(board, mv));
    // if let Some(entry) = tt_entry {
    //     if entry.get_depth() >= depth {
    //         let tt_score = entry.get_score();
    //         if entry.is_exact() {
    //             return Some(tt_score)
    //         } else if entry.is_lower_bound() {
    //             alpha = alpha.max(tt_score);
    //         } else if entry.is_upper_bound() && tt_score < beta {
    //             beta = beta.min(tt_score);
    //         }
    //         if alpha >= beta {
    //             return Some(tt_score);
    //         }
    //     }
    // }

    // search
    let mut best_score = Score::MAX;
    let mut best_moves = [Move::Up; S];
    for mvs in enemy_moves { // TODO: apply move ordering
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
                break 'max_call eval_terminal(&child, depth_searched);
            } else if depth == 1 {
                // TODO: insert into TT and move TT check to before?
                break 'max_call eval(&child);
            }
            // check TT
            // let itt_entry = ttable::get(&(board, mv));
            // if let Some(entry) = itt_entry {
            //     if entry.get_depth() >= depth {
            //         let tt_score = entry.get_score();
            //         if entry.is_exact() {
            //             break 'max_call tt_score;
            //         } else if entry.is_lower_bound() {
            //             ialpha = ialpha.max(tt_score);
            //         } else if entry.is_upper_bound() && tt_score < beta {
            //             ibeta = ibeta.min(tt_score);
            //         }
            //         if alpha >= beta {
            //             return Some(tt_score);
            //         }
            //     }
            //     // if let Some(mv) = entry.get_best_moves::<1>() {
            //     //     let my_mv = mv[0];
            //     // }
            // }

            // continue search
            let mut next_enemy_moves = limited_move_combinations(&child, 1);
            for mv in allowed_moves(&child, child.snakes[0].head) { // TODO: apply move ordering
                let iscore = alphabeta(&child, node_counter, stop_receiver, mv, &mut next_enemy_moves, depth-1, depth_searched+1, alpha, beta)?;
                if iscore > ibeta {
                    ibest_score = iscore;
                    ibest_move = mv;
                    break;
                }
                if iscore > ibest_score {
                    ibest_score = iscore;
                    ibest_move = mv;
                    if iscore > ialpha {
                        ialpha = iscore;
                    }
                }
            }
            // ttable::insert(board, ibest_score, ibest_score >= ibeta, ibest_score <= ialpha, depth, [ibest_move; 1]);
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
    // ttable::insert(&(board, mv), best_score, best_score >= beta, best_score <= alpha, depth, best_moves); // TODO: is &board hashed like I expect?
    Some(best_score)
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
