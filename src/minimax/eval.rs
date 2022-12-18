use bitssset::Bitset;
use crate::bitboard::*;
use crate::minimax::Score;
use crate::minimax::endgame;

macro_rules! score {
    ($progress:expr , $( $w0:expr, $w1:expr, $feat:expr ),* $(,)?) => {
        {
            let mut early_score: i32 = 0;
            let mut late_score: i32 = 0;
            $(
                early_score += ($w0 * $feat) as i32;
                late_score += ($w1 * $feat) as i32;
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
pub fn eval<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    unsafe {
        if let Some(weights) = &TRAINING_WEIGHTS {
            let id = board.tt_id as usize;
            eval_with_weights(board, &weights[id])
        } else {
            panic!("no training weights set, but using training eval")
        }
    }
}

pub fn eval_with_weights<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    weights: &Vec<Score>,
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let me = board.snakes[0];
    let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board, 5 as usize);
    score!(
        turn_progression(board.turn, weights[0], weights[1]),
        weights[2],weights[3],me.health as Score,
        weights[4],weights[5],lowest_enemy_health(board),
        weights[6],weights[7],capped_length_diff(board, weights[8]),
        weights[9],weights[10],being_longer(board),
        weights[11],weights[12],controlled_food_diff(board, &my_area, &enemy_area),
        weights[13],weights[14],hazard_area_diff(board, &my_area, &enemy_area),
        weights[15],weights[16],area_diff(&my_close_area, &enemy_close_area),
        weights[17],weights[18],non_hazard_area_diff(board, &my_area, &enemy_area),
        weights[19],weights[20],(W as Score - closest_food_distance),
        weights[21],weights[22],controlled_tail_diff(board, &my_area, &enemy_area),
        weights[23],weights[24],(board.snakes[0].length%2) as Score,
    )
}

#[cfg(not(feature = "training"))]
pub fn eval<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
   board: &Bitboard<S, W, H, WRAP, HZSTACK> 
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    match board.gamemode {
        Gamemode::WrappedIslandsBridges => {
            let ((my_area, enemy_area), _, food_dist) = area_control(board, 5);
            let (my_area_size, enemy_area_size) = (checkered_area_size(board, &my_area) as Score, checkered_area_size(board, &enemy_area) as Score);
            if let Some(score) = endgame::solver(board, &my_area, &enemy_area, my_area_size, enemy_area_size, food_dist) {
                return score
            }
            score!(
                turn_or_duel_progression(board, board.turn, 67, 250),
                3,0,capped_length_diff(board, 5),
                5,1,my_area_size - enemy_area_size,
                3,0,(W as Score - food_dist),
                27,29,controlled_tail_diff(board, &my_area, &enemy_area),
                0,10,(board.snakes[0].length%2) as Score,
            )
        },
        Gamemode::WrappedArcadeMaze => {
            let me = board.snakes[0];
            let ((my_area, enemy_area), (my_close_area, enemy_close_area), _) = area_control(board, 5);
            score!(
                turn_progression(board.turn, 0, 500),
                1,1,me.health as Score,
                2,0,length_diff(board),
                2,0,controlled_food_diff(board, &my_area, &enemy_area),
                1,2,area_diff(&my_area, &enemy_area),
                0,2,area_diff(&my_close_area, &enemy_close_area),
                10,10,controlled_tail_diff(board, &my_area, &enemy_area),
            )
        },
        Gamemode::Standard => {
            let me = board.snakes[0];
            let ((my_area, enemy_area), _, food_dist) = area_control(board, 5);
            let (my_area_size, enemy_area_size) = (checkered_area_size(board, &my_area) as Score, checkered_area_size(board, &enemy_area) as Score);
            if let Some(score) = endgame::solver(board, &my_area, &enemy_area, my_area_size, enemy_area_size, food_dist) {
                return score
            }
            score!(
                turn_progression(board.turn, 0, 632),
                1,0,me.health as Score,
                -2,0,lowest_enemy_health(board),
                9,0,being_longer(board),
                0,3,controlled_food_diff(board, &my_area, &enemy_area),
                1,7,(my_area_size - enemy_area_size),
                7,0,(W as Score - food_dist),
                6,20,controlled_tail_diff(board, &my_area, &enemy_area),
            )
        },
        Gamemode::Constrictor => {
            let ((my_area, enemy_area), (_, _), _) = area_control(board, 5);
            let (my_area_size, enemy_area_size) = (checkered_area_size(board, &my_area) as Score, checkered_area_size(board, &enemy_area) as Score);
            (my_area_size - enemy_area_size) as Score
        }
        Gamemode::WrappedSpiral | Gamemode::WrappedWithHazard => {
            let me = board.snakes[0];
            let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board, 5);
            score!(
                turn_progression(board.turn, 83, 250),
                3,3,me.health as Score,
                -1,-1,lowest_enemy_health(board),
                7,0,being_longer(board),
                7,5,controlled_food_diff(board, &my_area, &enemy_area),
                4,7,non_hazard_area_diff(board, &my_area, &enemy_area),
                10,6,(W as Score - closest_food_distance),
                0,16,controlled_tail_diff(board, &my_area, &enemy_area),
            )
        }
        _ => {
            let me = board.snakes[0];
            let ((my_area, enemy_area), (my_close_area, enemy_close_area), closest_food_distance) = area_control(board, 5);
            score!(
                turn_progression(board.turn, 83, 250),
                3,3,me.health as Score,
                -1,-1,lowest_enemy_health(board),
                7,0,being_longer(board),
                7,5,controlled_food_diff(board, &my_area, &enemy_area),
                4,7,non_hazard_area_diff(board, &my_area, &enemy_area),
                10,6,(W as Score - closest_food_distance),
                0,16,controlled_tail_diff(board, &my_area, &enemy_area),
            )
        },
    }
}

