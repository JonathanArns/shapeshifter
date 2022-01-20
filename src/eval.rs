use crate::types::*;
use crate::bitboard::*;

use std::env;

const BORDER_MASK: u128 = 0b_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110;

lazy_static! {
    /// Weights for eval function can be loaded from environment.
    static ref WEIGHTS: [Score; 7] = if let Ok(var) = env::var("WEIGHTS") {
        serde_json::from_str(&var).unwrap()
    } else {
       [-10, 1, 3, 1, 3, 0, 0]
    };
}


fn flood_fill<const N: usize>(board: &Bitboard<N>) -> u128 {
    let b = !board.bodies[0];
    let mut x = 1_u128<<board.snakes[0].head;
    let mut y = x;
    loop {
        x = b & (x | (BORDER_MASK & x)<<1 | (BORDER_MASK & x)>>1 | x<<11 | x>>11);
        if x == y {
            break
        } else {
            y = x;
        }
    }
    x
}

fn area_control<const N: usize>(board: &Bitboard<N>) -> (u128, u128) {
    let mut x = (1_u128<<board.snakes[0].head, 0_u128);
    let mut tails_mask = 1_u128<<board.snakes[0].tail;
    for snake in &board.snakes[1..] {
        if snake.is_alive() {
            x.1 |= 1<<snake.head;
            tails_mask |= 1<<snake.tail;
        }
    }
    let b = !board.bodies[0] & !tails_mask;
    let mut y = x;
    loop {
        let me = b & (x.0 | (BORDER_MASK & x.0)<<1 | (BORDER_MASK & x.0)>>1 | x.0<<11 | x.0>>11);
        let enemies = b & (x.1 | (BORDER_MASK & x.1)<<1 | (BORDER_MASK & x.1)>>1 | x.1<<11 | x.1>>11);
        x = (me & !enemies, enemies & !me);
        if x == y {
            break
        } else {
            y = x;
        }
    }
    (x.0, x.1)
}

pub fn eval<const N: usize>(board: &Bitboard<N>) -> Score {
    let mut enemies_alive = 0;
    let mut lowest_enemy_health = 100;
    let mut largest_enemy_length = 0;
    let mut tails_mask = 1_u128<<board.snakes[0].tail;

    let mut x = 1_u128<<board.snakes[0].head;
    x = x | (BORDER_MASK & x)<<1 | (BORDER_MASK & x)>>1 | x<<11 | x>>11;
    let distance_1_mask = x;
    x = x | (BORDER_MASK & x)<<1 | (BORDER_MASK & x)>>1 | x<<11 | x>>11;
    let distance_2_mask = x;
    x = x | (BORDER_MASK & x)<<1 | (BORDER_MASK & x)>>1 | x<<11 | x>>11;
    let distance_3_mask = x;

    for i in 1..N {
        if board.snakes[i].is_alive() {
            enemies_alive += 1;
            tails_mask |= 1<<board.snakes[i].tail;
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
    //close controlled area
    score += WEIGHTS[4] * (my_area & distance_3_mask).count_ones() as Score;
    // difference in controlled food
    score += WEIGHTS[5] * (my_area & board.food).count_ones() as Score - (enemy_area & board.food).count_ones() as Score;
    // number of close tails
    score += WEIGHTS[6] * (distance_2_mask & tails_mask).count_ones() as Score;

    score
}

pub fn eval_terminal<const N: usize>(board: &Bitboard<N>) -> Score {
    if !board.snakes[0].is_alive() {
        return Score::MIN - board.snakes[0].health as i16
    } else {
        return Score::MAX
    }
}
