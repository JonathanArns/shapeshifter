use crate::types::*;
use std::time;

pub fn iterative_deepening_search(b: &Board, g: &mut Game) -> (Move, Score, u8) {
    let mut best_move = (Move::Up, 0, 0);
    let start_time = time::Instant::now();
    let mut depth = 1;

    while time::Instant::now().duration_since(start_time).lt(&g.move_time.div_f32(b.snakes.len() as f32 * 10_f32)) {
        let new_best = minimax(b, depth, Score::MIN, Score::MAX);
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
            println!("{:?}", b);
            println!("====================================");
            println!("{:?}", b.children());
            println!("====================================");
            println!("{:?}", best_move);
            println!("====================================");
            panic!();
        }
    }
    println!("Move: {:?}, Depth: {}, Time: {}", best_move.0, depth, time::Instant::now().duration_since(start_time).as_millis());
    best_move
}

fn minimax(b: &Board, d: u8, mut alpha: Score, mut beta: Score) -> (Move, Score, u8) {
    let beta_init = beta;
    if d == 0 || b.is_terminal() {
        return (Move::Up, b.eval(), d)
    }
    let mut max = (Move::Up, Score::MIN, d);
    let my_moves = b.children();
    for maybe_mv in my_moves {
        if let Some((mv, positions)) = maybe_mv {
            beta = beta_init; // because the inner loop is essentially the minimizing call
            let mut min = Score::MAX;
            let mut min_depth = d;
            for position in positions {
                let (_, score, depth) = minimax(&position, d-1, alpha, beta);
                if score < min {
                    min = score;
                    min_depth = depth;
                }
                if min < beta {
                    beta = min;
                    if beta < alpha {
                        break
                    }
                }
            }
            if min > max.1 || (min == max.1 && min_depth < max.2) {
                max = (mv, min, min_depth);
                if max.1 > alpha {
                    alpha = max.1;
                    if beta < alpha {
                        break
                    }
                }
            }
        }
    }
    max
}
