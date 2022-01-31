use crate::types::*;
use crate::bitboard::*;
use crate::bitset::Bitset;

use std::env;

lazy_static! {
    /// Weights for eval function can be loaded from environment.
    static ref WEIGHTS: [Score; 5] = if let Ok(var) = env::var("WEIGHTS") {
        serde_json::from_str(&var).unwrap()
    } else {
       [-10, 1, 3, 1, 3]
    };
}
// pub static mut WEIGHTS: [Score; 5] = [-10, 1, 3, 1, 3];

fn area_control<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>) -> (Bitset<{W*H}>, Bitset<{W*H}>)
where [(); (W*H+127)/128]: Sized {
    let mut debug_counter = 0;
    let mut x = (Bitset::<{W*H}>::with_bit_set(board.snakes[0].head as usize), Bitset::<{W*H}>::new());
    let mut b = !board.bodies[0];
    b.set_bit(board.snakes[0].tail as usize);
    for snake in &board.snakes[1..] {
        if snake.is_alive() {
            x.1.set_bit(snake.head as usize);
            b.set_bit(snake.tail as usize);
        }
    }
    let mut y = x; // x at n-1
    loop {
        debug_counter += 1;
        debug_assert!(debug_counter < 10000, "endless loop in area_control\n{:?}\n{:?}", x, y);
        let mut me = b & (x.0 | (Bitboard::<S, W, H>::ALL_BUT_LEFT_EDGE_MASK & x.0)<<1 | (Bitboard::<S, W, H>::ALL_BUT_RIGHT_EDGE_MASK & x.0)>>1 | x.0<<W | x.0>>W);
        let mut enemies = b & (x.1 | (Bitboard::<S, W, H>::ALL_BUT_LEFT_EDGE_MASK & x.1)<<1 | (Bitboard::<S, W, H>::ALL_BUT_RIGHT_EDGE_MASK & x.1)>>1 | x.1<<W | x.1>>W);
        if board.wrap {
            me |= (Bitboard::<S, W, H>::LEFT_EDGE_MASK & x.0) >> (W-1)
                | (Bitboard::<S, W, H>::RIGHT_EDGE_MASK & x.0) << (W-1)
                | (Bitboard::<S, W, H>::BOTTOM_EDGE_MASK & x.0) << ((H-1)*W)
                | (Bitboard::<S, W, H>::TOP_EDGE_MASK & x.0) >> ((H-1)*W);
            enemies |= (Bitboard::<S, W, H>::LEFT_EDGE_MASK & x.1) >> (W-1)
                | (Bitboard::<S, W, H>::RIGHT_EDGE_MASK & x.1) << (W-1)
                | (Bitboard::<S, W, H>::BOTTOM_EDGE_MASK & x.1) << ((H-1)*W) // debug changes
                | (Bitboard::<S, W, H>::TOP_EDGE_MASK & x.1) >> ((H-1)*W);
        }
        x = (x.0 | (Bitboard::<S, W, H>::FULL_BOARD_MASK & (me & !enemies)), x.1 | (Bitboard::<S, W, H>::FULL_BOARD_MASK & (enemies & !me)));
        if x == y {
            return x
        } else {
            y = x;
        }
    }
}

#[allow(unused)]
fn print_area_control<const W: usize, const H: usize>(me: Bitset<{W*H}>, enemies: Bitset<{W*H}>)
where [(); (W*H+127)/128]: Sized {
    let mut debug = "".to_string();
    for i in 0..H {
        for j in 0..W {
            debug.push_str(if me.get_bit(W*(W-1-i)+j) { "x " } else if enemies.get_bit(W*(W-1-i)+j) { "o " } else { ". " });
        }
        debug.push_str("\n");
    }
    println!("{}", debug);
}


pub fn eval<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>, ruleset: Ruleset) -> Score
where [(); (W*H+127)/128]: Sized {
    let mut enemies_alive = 0;
    let mut lowest_enemy_health = 100;
    let mut largest_enemy_length = 0;

    for i in 1..S {
        if board.snakes[i].is_alive() {
            enemies_alive += 1;
            let len = board.snakes[i].length;
            if len > largest_enemy_length {
                largest_enemy_length = len;
            }
            if board.snakes[i].health < lowest_enemy_health {
                lowest_enemy_health = board.snakes[i].health;
            }
        }
    }
    let (my_area, enemy_area) = area_control(board);

    let mut score: Score = 0;
    // number of enemies alive
    score += WEIGHTS[0] * enemies_alive as Score;
    // difference in health to lowest enemy
    score += WEIGHTS[1] * board.snakes[0].health as Score - lowest_enemy_health as Score;
    // difference in length to longest enemy
    score += WEIGHTS[2] * board.snakes[0].length as Score - largest_enemy_length as Score;
    // difference in controlled non-hazard area
    score += WEIGHTS[3] * (my_area & !board.hazards).count_ones() as Score - (enemy_area & !board.hazards).count_ones() as Score;
    // difference in controlled food
    score += WEIGHTS[4] * (my_area & board.food).count_ones() as Score - (enemy_area & board.food).count_ones() as Score;

    score
}

pub fn eval_terminal<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>) -> Score
where [(); (W*H+127)/128]: Sized {
    if !board.snakes[0].is_alive() {
        return Score::MIN - board.snakes[0].health as Score
    } else {
        return Score::MAX
    }
}
