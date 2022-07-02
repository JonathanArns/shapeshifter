use crate::bitboard::*;
use crate::minimax::Score;

macro_rules! score {
    ($progress:expr , $( $w0:expr, $w1:expr, $feat:expr ),* $(,)?) => {
        {
            let mut early_score: Score = 0;
            let mut late_score: Score = 0;
            $(
                early_score += $w0 * $feat;
                late_score += $w1 * $feat;
            )*
            (early_score as f64 * (1.0 - $progress) + late_score as f64 * $progress).floor() as Score
        }
    };
}

#[cfg(feature = "training")]
static mut TRAINING_WEIGHTS: Option<Vec<Vec<Score>>> = None;

#[cfg(feature = "training")]
pub unsafe fn set_training_weights(weights: Vec<Vec<Score>>) {
    TRAINING_WEIGHTS = Some(weights);
}

#[cfg(feature = "training")]
pub fn eval<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>
) -> Score
where [(); (W*H+63)/64]: Sized {
    unsafe {
        if let Some(weights) = &TRAINING_WEIGHTS {
            let id = board.tt_id as usize;
            let me = board.snakes[0];
            let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board);
            score!(
                turn_progression(board.turn, weights[id][0]),
                weights[id][1],weights[id][2],me.health as Score,
                weights[id][3],weights[id][4],lowest_enemy_health(board),
                weights[id][5],weights[id][6],length_diff(board),
                weights[id][7],weights[id][8],being_longer(board),
                weights[id][9],weights[id][10],controlled_food_diff(board, &my_area, &enemy_area),
                weights[id][11],weights[id][12],area_diff(&my_area, &enemy_area),
                weights[id][13],weights[id][14],area_diff(&my_close_area, &enemy_close_area),
                weights[id][15],weights[id][16],non_hazard_area_diff(board, &my_area, &enemy_area),
                weights[id][17],weights[id][18],(W as Score - closest_food_distance),
                weights[id][19],weights[id][20],controlled_tail_diff(board, &my_area, &enemy_area),
            )
        } else {
            panic!("no training weights set, but using training eval")
        }
    }
}

#[cfg(not(feature = "training"))]
pub fn eval<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
   board: &Bitboard<S, W, H, WRAP> 
) -> Score
where [(); (W*H+63)/64]: Sized {
    match board.gamemode {
        // gen 66
        // Gamemode::WrappedArcadeMaze => {
        //     let me = board.snakes[0];
        //     let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board);
        //     score!(
        //         turn_progression(board.turn, 1514),
        //         1,0,me.health as Score,
        //         -2,0,length_diff(board),
        //         0,4,being_longer(board),
        //         2,6,area_diff(&my_area, &enemy_area),
        //         6,0,area_diff(&my_close_area, &enemy_close_area),
        //         3,4,(W as Score - closest_food_distance),
        //         20,7,controlled_tail_diff(board, &my_area, &enemy_area),
        //     )
        // },

        // gen 50
        // Gamemode::WrappedArcadeMaze => {
        //     let me = board.snakes[0];
        //     let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board);
        //     score!(
        //         turn_progression(board.turn, 1056),
        //         0,-3,lowest_enemy_health(board),
        //         -2,0,length_diff(board),
        //         7,0,controlled_food_diff(board, &my_area, &enemy_area),
        //         7,2,area_diff(&my_area, &enemy_area),
        //         6,0,area_diff(&my_close_area, &enemy_close_area),
        //         0,10,controlled_tail_diff(board, &my_area, &enemy_area),
        //     )
        // },

        Gamemode::WrappedArcadeMaze => {
            let me = board.snakes[0];
            let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board);
            score!(
                turn_progression(board.turn, 500),
                1,1,me.health as Score,
                // -1 -1 lowest_enemy_health(board),
                2,0,length_diff(board),
                2,0,controlled_food_diff(board, &my_area, &enemy_area),
                1,2,area_diff(&my_area, &enemy_area),
                0,2,area_diff(&my_close_area, &enemy_close_area),
                // 3 3 controlled_arcade_maze_junctions(board, &my_area, &enemy_area),
                // 5 5 controlled_tail_diff(board, &my_area, &enemy_area),
            )
        },
        Gamemode::Standard => {
            let me = board.snakes[0];
            let ((my_area, enemy_area), (my_close_area, enemy_close_area), _) = area_control(board);
            score!(
                turn_progression(board.turn, 1),
                1,1,me.health as Score,
                -1,-1,lowest_enemy_health(board),
                1,1,controlled_food_diff(board, &my_area, &enemy_area),
                1,1,area_diff(&my_area, &enemy_area),
                1,1,area_diff(&my_close_area, &enemy_close_area),
                5,5,being_longer(board),
            )
        },
        Gamemode::Constrictor => {
            let ((my_area, enemy_area), (_, _), _) = area_control(board);
            score!(
                turn_progression(board.turn, 1),
                1,1,area_diff(&my_area, &enemy_area),
            )
        }
        Gamemode::WrappedSpiral | Gamemode::WrappedWithHazard => {
            let me = board.snakes[0];
            let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board);
            score!(
                fill_progression(board),
                1,2,me.health as Score,
                -1,-2,lowest_enemy_health(board),
                2,2,length_diff(board),
                1,1,non_hazard_area_diff(board, &my_area, &enemy_area),
                3,3,controlled_food_diff(board, &my_area, &enemy_area),
                2,2,(W as Score - closest_food_distance),
                0,1,area_diff(&my_close_area, &enemy_close_area),
            )
        }
        _ => {
            let me = board.snakes[0];
            let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board);
            score!(
                fill_progression(board),
                1,2,me.health as Score,
                -1,-2,lowest_enemy_health(board),
                2,2,length_diff(board),
                1,1,non_hazard_area_diff(board, &my_area, &enemy_area),
                3,3,controlled_food_diff(board, &my_area, &enemy_area),
                2,2,(W as Score - closest_food_distance),
                0,1,area_diff(&my_close_area, &enemy_close_area),
            )
        },
    }
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
        return match board.gamemode {
            Gamemode::Constrictor => 0,
            _ => -5000 + board.turn as Score,
        }
    } else {
        return Score::MAX - board.turn as Score
    }
}


