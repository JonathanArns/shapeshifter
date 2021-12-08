use crate::types::*;
use std::time;
use std::marker::Sized;
use std::fmt::Debug;

pub fn iterative_deepening_search<T: Board + Debug + Sized>(b: T, g: &mut Game) -> (Move, Score, u8) {
    let mut best_move = (Move::Up, 0, 0);
    let start_time = time::Instant::now();
    let mut depth = 1;

    while time::Instant::now().duration_since(start_time).lt(&g.move_time.div_f32(b.num_snakes() as f32 * 10_f32)) {
        let new_best = b.alphabeta(depth, Score::MIN, Score::MAX);
        best_move = new_best;
        if best_move.1 == Score::MAX || best_move.1 == Score::MIN {
            break
        }
        depth += 1;

        // debug
        if depth > 100 {
            println!("====================================");
            println!("STACKOVERFLOW");
            println!("====================================");
            panic!();
        }
    }
    println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move.0, best_move.1, depth, time::Instant::now().duration_since(start_time).as_millis());
    best_move
}

