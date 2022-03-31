use crate::bitboard::*;
use crate::minimax::Score;

use std::env;

lazy_static! {
    /// Weights for eval function can be loaded from environment.
    static ref WEIGHTS: [Score; 18] = if let Ok(var) = env::var("WEIGHTS") {
        serde_json::from_str(&var).unwrap()
    } else {
        // 0 number of enemies alive
        // 1 my health
        // 2 lowest enemy health
        // 3 difference in length to longest enemy
        // 4 difference in controlled non-hazard area
        // 5 difference in controlled food
        // 6 difference in controlled area
        // 7 distance to closest food
        // 8 difference in close reach
        [
            0, 1, -1, 2, 1, 3, 0, 2, 0, // early game
            0, 2, -2, 2, 1, 3, 0, 2, 1, // late game
        ]
    };
}
// pub static mut WEIGHTS: [Score; 5] = [-10, 1, 3, 1, 3];

fn area_control<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>
) -> ((Bitset<{W*H}>, Bitset<{W*H}>), (Bitset<{W*H}>, Bitset<{W*H}>), Score)
where [(); (W*H+127)/128]: Sized {
    let mut state = (Bitset::<{W*H}>::with_bit_set(board.snakes[0].head as usize), Bitset::<{W*H}>::new());
    let mut reachable5 = state;
    let b = !board.bodies[0];
    for snake in &board.snakes[1..] {
        if snake.is_alive() {
            state.1.set_bit(snake.head as usize);
        }
    }
    let mut old_state = state; // state at n-1
    let mut turn_counter = 0;
    let mut closest_food_distance = None;
    loop {
        turn_counter += 1;
        debug_assert!(turn_counter < 10000, "endless loop in area_control\n{:?}\n{:?}", state, old_state);
        let mut me = b & (state.0 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_LEFT_EDGE_MASK & state.0)<<1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_RIGHT_EDGE_MASK & state.0)>>1 | state.0<<W | state.0>>W);
        let mut enemies = b & (state.1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_LEFT_EDGE_MASK & state.1)<<1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_RIGHT_EDGE_MASK & state.1)>>1 | state.1<<W | state.1>>W);
        if WRAP {
            me |= (Bitboard::<S, W, H, WRAP>::LEFT_EDGE_MASK & state.0) >> (W-1)
                | (Bitboard::<S, W, H, WRAP>::RIGHT_EDGE_MASK & state.0) << (W-1)
                | (Bitboard::<S, W, H, WRAP>::BOTTOM_EDGE_MASK & state.0) << ((H-1)*W)
                | (Bitboard::<S, W, H, WRAP>::TOP_EDGE_MASK & state.0) >> ((H-1)*W);
            enemies |= (Bitboard::<S, W, H, WRAP>::LEFT_EDGE_MASK & state.1) >> (W-1)
                | (Bitboard::<S, W, H, WRAP>::RIGHT_EDGE_MASK & state.1) << (W-1)
                | (Bitboard::<S, W, H, WRAP>::BOTTOM_EDGE_MASK & state.1) << ((H-1)*W) // debug changes
                | (Bitboard::<S, W, H, WRAP>::TOP_EDGE_MASK & state.1) >> ((H-1)*W);
        }
        state = (state.0 | (Bitboard::<S, W, H, WRAP>::FULL_BOARD_MASK & (me & !enemies)), state.1 | (Bitboard::<S, W, H, WRAP>::FULL_BOARD_MASK & (enemies & !me)));
        if closest_food_distance == None && (state.0 & board.food).any() {
            closest_food_distance = Some(turn_counter);
        }
        if turn_counter == 5 {
            reachable5 = state;
        }
        if state == old_state {
            if let Some(dist) = closest_food_distance {
                return (state, reachable5, dist as Score)
            } else {
                return (state, reachable5, W as Score)
            }
        } else {
            old_state = state;
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
    let me = board.snakes[0];
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
    let ((my_area, enemy_area), (my_reach5, enemy_reach5), closest_food_distance) = area_control(board);

    let game_progression = ((board.hazards & board.bodies[0]).count_zeros() as f64 / ((W-1) * (H-1)) as f64).min(1.0);

    // difference in length to longest enemy
    let size_diff = W as Score * (me.length as Score - largest_enemy_length as Score);
    // difference in controlled non-hazard area
    let non_hazard_area_diff = (my_area & !board.hazards).count_ones() as Score - (enemy_area & !board.hazards).count_ones() as Score;
    // difference in controlled food
    let food_control_diff = (my_area & board.food).count_ones() as Score - (enemy_area & board.food).count_ones() as Score;
    // difference in controlled area
    // let area_diff = my_area.count_ones() as Score - enemy_area.count_ones() as Score;
    // distance to closest food
    let food_dist = W as Score - closest_food_distance;
    // reach difference
    let reach_diff = my_reach5.count_ones() as Score - enemy_reach5.count_ones() as Score;
    // let reach_diff = my_reach3.count_ones() as Score - enemy_reach3.count_ones() as Score;

    let mut early_score: Score = 0;
    early_score += WEIGHTS[0] * enemies_alive;
    early_score += WEIGHTS[1] * me.health as Score;
    early_score += WEIGHTS[2] * lowest_enemy_health as Score;
    early_score += WEIGHTS[3] * size_diff;
    early_score += WEIGHTS[4] * non_hazard_area_diff;
    early_score += WEIGHTS[5] * food_control_diff;
    // early_score += WEIGHTS[6] * area_diff;
    early_score += WEIGHTS[7] * food_dist;
    early_score += WEIGHTS[8] * reach_diff;

    let mut late_score: Score = 0;
    late_score += WEIGHTS[9] * enemies_alive;
    late_score += WEIGHTS[10] * me.health as Score;
    late_score += WEIGHTS[11] * lowest_enemy_health as Score;
    late_score += WEIGHTS[12] * size_diff;
    late_score += WEIGHTS[13] * non_hazard_area_diff;
    late_score += WEIGHTS[14] * food_control_diff;
    // late_score += WEIGHTS[15] * area_diff;
    late_score += WEIGHTS[16] * food_dist;
    late_score += WEIGHTS[17] * reach_diff;

    (early_score as f64 * (1.0 - game_progression) + late_score as f64 * game_progression).floor() as Score
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
    } else {
        return Score::MAX - board.turn as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
    use crate::bitboard::move_gen;
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
