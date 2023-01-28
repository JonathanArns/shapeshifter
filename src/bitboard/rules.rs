use arrayvec::ArrayVec;
use std::sync::Arc;
use super::*;
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use crate::wire_rep;

pub fn attach_rules<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(
    board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>,
    api_state: &wire_rep::GameState
)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    board.apply_moves = match api_state.game.ruleset["name"].as_str() {
        Some("constrictor") | Some("wrapped-constrictor") => Arc::new(|board, moves| {
            board.turn += 1;
            move_heads::<S, W, H, WRAP, HZSTACK, SILLY>(board, moves);
            update_health::<S, W, H, WRAP, HZSTACK, SILLY>(board);
            perform_collisions::<S, W, H, WRAP, HZSTACK, SILLY>(board);
            finish_head_movement::<S, W, H, WRAP, HZSTACK, SILLY>(board, moves);
        }),
        _ => match api_state.game.map.as_str() {
            "sinkholes" if api_state.board.hazards.len() > 0 => {
                Arc::new(move |board, moves| {
                    board.turn += 1;
                    move_heads::<S, W, H, WRAP, HZSTACK, SILLY>(board, moves);
                    move_tails::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                    update_health::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                    perform_collisions::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                    finish_head_movement::<S, W, H, WRAP, HZSTACK, SILLY>(board, moves);
                    finish_tail_movement::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                    inc_sinkholes_hazards::<S, W, H, WRAP, HZSTACK, SILLY>(board, 20);
                })
            },
            "hz_spiral" if api_state.board.hazards.len() > 0 => {
                let center = (api_state.board.width*api_state.board.hazards[0].y + api_state.board.hazards[0].x) as u16;
                Arc::new(move |board, moves| {
                    board.turn += 1;
                    move_heads::<S, W, H, WRAP, HZSTACK, SILLY>(board, moves);
                    move_tails::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                    update_health::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                    perform_collisions::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                    finish_head_movement::<S, W, H, WRAP, HZSTACK, SILLY>(board, moves);
                    finish_tail_movement::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                    inc_spiral_hazards::<S, W, H, WRAP, HZSTACK, SILLY>(board, center);
                })
            },
            _ => Arc::new(|board, moves| {
                board.turn += 1;
                move_heads::<S, W, H, WRAP, HZSTACK, SILLY>(board, moves);
                move_tails::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                update_health::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                perform_collisions::<S, W, H, WRAP, HZSTACK, SILLY>(board);
                finish_head_movement::<S, W, H, WRAP, HZSTACK, SILLY>(board, moves);
                finish_tail_movement::<S, W, H, WRAP, HZSTACK, SILLY>(board);
            }),
        },
    };
}

fn move_heads<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>, moves: &[Move; S])
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    for i in 0..S {
        if board.snakes[i].is_dead() {
            continue
        }
        let mv = moves[i];
        let mv_int = mv.to_int();
        // set direction of new body part
        board.bodies[1].set(board.snakes[i].head as usize, (mv_int&1) != 0);
        board.bodies[2].set(board.snakes[i].head as usize, (mv_int>>1) != 0);
        // set new head
        board.snakes[i].head = if let Some(new_head) = Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::MOVES_FROM_POSITION[board.snakes[i].head as usize][mv.to_int() as usize] {
            new_head as u16
        } else { // this snake has moved out of bounds
            board.kill_snake(i);
            board.snakes[i].head
        };
    }
}

fn move_tails<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    for i in 0..S {
        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        if snake.curled_bodyparts == 0 {
            let tail_move_int = board.bodies[1].get(snake.tail as usize) as u8 | (board.bodies[2].get(snake.tail as usize) as u8) << 1;
            board.bodies[0].unset_bit(snake.tail as usize);
            board.bodies[1].unset_bit(snake.tail as usize);
            board.bodies[2].unset_bit(snake.tail as usize);
            snake.tail = if WRAP {
                snake.tail as i16 + Move::int_to_index_wrapping(tail_move_int, W, H, snake.tail)
            } else {
                snake.tail as i16 + Move::int_to_index(tail_move_int, W)
            } as u16;
        } else {
            snake.curled_bodyparts -= 1;
        }
    }
}

fn update_health<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut eaten = ArrayVec::<u16, S>::new();
    for i in 0..S {
        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        // reduce health
        if HZSTACK {
            snake.health -= 1 + (board.hazard_dmg as i16 * board.hazards[snake.head as usize] as i16).min(100) as i8;
        } else {
            let is_on_hazard = board.hazard_mask.get(snake.head as usize) as i8;
            snake.health -= 1 + board.hazard_dmg * is_on_hazard;
        }

        // feed snake
        if board.food.get(snake.head as usize) {
            snake.health = 100;
            snake.curled_bodyparts += 1;
            snake.length += 1;
            eaten.push(snake.head); // remember which food has been eaten
        }

        // starvation
        if snake.is_dead() {
            board.kill_snake(i);
        }
    }
    // remove eaten food
    for food in eaten {
        board.food.unset_bit(food as usize);
    }
}