pub fn eval_terminal<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
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


fn turn_progression(turns: u16, early_game_end: i16, late_game_start: i16) -> f64 {
    ((turns as i16 - early_game_end) as f64 / (late_game_start - early_game_end) as f64).min(1.0)
}

fn turn_or_duel_progression<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, turns: u16, early_game_end: i16, late_game_start: i16) -> f64
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut count = 0;
    for snake in board.snakes {
        if snake.is_alive() {
            count += 1;
        }
    }
    if count <= 2 {
        return 1.0
    }
    ((turns as i16 - early_game_end).min(0) as f64 / (late_game_start - early_game_end) as f64).min(1.0)
}

fn lowest_enemy_health<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
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

fn largest_enemy_length<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
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

fn length_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    W as Score * (board.snakes[0].length as Score - largest_enemy_length(board))
}

fn capped_length_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, cap: Score) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    W as Score * (board.snakes[0].length as Score - largest_enemy_length(board)).min(cap)
}

fn being_longer<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let length_diff = length_diff(board);
    if length_diff > 0 {
        (((length_diff + 1) as f64).log(1.5) * W as f64) as Score
    } else {
        -((((-length_diff + 1) as f64).log(1.5) * W as f64) as Score)
    }
}

fn distance_from_center<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    board.distance(board.snakes[0].head, ((W/2)+(H/2)) as u16) as Score
}

fn controlled_food_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    (*my_area & board.food).count_ones() as Score - (*enemy_area & board.food).count_ones() as Score
}

fn hazard_area_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    (*my_area & board.hazard_mask).count_ones() as Score - (*enemy_area & board.hazard_mask).count_ones() as Score
}

fn non_hazard_area_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    (*my_area & !board.hazard_mask).count_ones() as Score - (*enemy_area & !board.hazard_mask).count_ones() as Score
}

fn area_diff<const N: usize>(my_area: &Bitset<N>, enemy_area: &Bitset<N>) -> Score
where [(); (N+63)/64]: Sized {
    (*my_area).count_ones() as Score - (*enemy_area).count_ones() as Score
}

fn checkered_area_size<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>, area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let x = (*area & Bitboard::<S, W, H, WRAP, HZSTACK>::CHECKER_BOARD_MASK).count_ones();
    let y = (*area & !Bitboard::<S, W, H, WRAP, HZSTACK>::CHECKER_BOARD_MASK).count_ones();

    let over = x.max(y) - x.min(y);
    (x + y - over + over.min(1)) as i16
}

fn checkered_area_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    checkered_area_size(board, my_area) - checkered_area_size(board, enemy_area)
}

