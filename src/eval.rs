use crate::types::*;
use crate::bitboard::*;
use crate::bitset::Bitset;

use std::env;

lazy_static! {
    /// Weights for eval function can be loaded from environment.
    static ref WEIGHTS: [Score; 6] = if let Ok(var) = env::var("WEIGHTS") {
        serde_json::from_str(&var).unwrap()
    } else {
       [-10, 1, 3, 1, 3, 3]
    };
}
// pub static mut WEIGHTS: [Score; 5] = [-10, 1, 3, 1, 3];

fn area_control<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> (Bitset<{W*H}>, Bitset<{W*H}>)
where [(); (W*H+127)/128]: Sized {
    let mut debug_counter = 0;
    let mut x = (Bitset::<{W*H}>::with_bit_set(board.snakes[0].head as usize), Bitset::<{W*H}>::new());
    let mut b = !board.bodies[0];
    if board.ruleset != Ruleset::Constrictor && board.snakes[0].curled_bodyparts == 0 {
        b.set_bit(board.snakes[0].tail as usize);
    }
    for snake in &board.snakes[1..] {
        if snake.is_alive() {
            x.1.set_bit(snake.head as usize);
            if board.ruleset != Ruleset::Constrictor && snake.curled_bodyparts == 0 {
                b.set_bit(snake.tail as usize);
            }
        }
    }
    let mut y = x; // x at n-1
    loop {
        debug_counter += 1;
        debug_assert!(debug_counter < 10000, "endless loop in area_control\n{:?}\n{:?}", x, y);
        let mut me = b & (x.0 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_LEFT_EDGE_MASK & x.0)<<1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_RIGHT_EDGE_MASK & x.0)>>1 | x.0<<W | x.0>>W);
        let mut enemies = b & (x.1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_LEFT_EDGE_MASK & x.1)<<1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_RIGHT_EDGE_MASK & x.1)>>1 | x.1<<W | x.1>>W);
        if WRAP {
            me |= (Bitboard::<S, W, H, WRAP>::LEFT_EDGE_MASK & x.0) >> (W-1)
                | (Bitboard::<S, W, H, WRAP>::RIGHT_EDGE_MASK & x.0) << (W-1)
                | (Bitboard::<S, W, H, WRAP>::BOTTOM_EDGE_MASK & x.0) << ((H-1)*W)
                | (Bitboard::<S, W, H, WRAP>::TOP_EDGE_MASK & x.0) >> ((H-1)*W);
            enemies |= (Bitboard::<S, W, H, WRAP>::LEFT_EDGE_MASK & x.1) >> (W-1)
                | (Bitboard::<S, W, H, WRAP>::RIGHT_EDGE_MASK & x.1) << (W-1)
                | (Bitboard::<S, W, H, WRAP>::BOTTOM_EDGE_MASK & x.1) << ((H-1)*W) // debug changes
                | (Bitboard::<S, W, H, WRAP>::TOP_EDGE_MASK & x.1) >> ((H-1)*W);
        }
        x = (x.0 | (Bitboard::<S, W, H, WRAP>::FULL_BOARD_MASK & (me & !enemies)), x.1 | (Bitboard::<S, W, H, WRAP>::FULL_BOARD_MASK & (enemies & !me)));
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


pub fn eval<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+127)/128]: Sized {
    let mut enemies_alive = 0;
    let mut lowest_enemy_health = 100;
    let mut largest_enemy_length = 0;
    let mut tail_mask = Bitset::<{W*H}>::with_bit_set(board.snakes[0].tail as usize);

    for i in 1..S {
        if board.snakes[i].is_alive() {
            enemies_alive += 1;
            tail_mask.set_bit(board.snakes[i].tail as usize);
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
    // difference in controlled tails
    score += WEIGHTS[5] * (my_area & tail_mask).count_ones() as Score - (enemy_area & tail_mask).count_ones() as Score;

    score
}

pub fn eval_terminal<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+127)/128]: Sized {
    if board.snakes[0].is_dead() {
        for snake in board.snakes[1..].iter() {
            if snake.is_alive() {
                return Score::MIN + board.turn as i16
            }
        }
        return 0
        // return Score::MIN - board.snakes[0].health as Score
    } else {
        return Score::MAX - board.turn as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
    use crate::move_gen;
    use test::Bencher;

    fn c(x: usize, y: usize) -> api::Coord {
        api::Coord{x, y}
    }

    fn create_board() -> Bitboard<4, 11, 11, true> {
        let mut ruleset = std::collections::HashMap::new();
        ruleset.insert("name".to_string(), serde_json::Value::String("wrapped".to_string()));
        let state = api::GameState{
            game: api::Game{ id: "".to_string(), timeout: 100, ruleset },
            turn: 157,
            you: api::Battlesnake{
                id: "a".to_string(),
                name: "a".to_string(),
                shout: None,
                squad: None,
                health: 100,
                length: 11,
                head: c(5,2),
                body: vec![c(5,2), c(5,1), c(6, 1), c(7,1), c(7,2), c(8,2), c(8,3), c(7,3), c(7,4), c(6,4), c(6,4)],
            },
            board: api::Board{
                height: 11,
                width: 11,
                food: vec![c(3,10), c(6,0), c(10,1), c(0,10), c(3,0), c(9,5), c(10,3), c(9,4), c(8,4), c(8,10), c(0,6)],
                hazards: vec![],
                snakes: vec![
                    api::Battlesnake{
                        id: "a".to_string(),
                        name: "a".to_string(),
                        shout: None,
                        squad: None,
                        health: 100,
                        length: 11,
                        head: c(5,2),
                        body: vec![c(5,2), c(5,1), c(6, 1), c(7,1), c(7,2), c(8,2), c(8,3), c(7,3), c(7,4), c(6,4), c(6,4)],
                    },  
                    api::Battlesnake{
                        id: "b".to_string(),
                        name: "b".to_string(),
                        shout: None,
                        squad: None,
                        health: 95,
                        length: 12,
                        head: c(3,4),
                        body: vec![c(3,4), c(2,4), c(2,5), c(3, 5), c(3,6), c(3,7), c(3,8), c(4,8), c(4,7), c(4,6), c(4,5), c(4,4)],
                    },  
                    api::Battlesnake{
                        id: "c".to_string(),
                        name: "c".to_string(),
                        shout: None,
                        squad: None,
                        health: 95,
                        length: 3,
                        head: c(6,7),
                        body: vec![c(6,7), c(7,7), c(8,7)],
                    },  
                    api::Battlesnake{
                        id: "d".to_string(),
                        name: "d".to_string(),
                        shout: None,
                        squad: None,
                        health: 95,
                        length: 3,
                        head: c(9,9),
                        body: vec![c(9,9), c(9,8), c(8,8)],
                    },  
                ],
            },
        };
        Bitboard::<4, 11, 11, true>::from_gamestate(state)
    }
    
    #[bench]
    fn bench_eval(b: &mut Bencher) {
        let board = create_board();
        b.iter(|| {
            eval(&board)
        })
    }
}
