use arrayvec::ArrayVec;
use std::rc::Rc;
use super::*;

const BODY_COLLISION: i8 = -1;
const OUT_OF_HEALTH: i8 = -2;
const HEAD_COLLISION: i8 = -3;
const EVEN_HEAD_COLLISION: i8 = -4;
const HAZARD_SPIRAL_SHIFTS: [(i8, i8); 144] = constants::precompute_hazard_spiral();

pub fn attach_rules<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    board: &mut Bitboard<S, W, H, WRAP>,
    api_state: &GameState
)
where [(); (W*H+63)/64]: Sized {
    board.apply_moves = match api_state.game.ruleset["name"].as_str() {
        Some("constrictor") => Rc::new(|board, moves| {
            board.turn += 1;
            board.depth += 1;
            move_heads::<S, W, H, WRAP>(board, moves);
            perform_collisions::<S, W, H, WRAP>(board);
            finish_head_movement::<S, W, H, WRAP>(board);
        }),
        _ => match api_state.game.map.as_str() {
            // "arcade_maze" => {
            //     let center = (api_state.board.width*api_state.board.hazards[0].y + api_state.board.hazards[0].x) as u16;
            //     Rc::new(move |board, moves| {
            //         board.turn += 1;
            //         board.depth += 1;
            //         move_heads::<S, W, H, WRAP>(board, moves);
            //         move_tails::<S, W, H, WRAP>(board);
            //         update_health_with_fixed_spawns::<S, W, H, WRAP>(board);
            //         perform_collisions::<S, W, H, WRAP>(board);
            //         finish_head_movement::<S, W, H, WRAP>(board);
            //         finish_tail_movement::<S, W, H, WRAP>(board);
            //     })
            // },
            "hz_spiral" if api_state.board.hazards.len() > 0 => {
                let center = (api_state.board.width*api_state.board.hazards[0].y + api_state.board.hazards[0].x) as u16;
                Rc::new(move |board, moves| {
                    board.turn += 1;
                    board.depth += 1;
                    move_heads::<S, W, H, WRAP>(board, moves);
                    move_tails::<S, W, H, WRAP>(board);
                    update_health::<S, W, H, WRAP>(board);
                    perform_collisions::<S, W, H, WRAP>(board);
                    finish_head_movement::<S, W, H, WRAP>(board);
                    finish_tail_movement::<S, W, H, WRAP>(board);
                    inc_spiral_hazards::<S, W, H, WRAP>(board, center);
                })
            },
            _ => Rc::new(|board, moves| {
                board.turn += 1;
                board.depth += 1;
                move_heads::<S, W, H, WRAP>(board, moves);
                move_tails::<S, W, H, WRAP>(board);
                update_health::<S, W, H, WRAP>(board);
                perform_collisions::<S, W, H, WRAP>(board);
                finish_head_movement::<S, W, H, WRAP>(board);
                finish_tail_movement::<S, W, H, WRAP>(board);
            }),
        },
    };
}

fn move_heads<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>, moves: &[Move; S])
where [(); (W*H+63)/64]: Sized {
    for i in 0..S {
        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        let mv = moves[i];
        let mv_int = mv.to_int();
        // set direction of new body part
        board.bodies[1].set(snake.head as usize, (mv_int&1) != 0);
        board.bodies[2].set(snake.head as usize, (mv_int>>1) != 0);
        // set new head
        snake.head = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[snake.head as usize][mv.to_int() as usize].expect("move out of bounds") as u16;
    }
}