fn turn_progression(turns: u16, late_game_start: i16) -> f64 {
    (turns as f64 / late_game_start as f64).min(1.0)
}

fn fill_progression<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> f64
where [(); (W*H+63)/64]: Sized {
    ((board.hazards & board.bodies[0]).count_zeros() as f64 / ((W-1) * (H-1)) as f64).min(1.0)
}

fn lowest_enemy_health<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+63)/64]: Sized {
    let mut lowest_enemy_health = 100;
    for i in 1..S {
        if board.snakes[i].is_alive() {
            if board.snakes[i].health < lowest_enemy_health {
                lowest_enemy_health = board.snakes[i].health;
            }
        }
    }
    lowest_enemy_health as Score
}

fn largest_enemy_length<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+63)/64]: Sized {
    let mut largest_enemy_length = 0;
    for i in 1..S {
        if board.snakes[i].is_alive() {
            if board.snakes[i].length > largest_enemy_length {
                largest_enemy_length = board.snakes[i].length;
            }
        }
    }
    largest_enemy_length as Score
}

fn length_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+63)/64]: Sized {
    W as Score * (board.snakes[0].length as Score - largest_enemy_length(board))
}

fn being_longer<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+63)/64]: Sized {
    let length_diff = length_diff(board);
    if length_diff > 0 {
        ((length_diff + 1) as f64).log2() as Score
    } else {
        -(((-length_diff + 1) as f64).log2() as Score)
    }
}

fn controlled_food_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized {
    (*my_area & board.food).count_ones() as Score - (*enemy_area & board.food).count_ones() as Score
}

fn non_hazard_area_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized {
    (*my_area & !board.hazards).count_ones() as Score - (*enemy_area & !board.hazards).count_ones() as Score
}

fn area_diff<const N: usize>(my_area: &Bitset<N>, enemy_area: &Bitset<N>) -> Score
where [(); (N+63)/64]: Sized {
    (*my_area).count_ones() as Score - (*enemy_area).count_ones() as Score
}

fn controlled_tail_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized {
    let mut res = 0;
    for snake in board.snakes {
        if snake.is_dead() {
            continue
        }
        if my_area.get_bit(snake.tail as usize) {
            res += 1;
        } else if enemy_area.get_bit(snake.tail as usize) {
            res -= 1;
        }
    }
    res
}

fn controlled_arcade_maze_junctions<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized {
    let mut res = 0;
    for pos in [20, 27, 29, 36, 59, 73, 99, 101, 103, 105, 107, 109, /* 134, 135, */ 137, 139, 145, 147, /* 149, 150, */ 177, 179, 181, 183, 213, 215, 217, 219, 221, 223, 255, 257, 289, 299, 324, 327, 329, 331, 333, 335, 337, 339, 361, 364, 374, 377] {
        if my_area.get_bit(pos) {
            res += 1;
        } else if enemy_area.get_bit(pos) {
            res -= 1;
        }
    }
    res
}

fn get_food_spawns(gamemode: Gamemode) -> &'static [usize] {
    match gamemode {
        Gamemode::WrappedArcadeMaze => &[20, 36, 104, 137, 147, 212, 218, 224, 327, 332, 337],
        _ => &[],
    }
}

fn area_control<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &Bitboard<S, W, H, WRAP>
) -> ((Bitset<{W*H}>, Bitset<{W*H}>), (Bitset<{W*H}>, Bitset<{W*H}>), Score)
where [(); (W*H+63)/64]: Sized {
    let mut state = (Bitset::<{W*H}>::with_bit_set(board.snakes[0].head as usize), Bitset::<{W*H}>::new());
    let mut reachable5 = state;
    let mut walkable = if board.hazard_dmg > 95 {
        !board.hazards & !board.bodies[0] & Bitboard::<S, W, H, WRAP>::FULL_BOARD_MASK
    } else {
        !board.bodies[0] & Bitboard::<S, W, H, WRAP>::FULL_BOARD_MASK
    };
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
        let mut me = state.0 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_LEFT_EDGE_MASK & state.0)<<1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_RIGHT_EDGE_MASK & state.0)>>1 | state.0<<W | state.0>>W;
        let mut enemies = state.1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_LEFT_EDGE_MASK & state.1)<<1 | (Bitboard::<S, W, H, WRAP>::ALL_BUT_RIGHT_EDGE_MASK & state.1)>>1 | state.1<<W | state.1>>W;
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
        state = (state.0 | (walkable & (me & !enemies)), state.1 | (walkable & (enemies & !me)));
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
            debug.push_str(if me.get_bit(W*(H-1-i)+j) { "x " } else if enemies.get_bit(W*(H-1-i)+j) { "o " } else { ". " });
        }
        debug.push_str("\n");
    }
    println!("{}", debug);
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
            game: api::Game{ id: "".to_string(), timeout: 100, ruleset, map: "standard".to_string() },
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
