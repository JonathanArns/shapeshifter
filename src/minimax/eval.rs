use crate::bitboard::*;
use crate::minimax::Score;

use std::env;

// feature weight indices
const ENEMIES_ALIVE: usize = 0;
const MY_HEALTH: usize = 1;
const LOWEST_ENEMY_HEALTH: usize = 2;
const LENGTH_DIFF: usize = 3;
const NON_HAZARD_AREA_DIFF: usize = 4;
const CONTROLLED_FOOD_DIFF: usize = 5;
const AREA_DIFF: usize = 6;
const CLOSEST_FOOD_DIST: usize = 7;
const CLOSE_AREA_DIFF: usize = 8;
const BEING_LONGER: usize = 9;

const NUM_FEATURES: usize = 10;

fn get_weights(ruleset: Ruleset) -> [Score; 20] {
    match ruleset {
        Ruleset::Constrictor => [
            0, 0, -0, 0, 0, 0, 1, 0, 0, 0, // early game
            0, 0, -0, 0, 0, 0, 1, 0, 0, 0, // late game
        ],
        Ruleset::WrappedSpiral(_) | Ruleset::Royale => [
            0, 1, -1, 2, 1, 3, 0, 2, 0, 0, // early game
            0, 2, -2, 2, 1, 3, 0, 2, 1, 0, // late game
        ],
        Ruleset::Standard => [
            0, 1, -1, 0, 0, 1, 1, 0, 1, 5, // early game
            0, 1, -1, 0, 0, 1, 1, 0, 1, 5, // late game
        ],
        _ => [
            0, 1, -1, 2, 1, 3, 0, 2, 0, 0, // early game
            0, 2, -2, 2, 1, 3, 0, 2, 1, 0, // late game
        ],
    }
}

fn area_control<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>
) -> ((Bitset<{W*H}>, Bitset<{W*H}>), (Bitset<{W*H}>, Bitset<{W*H}>), Score)
where [(); (W*H+63)/64]: Sized {
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
where [(); (W*H+63)/64]: Sized {
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
where [(); (W*H+63)/64]: Sized {
    let weights = get_weights(board.ruleset);

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
    let size_diff = if weights[LENGTH_DIFF] != 0 {
        W as Score * (me.length as Score - largest_enemy_length as Score)
    } else {
        0
    };
    // difference in controlled non-hazard area
    let non_hazard_area_diff = if weights[NON_HAZARD_AREA_DIFF] != 0 {
        (my_area & !board.hazards).count_ones() as Score - (enemy_area & !board.hazards).count_ones() as Score
    } else {
        0
    };
    // difference in controlled food
    let food_control_diff = if weights[CONTROLLED_FOOD_DIFF] != 0 {
        (my_area & board.food).count_ones() as Score - (enemy_area & board.food).count_ones() as Score
    } else {
        0
    };
    // difference in controlled area
    let area_diff = if weights[AREA_DIFF] != 0 {
        my_area.count_ones() as Score - enemy_area.count_ones() as Score
    } else {
        0
    };
    // distance to closest food
    let food_dist = if weights[CLOSEST_FOOD_DIST] != 0 {
        W as Score - closest_food_distance
    } else {
        0
    };
    // reach difference
    let reach_diff = if weights[CLOSE_AREA_DIFF] != 0 {
        my_reach5.count_ones() as Score - enemy_reach5.count_ones() as Score
    } else {
        0
    };
    let being_longer = if weights[BEING_LONGER] != 0 {
        if size_diff > 0 {
            ((size_diff + 1) as f64).log2() as Score
        } else {
            -(((-size_diff + 1) as f64).log2() as Score)
        }
    } else {
        0
    };

    let mut early_score: Score = 0;
    early_score += weights[ENEMIES_ALIVE] * enemies_alive;
    early_score += weights[MY_HEALTH] * me.health as Score;
    early_score += weights[LOWEST_ENEMY_HEALTH] * lowest_enemy_health as Score;
    early_score += weights[LENGTH_DIFF] * size_diff;
    early_score += weights[NON_HAZARD_AREA_DIFF] * non_hazard_area_diff;
    early_score += weights[CONTROLLED_FOOD_DIFF] * food_control_diff;
    early_score += weights[AREA_DIFF] * area_diff;
    early_score += weights[CLOSEST_FOOD_DIST] * food_dist;
    early_score += weights[CLOSE_AREA_DIFF] * reach_diff;
    early_score += weights[BEING_LONGER] * being_longer;

    let mut late_score: Score = 0;
    late_score += weights[NUM_FEATURES+ENEMIES_ALIVE] * enemies_alive;
    late_score += weights[NUM_FEATURES+MY_HEALTH] * me.health as Score;
    late_score += weights[NUM_FEATURES+LOWEST_ENEMY_HEALTH] * lowest_enemy_health as Score;
    late_score += weights[NUM_FEATURES+LENGTH_DIFF] * size_diff;
    late_score += weights[NUM_FEATURES+NON_HAZARD_AREA_DIFF] * non_hazard_area_diff;
    late_score += weights[NUM_FEATURES+CONTROLLED_FOOD_DIFF] * food_control_diff;
    late_score += weights[NUM_FEATURES+AREA_DIFF] * area_diff;
    late_score += weights[NUM_FEATURES+CLOSEST_FOOD_DIST] * food_dist;
    late_score += weights[NUM_FEATURES+CLOSE_AREA_DIFF] * reach_diff;
    late_score += weights[NUM_FEATURES+BEING_LONGER] * being_longer;

    (early_score as f64 * (1.0 - game_progression) + late_score as f64 * game_progression).floor() as Score
}

pub fn eval_terminal<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+63)/64]: Sized {
    if board.snakes[0].is_dead() {
        for snake in board.snakes[1..].iter() {
            if snake.is_alive() {
                return Score::MIN + board.turn as Score
            }
        }
        // draw value is different depending on gamemode
        return match board.ruleset {
            Ruleset::Constrictor => 0,
            _ => -5000 + board.turn as Score,
        }
    } else {
        return Score::MAX - board.turn as Score
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
