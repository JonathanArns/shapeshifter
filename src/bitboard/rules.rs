use arrayvec::ArrayVec;
use std::sync::Arc;
use super::*;
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use crate::wire_rep;

pub fn attach_rules<const S: usize, MODE: Mode>(
    board: &mut Bitboard<S, MODE>,
    api_state: &wire_rep::GameState
) {
    board.apply_moves = match api_state.game.ruleset["name"].as_str() {
        Some("constrictor") | Some("wrapped-constrictor") => Arc::new(|board, moves| {
            board.turn += 1;
            move_heads::<S, MODE>(board, moves);
            let collided = perform_collisions::<S, MODE>(board);
            collided.into_iter().for_each(|snake_idx| board.kill_snake(snake_idx));
            finish_head_movement::<S, MODE>(board);
        }),
        _ => match api_state.game.map.as_str() {
            "hz_spiral" if api_state.board.hazards.len() > 0 => {
                let center = (api_state.board.width*api_state.board.hazards[0].y + api_state.board.hazards[0].x) as u16;
                Arc::new(move |board, moves| {
                    board.turn += 1;
                    move_heads::<S, MODE>(board, moves);
                    move_tails::<S, MODE>(board);
                    let starved = update_health::<S, MODE>(board);
                    starved.into_iter().for_each(|snake_idx| board.kill_snake(snake_idx));
                    let collided = perform_collisions::<S, MODE>(board);
                    collided.into_iter().for_each(|snake_idx| board.kill_snake(snake_idx));
                    finish_head_movement::<S, MODE>(board);
                    finish_tail_movement::<S, MODE>(board);
                    inc_spiral_hazards::<S, MODE>(board, center);
                })
            },
            _ => Arc::new(|board, moves| {
                board.turn += 1;
                move_heads::<S, MODE>(board, moves);
                move_tails::<S, MODE>(board);
                let starved = update_health::<S, MODE>(board);
                starved.into_iter().for_each(|snake_idx| board.kill_snake(snake_idx));
                let collided = perform_collisions::<S, MODE>(board);
                collided.into_iter().for_each(|snake_idx| board.kill_snake(snake_idx));
                finish_head_movement::<S, MODE>(board);
                finish_tail_movement::<S, MODE>(board);
            }),
        },
    };
}

fn move_heads<const S: usize, MODE: Mode>(board: &mut Bitboard<S, MODE>, moves: &[Move; S]) {
    for i in 0..S {
        if board.snakes[i].is_dead() {
            continue
        }
        let mv = moves[i];
        let mv_int = mv.to_int();
        // set direction of new body part
        let pos = board.snakes[i].head as usize;
        board.bodies[1].set(pos, (mv_int&1) != 0);
        board.bodies[2].set(pos, (mv_int>>1) != 0);
        // set new head
        board.snakes[i].head = if let Some(new_head) = MODE::moves_from_position(pos as u16)[mv_int as usize] {
            new_head
        } else { // this snake has moved out of bounds
            board.kill_snake(i);
            pos as u16
        };
    }
}

fn move_tails<const S: usize, MODE: Mode>(board: &mut Bitboard<S, MODE>) {
    for i in 0..S {
        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        if snake.curled_bodyparts == 0 || snake.curled_bodyparts == 1 && snake.health == 100 {
            let tail_move_int = board.bodies[1].get(snake.tail as usize) as u8 | (board.bodies[2].get(snake.tail as usize) as u8) << 1;
            board.bodies[0].unset_bit(snake.tail as usize);
            board.bodies[1].unset_bit(snake.tail as usize);
            board.bodies[2].unset_bit(snake.tail as usize);
            snake.tail = if MODE::WRAP {
                snake.tail as i16 + Move::int_to_index_wrapping(tail_move_int, MODE::W, MODE::H, snake.tail)
            } else {
                snake.tail as i16 + Move::int_to_index(tail_move_int, MODE::W)
            } as u16;
        } else {
            snake.curled_bodyparts -= 1;
        }
    }
}

