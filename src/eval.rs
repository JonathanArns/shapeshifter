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

const fn border_mask<const W: usize, const H: usize>(left: bool) -> Bitset<{W*H}>
where [(); (W*H+127)/128]: Sized {
    let mut arr = [0_u128; (W*H+127)/128];
    let mut i = 0;
    let mut j;
    loop {
        if i == H {
            break
        }
        if left {
            j = 0;
        } else {
            j = 1;
        }
        loop {
            if left && j == W-1 {
                break
            } else if !left && j == W {
                break
            }
            let idx = (i*W+j)>>7;
            let offset = (i*W+j) % 128;
            arr[idx] |= 1_u128<<offset;

            j += 1;
        }
        i += 1;
    }
    Bitset::<{W*H}>::from_array(arr)
}

struct BorderMaskHelper<const W: usize, const H: usize> {}

impl<const W: usize, const H: usize> BorderMaskHelper<W, H>
where [(); (W*H+127)/128]: Sized {
    const LEFT_BORDER_MASK: Bitset<{W*H}> = border_mask::<W, H>(true);
    const RIGHT_BORDER_MASK: Bitset<{W*H}> = border_mask::<W, H>(false);

    fn area_control<const S: usize>(board: &Bitboard<S, W, H>) -> (Bitset<{W*H}>, Bitset<{W*H}>)
    where [(); (W*H+127)/128]: Sized {
        let mut x = (Bitset::<{W*H}>::with_bit_set(board.snakes[0].head as usize), Bitset::<{W*H}>::new());
        let mut b = !board.bodies[0];
        b.set_bit(board.snakes[0].tail as usize);
        for snake in &board.snakes[1..] {
            if snake.is_alive() {
                x.1.set_bit(snake.head as usize);
                b.set_bit(snake.tail as usize);
            }
        }
        let mut y = x;
        loop {
            let me = b & (x.0 | (Self::LEFT_BORDER_MASK & x.0)<<1 | (Self::RIGHT_BORDER_MASK & x.0)>>1 | x.0<<W | x.0>>W);
            let enemies = b & (x.1 | (Self::LEFT_BORDER_MASK & x.1)<<1 | (Self::RIGHT_BORDER_MASK & x.1)>>1 | x.1<<W | x.1>>W);
            x = (me & !enemies, enemies & !me);
            if x == y {
                break
            } else {
                y = x;
            }
        }
        (x.0, x.1)
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


pub fn eval<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>) -> Score
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
    let (my_area, enemy_area) = BorderMaskHelper::<W, H>::area_control(board);

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