fn controlled_tail_diff<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>, my_area: &Bitset<{W*H}>, enemy_area: &Bitset<{W*H}>
) -> Score
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut res = 0;
    for snake in board.snakes {
        if snake.is_dead() {
            continue
        }
        if my_area.get(snake.tail as usize) {
            res += 1;
        } else if enemy_area.get(snake.tail as usize) {
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

// Returns ((my_fill, enemy_fill), (my_area, enemy_area), (my_close_area, enemy_close_area), my_distance_to_food)
pub fn area_control<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    close_area_distance: usize
) -> ((Bitset<{W*H}>, Bitset<{W*H}>), (Bitset<{W*H}>, Bitset<{W*H}>), Score)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut state = (Bitset::<{W*H}>::with_bit_set(board.snakes[0].head as usize), Bitset::<{W*H}>::new());
    let mut close_area = state;
    let walkable = if board.hazard_dmg > 95 {
        !board.hazard_mask & !board.bodies[0] & Bitboard::<S, W, H, WRAP, HZSTACK>::FULL_BOARD_MASK
    } else {
        !board.bodies[0] & Bitboard::<S, W, H, WRAP, HZSTACK>::FULL_BOARD_MASK
    };
    for snake in &board.snakes[1..] {
        if snake.is_alive() {
            state.1.set_bit(snake.head as usize);
        }
    }
    let mut old_state = state; // state at n-1
    let mut turn_counter = 0;
    let mut closest_food_distance = None;

    let longer = if S == 2 {
        let x = largest_enemy_length(board);
        if board.snakes[0].length > x as u8 {
            Some(true)
        } else if board.snakes[0].length < x as u8 {
            Some(false)
        } else {
            None
        }
    } else {
        None
    };

    loop {
        turn_counter += 1;
        debug_assert!(turn_counter < 10000, "endless loop in area_control\n{:?}\n{:?}", state, old_state);
        let mut me = state.0 | (Bitboard::<S, W, H, WRAP, HZSTACK>::ALL_BUT_LEFT_EDGE_MASK & state.0)<<1 | (Bitboard::<S, W, H, WRAP, HZSTACK>::ALL_BUT_RIGHT_EDGE_MASK & state.0)>>1 | state.0<<W | state.0>>W;
        let mut enemies = state.1 | (Bitboard::<S, W, H, WRAP, HZSTACK>::ALL_BUT_LEFT_EDGE_MASK & state.1)<<1 | (Bitboard::<S, W, H, WRAP, HZSTACK>::ALL_BUT_RIGHT_EDGE_MASK & state.1)>>1 | state.1<<W | state.1>>W;
        if WRAP {
            me |= (Bitboard::<S, W, H, WRAP, HZSTACK>::LEFT_EDGE_MASK & state.0) >> (W-1)
                | (Bitboard::<S, W, H, WRAP, HZSTACK>::RIGHT_EDGE_MASK & state.0) << (W-1)
                | (Bitboard::<S, W, H, WRAP, HZSTACK>::BOTTOM_EDGE_MASK & state.0) << ((H-1)*W)
                | (Bitboard::<S, W, H, WRAP, HZSTACK>::TOP_EDGE_MASK & state.0) >> ((H-1)*W);
            enemies |= (Bitboard::<S, W, H, WRAP, HZSTACK>::LEFT_EDGE_MASK & state.1) >> (W-1)
                | (Bitboard::<S, W, H, WRAP, HZSTACK>::RIGHT_EDGE_MASK & state.1) << (W-1)
                | (Bitboard::<S, W, H, WRAP, HZSTACK>::BOTTOM_EDGE_MASK & state.1) << ((H-1)*W)
                | (Bitboard::<S, W, H, WRAP, HZSTACK>::TOP_EDGE_MASK & state.1) >> ((H-1)*W);
        }
        state = match longer {
            None => (state.0 | (walkable & (me & !enemies)), state.1 | (walkable & (enemies & !me))),
            Some(true) => {
                let x = state.1 | (walkable & (enemies & !me));
                (state.0 | (walkable & (me & !x)), x)
            },
            Some(false) => {
                let x = state.0 | (walkable & (me & !enemies)); 
                (x, state.1 | (walkable & (enemies & !x)))
            },
        };
        if closest_food_distance == None && (state.0 & board.food).any() {
            closest_food_distance = Some(turn_counter);
        }
        if turn_counter == close_area_distance {
            close_area = state;
        }
        if state == old_state {
            if let Some(dist) = closest_food_distance {
                return (state, close_area, dist as Score)
            } else {
                return (state, close_area, W as Score)
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
            debug.push_str(if me.get(W*(H-1-i)+j) { "x " } else if enemies.get(W*(H-1-i)+j) { "o " } else { ". " });
        }
        debug.push_str("\n");
    }
    println!("{}", debug);
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    fn create_board() -> Bitboard<4, 11, 11, true, false> {
        let val = r###"{"game":{"id":"7ddd5c60-e27a-42ae-985e-f056e5695836","ruleset":{"name":"wrapped","version":"?","settings":{"foodSpawnChance":15,"minimumFood":1,"hazardDamagePerTurn":100,"royale":{},"squad":{"allowBodyCollisions":false,"sharedElimination":false,"sharedHealth":false,"sharedLength":false}}},"map":"hz_islands_bridges","timeout":500,"source":"league"},"turn":445,"board":{"width":11,"height":11,"food":[{"x":1,"y":9},{"x":1,"y":8},{"x":9,"y":1},{"x":6,"y":3},{"x":7,"y":3},{"x":7,"y":4},{"x":8,"y":3},{"x":4,"y":9},{"x":10,"y":8},{"x":6,"y":6}],"hazards":[{"x":5,"y":10},{"x":5,"y":9},{"x":5,"y":7},{"x":5,"y":6},{"x":5,"y":5},{"x":5,"y":4},{"x":5,"y":3},{"x":5,"y":0},{"x":5,"y":1},{"x":6,"y":5},{"x":7,"y":5},{"x":9,"y":5},{"x":10,"y":5},{"x":4,"y":5},{"x":3,"y":5},{"x":1,"y":5},{"x":0,"y":5},{"x":1,"y":10},{"x":9,"y":10},{"x":1,"y":0},{"x":9,"y":0},{"x":10,"y":1},{"x":10,"y":0},{"x":10,"y":10},{"x":10,"y":9},{"x":0,"y":10},{"x":0,"y":9},{"x":0,"y":1},{"x":0,"y":0},{"x":0,"y":6},{"x":0,"y":4},{"x":10,"y":6},{"x":10,"y":4},{"x":6,"y":10},{"x":4,"y":10},{"x":6,"y":0},{"x":4,"y":0}],"snakes":[{"id":"gs_P3P9rW63VPgMcYFFJ9R6McrM","name":"Shapeshifter","health":91,"body":[{"x":6,"y":2},{"x":6,"y":1},{"x":7,"y":1},{"x":7,"y":0},{"x":7,"y":10},{"x":8,"y":10},{"x":8,"y":0},{"x":8,"y":1},{"x":8,"y":2},{"x":9,"y":2},{"x":9,"y":3},{"x":10,"y":3},{"x":10,"y":2},{"x":0,"y":2},{"x":0,"y":3},{"x":1,"y":3},{"x":1,"y":4},{"x":2,"y":4},{"x":3,"y":4},{"x":3,"y":3},{"x":2,"y":3},{"x":2,"y":2},{"x":1,"y":2},{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":0},{"x":3,"y":0},{"x":3,"y":1},{"x":4,"y":1},{"x":4,"y":2}],"latency":11,"head":{"x":6,"y":2},"length":30,"shout":"","squad":"","customizations":{"color":"#900050","head":"cosmic-horror-special","tail":"cosmic-horror"}},{"id":"gs_YMFKJHvJwS9VV7SgtTMVmKVQ","name":"🇺🇦 Jagwire 🇺🇦","health":76,"body":[{"x":9,"y":9},{"x":8,"y":9},{"x":7,"y":9},{"x":6,"y":9},{"x":6,"y":8},{"x":5,"y":8},{"x":4,"y":8},{"x":3,"y":8},{"x":3,"y":9},{"x":3,"y":10},{"x":2,"y":10},{"x":2,"y":9},{"x":2,"y":8},{"x":2,"y":7},{"x":3,"y":7},{"x":4,"y":7},{"x":4,"y":6},{"x":3,"y":6},{"x":2,"y":6},{"x":1,"y":6},{"x":1,"y":7},{"x":0,"y":7},{"x":10,"y":7},{"x":9,"y":7},{"x":9,"y":6},{"x":8,"y":6},{"x":7,"y":6},{"x":7,"y":7},{"x":7,"y":8},{"x":8,"y":8},{"x":9,"y":8}],"latency":23,"head":{"x":9,"y":9},"length":31,"shout":"","squad":"","customizations":{"color":"#ffd900","head":"smile","tail":"wave"}}]},"you":{"id":"gs_P3P9rW63VPgMcYFFJ9R6McrM","name":"Shapeshifter","health":91,"body":[{"x":6,"y":2},{"x":6,"y":1},{"x":7,"y":1},{"x":7,"y":0},{"x":7,"y":10},{"x":8,"y":10},{"x":8,"y":0},{"x":8,"y":1},{"x":8,"y":2},{"x":9,"y":2},{"x":9,"y":3},{"x":10,"y":3},{"x":10,"y":2},{"x":0,"y":2},{"x":0,"y":3},{"x":1,"y":3},{"x":1,"y":4},{"x":2,"y":4},{"x":3,"y":4},{"x":3,"y":3},{"x":2,"y":3},{"x":2,"y":2},{"x":1,"y":2},{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":0},{"x":3,"y":0},{"x":3,"y":1},{"x":4,"y":1},{"x":4,"y":2}],"latency":11,"head":{"x":6,"y":2},"length":30,"shout":"","squad":"","customizations":{"color":"#900050","head":"cosmic-horror-special","tail":"cosmic-horror"}}}"###;
        Bitboard::<4, 11, 11, true, false>::from_str(&val).unwrap()
    }
    
    #[bench]
    fn bench_eval(b: &mut Bencher) {
        let board = create_board();
        b.iter(|| {
            eval(&board)
        })
    }
}