fn update_health<const S: usize, MODE: Mode>(board: &mut Bitboard<S, MODE>) -> ArrayVec<usize, S> {
    let mut eaten = ArrayVec::<u16, S>::new();
    let mut starved = ArrayVec::<usize, S>::new();
    for i in 0..S {
        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        // reduce health
        let is_on_hazard = board.hazard_mask.get(snake.head as usize) as i8;
        snake.health -= 1 + board.hazard_dmg * is_on_hazard;

        // feed snake
        if board.food.get(snake.head as usize) {
            snake.health = 100;
            snake.curled_bodyparts += 1;
            snake.length += 1;
            eaten.push(snake.head); // remember which food has been eaten
        }

        // starvation
        if snake.is_dead() {
            // mark snake for removal
            snake.health = 1;
            starved.push(i);
        }
    }
    // remove eaten food
    for food in eaten {
        board.food.unset_bit(food as usize);
    }
    starved
}

fn perform_collisions<const S: usize, MODE: Mode>(board: &mut Bitboard<S, MODE>) -> ArrayVec<usize, S> {
    let mut collided = ArrayVec::<usize, S>::new();
    'OUTER_LOOP: for i in 0..S {
        if board.snakes[i].is_dead() {
            continue
        }
        // body collisions
        if board.bodies[0].get(board.snakes[i].head as usize) {
            collided.push(i);
            continue
        }
        // head to head collisions
        for j in 0..S {
            if i != j
            && board.snakes[j].is_alive()
            && board.snakes[i].head == board.snakes[j].head {
                if board.snakes[i].length < board.snakes[j].length {
                    collided.push(i);
                    continue 'OUTER_LOOP
                } else if board.snakes[i].length == board.snakes[j].length {
                    collided.push(i);
                    continue 'OUTER_LOOP
                }
            }
        }
    }
    collided
}

fn finish_head_movement<const S: usize, MODE: Mode>(board: &mut Bitboard<S, MODE>) {
    for i in 0..S {
        if board.snakes[i].is_alive() {
            if board.snakes[i].is_dead() {
                continue
            }
            // set snake heads in bodies
            // we do this last, since it would break collision checks earlier, but we want this info
            // for move gen on the new board, since moving into the current space of a head is illegal
            board.bodies[0].set_bit(board.snakes[i].head as usize);
        }
    }
}

fn finish_tail_movement<const S: usize, MODE: Mode>(board: &mut Bitboard<S, MODE>) {
    for i in 0..S {
        if board.snakes[i].is_alive() && board.snakes[i].curled_bodyparts == 0 {
            // unset tail bits for snakes that have no curled bodyparts 
            // we do this, since it is allowed to move there and we can effectively treat these
            // spaces as empty for the next move
            // we also do this last, since we need it earlier for collision checks of this turn
            board.bodies[0].unset_bit(board.snakes[i].tail as usize);
        }
    }
}

fn spawn_food<const S: usize, MODE: Mode>(board: &mut Bitboard<S, MODE>) {
    let mut rng = Pcg64Mcg::new(91825765198273048172569872943871926276_u128);
    if rng.gen_ratio(15, 100) {
        let pos = rng.gen_range(0..(MODE::W*MODE::H));
        if !board.bodies[0].get(pos) {
            board.food.set_bit(pos);
        }
    }
}


// Map specific rules //


const HAZARD_SPIRAL_SHIFTS: [(i8, i8); 144] = constants::precompute_hazard_spiral();

fn inc_spiral_hazards<const S: usize, MODE: Mode>(board: &mut Bitboard<S, MODE>, center: u16) {
    if board.turn % 3 != 0 || board.turn / 3 > 142 || board.turn == 0 {
        return
    }
    let (x_shift, y_shift) = HAZARD_SPIRAL_SHIFTS[((board.turn/3)-1) as usize];
    let x = center as i16 % MODE::W as i16 + x_shift as i16;
    let y = center as i16 / MODE::W as i16 + y_shift as i16;
    if x >= 0 && x < MODE::W as i16 && y >= 0 && y < MODE::H as i16 {
        board.hazard_mask.set_bit((center as i16 + x_shift as i16 + y_shift as i16 * MODE::W as i16) as usize);
    }
}