fn move_tails<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>)
where [(); (W*H+63)/64]: Sized {
    for i in 0..S {
        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        if snake.curled_bodyparts == 0 {
            let tail_move_int = board.bodies[1].get_bit(snake.tail as usize) as u8 | (board.bodies[2].get_bit(snake.tail as usize) as u8) << 1;
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

fn update_health<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>)
where [(); (W*H+63)/64]: Sized {
    let mut eaten = ArrayVec::<u16, S>::new();
    for i in 0..S {
        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        // reduce health
        let is_on_hazard = board.hazards.get_bit(snake.head as usize) as i8;
        snake.health -= 1 + board.hazard_dmg * is_on_hazard;

        // feed snake
        if board.food.get_bit(snake.head as usize) {
            snake.health = 100;
            snake.curled_bodyparts += 1;
            snake.length += 1;
            eaten.push(snake.head); // remember which food has been eaten
        }

        // starvation
        if snake.is_dead() {
            snake.health = OUT_OF_HEALTH;
            board.remove_snake_body(i);
        }
    }
    // remove eaten food
    for food in eaten {
        board.food.unset_bit(food as usize);
    }
}

pub fn perform_collisions<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>)
where [(); (W*H+63)/64]: Sized {
    for i in 0..S {
        if board.snakes[i].is_dead() {
            continue
        }
        // body collisions
        if board.bodies[0].get_bit(board.snakes[i].head as usize) {
            board.snakes[i].curled_bodyparts = 100; // marked for removal
            continue
        }
        // head to head collisions
        for j in 0..S {
            if i != j
            && board.snakes[j].is_alive()
            && board.snakes[i].head == board.snakes[j].head {
                if board.snakes[i].length < board.snakes[j].length {
                    board.snakes[i].curled_bodyparts = 101; // marked for removal
                } else if board.snakes[i].length == board.snakes[j].length {
                    board.snakes[i].curled_bodyparts = 102; // marked for removal
                }
            }
        }
    }

    // remove collided snakes
    for i in 0..S {
        // remove collided snakes
        if board.snakes[i].curled_bodyparts >= 100 {
            if board.snakes[i].curled_bodyparts == 100 {
                board.snakes[i].health = BODY_COLLISION;
            } else if board.snakes[i].curled_bodyparts == 101 {
                board.snakes[i].health = HEAD_COLLISION;
            } else if board.snakes[i].curled_bodyparts == 102 {
                board.snakes[i].health = EVEN_HEAD_COLLISION;
            }
            board.snakes[i].curled_bodyparts = 0;
            board.remove_snake_body(i);
        }
    }
}

pub fn finish_head_movement<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>)
where [(); (W*H+63)/64]: Sized {
    for i in 0..S {
        if board.snakes[i].is_alive() {
            // set snake heads in bodies
            // we do this last, since it would break collision checks earlier, but we want this info
            // for move gen on the new board, since moving into the current space of a head is illegal
            board.bodies[0].set_bit(board.snakes[i].head as usize);
        }
    }
}

pub fn finish_tail_movement<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>)
where [(); (W*H+63)/64]: Sized {
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


// Map specific rules //


pub fn inc_spiral_hazards<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>, center: u16)
where [(); (W*H+63)/64]: Sized {
    if board.turn % 3 != 0 || board.turn / 3 > 142 || board.turn == 0 {
        return
    }
    let (x_shift, y_shift) = HAZARD_SPIRAL_SHIFTS[((board.turn/3)-1) as usize];
    let x = center as i16 % W as i16 + x_shift as i16;
    let y = center as i16 / W as i16 + y_shift as i16;
    if x >= 0 && x < W as i16 && y >= 0 && y < H as i16 {
        board.hazards.set_bit((center as i16 + x_shift as i16 + y_shift as i16 * W as i16) as usize);
    }
}

fn get_food_spawns(gamemode: Gamemode) -> &'static [usize] {
    match gamemode {
        Gamemode::WrappedArcadeMaze => &[20, 36, 104, 137, 147, 212, 218, 224, 327, 332, 337],
        _ => &[],
    }
}

pub fn simulate_maze_food_spawns<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>)
where [(); (W*H+63)/64]: Sized {
    for i in 0..S {
        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        // kind of feed snake
        if get_food_spawns(board.gamemode).contains(&(snake.head as usize)) {
            snake.curled_bodyparts += 1;
        }
    }
}

fn update_health_with_fixed_spawns<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &mut Bitboard<S, W, H, WRAP>)
where [(); (W*H+63)/64]: Sized {
    let mut eaten = ArrayVec::<u16, S>::new();

    let mut tails = ArrayVec::<u16, S>::new();
    for snake in board.snakes {
        if snake.is_alive() {
            tails.push(snake.tail);
        }
    }
    
    for i in 0..S {
        
        let mut should_simulate_food_spawn = true;
        if board.depth < 8 {
            should_simulate_food_spawn = false;
        } else {
            for tail in &tails {
                if board.distance(board.snakes[i].head, *tail) <= 1 {
                    should_simulate_food_spawn = false;
                    break
                }
            }
        }
        

        let snake = &mut board.snakes[i];
        if snake.is_dead() {
            continue
        }
        // reduce health
        let is_on_hazard = board.hazards.get_bit(snake.head as usize) as i8;
        snake.health -= 1 + board.hazard_dmg * is_on_hazard;

        // feed snake
        if board.food.get_bit(snake.head as usize) {
            snake.health = 100;
            snake.curled_bodyparts += 1;
            snake.length += 1;
            eaten.push(snake.head); // remember which food has been eaten
        } else if i != 0 && should_simulate_food_spawn && get_food_spawns(board.gamemode).contains(&(snake.head as usize)) {
            snake.curled_bodyparts += 1;
        }

        // starvation
        if snake.is_dead() {
            snake.health = OUT_OF_HEALTH;
            board.remove_snake_body(i);
        }
    }
    // remove eaten food
    for food in eaten {
        board.food.unset_bit(food as usize);
    }
}