pub fn perform_collisions<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    for i in 0..S {
        if board.snakes[i].is_dead() {
            continue
        }
        // body collisions
        if board.bodies[0].get(board.snakes[i].head as usize) {
            board.snakes[i].curled_bodyparts = 100; // marked for removal
            continue
        }
        // head to head collisions
        for j in 0..S {
            if i != j
            && board.snakes[j].is_alive()
            && board.snakes[i].head == board.snakes[j].head {
                if board.snakes[i].length < board.snakes[j].length {
                    board.snakes[i].curled_bodyparts = 100; // marked for removal
                } else if board.snakes[i].length == board.snakes[j].length {
                    board.snakes[i].curled_bodyparts = 100; // marked for removal
                }
            }
        }
    }

    // remove collided snakes
    for i in 0..S {
        // remove collided snakes
        if board.snakes[i].curled_bodyparts == 100 {
            board.snakes[i].curled_bodyparts = 0;
            board.kill_snake(i);
        }
    }
}

pub fn finish_head_movement<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>, moves: &[Move; S])
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    for i in 0..S {
        if board.snakes[i].is_alive() {
            if board.snakes[i].is_dead() {
                continue
            }
            let mv = moves[i];
            let mv_int = mv.to_int();
            // set new head position as move direction (used for silly move gen)
            board.bodies[1].set(board.snakes[i].head as usize, (mv_int&1) != 0);
            board.bodies[2].set(board.snakes[i].head as usize, (mv_int>>1) != 0);

            // set snake heads in bodies
            // we do this last, since it would break collision checks earlier, but we want this info
            // for move gen on the new board, since moving into the current space of a head is illegal
            board.bodies[0].set_bit(board.snakes[i].head as usize);
        }
    }
}

pub fn finish_tail_movement<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
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

pub fn spawn_food<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut rng = Pcg64Mcg::new(91825765198273048172569872943871926276_u128);
    if rng.gen_ratio(15, 100) {
        let pos = rng.gen_range(0..(W*H));
        if !board.bodies[0].get(pos) {
            board.food.set_bit(pos);
        }
    }
}


// Map specific rules //


const HAZARD_SPIRAL_SHIFTS: [(i8, i8); 144] = constants::precompute_hazard_spiral();

pub fn inc_spiral_hazards<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>, center: u16)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    if board.turn % 3 != 0 || board.turn / 3 > 142 || board.turn == 0 {
        return
    }
    let (x_shift, y_shift) = HAZARD_SPIRAL_SHIFTS[((board.turn/3)-1) as usize];
    let x = center as i16 % W as i16 + x_shift as i16;
    let y = center as i16 / W as i16 + y_shift as i16;
    if x >= 0 && x < W as i16 && y >= 0 && y < H as i16 {
        board.hazard_mask.set_bit((center as i16 + x_shift as i16 + y_shift as i16 * W as i16) as usize);
    }
}

pub fn inc_sinkholes_hazards<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>(
    board: &mut Bitboard<S, W, H, WRAP, HZSTACK, SILLY>,
    expand_every_n_turns: u16,
)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    if !HZSTACK {
        return // only works with stacking hazards
    }

    let start_turn = 1;
	let max_rings = if W == 7 {
		3
	} else if W == 19 {
		7
	} else {
        5
    };
    if (board.turn - start_turn) % expand_every_n_turns != 0 {
		return // this is not a turn to expand
	}
    if board.turn > expand_every_n_turns * max_rings {
		return // the sinkhole is at max size
	}

    let offset = ((board.turn - start_turn) as f64 / expand_every_n_turns as f64).floor() as u16;
    let spawn_x = (W as f32 / 2.0).ceil() as u16;
    let spawn_y = (H as f32 / 2.0).ceil() as u16;

    if board.turn == start_turn {
        let pos = W * spawn_y as usize + spawn_x as usize;
        board.hazard_mask.set_bit(pos);
        board.hazards[pos] += 1;
    }

    if offset > 0 && offset <= max_rings {
        for x in (spawn_x-offset)..(spawn_x+offset) {
            for y in (spawn_y-offset)..(spawn_y+offset) {
                // don't draw in the corners of the square so we get a rounded effect
                if !(x == spawn_x-offset && y == spawn_y-offset)
					&& !(x == spawn_x+offset && y == spawn_y-offset)
					&& !(x == spawn_x-offset && y == spawn_y+offset)
					&& !(x == spawn_x+offset && y == spawn_y+offset) {
                    let pos = W * y as usize + x as usize;
                    board.hazard_mask.set_bit(pos);
                    board.hazards[pos] += 1;
				}
            }
        }
    }
}
