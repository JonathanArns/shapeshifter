use crate::types::*;
use crate::api::GameState;

const BODY_COLLISION: i8 = -1;
const OUT_OF_HEALTH: i8 = -2;
const HEAD_COLLISION: i8 = -3;

#[derive(Clone, Copy, Debug, Hash)]
pub struct Snake {
    pub head: u8,
    pub tail: u8,
    pub length: u8,
    pub health: i8,
    pub curled_bodyparts: u8,
}

impl Snake {
    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

/// 104 Bytes for an 11x11 Board with 4 Snakes!
#[derive(Clone)]
pub struct Bitboard<const N: usize> {
    pub bodies: [u128; 3],
    pub snakes: [Snake; N],
    pub food: u128,
    pub hazards: u128,
}

pub fn distance(x: u8, y: u8) -> u8 {
    ((x/11).max(y/11) - (x/11).min(y/11)) + ((x%11).max(y%11) - (x%11).min(y%11))
}

pub fn is_in_direction(from: u8, to: u8, mv: Move) -> bool {
    match mv {
        Move::Left => from % 11 > to % 11,
        Move::Right => from % 11 < to % 11,
        Move::Down => from / 11 > to / 11,
        Move::Up => from / 11 < to / 11,
    }
}

impl<const N: usize> Bitboard<N> {
    pub fn new() -> Self {
        Bitboard{
            bodies: [0; 3],
            snakes: [Snake{head: 0, tail: 0, length: 0, health: 0, curled_bodyparts: 0}; N],
            food: 0,
            hazards: 0,
        }
    }

    pub fn from_gamestate(state: GameState) -> Self {
        let mut board = Self::new();
        for food in state.board.food {
            board.food |= 1<<(11*food.y + food.x)
        }
        for hazard in state.board.hazards {
            board.hazards |= 1<<(11*hazard.y + hazard.x)
        }
        let mut m = 0;
        let mut n;
        for snake in state.board.snakes {
            if snake.id == state.you.id {
                n = 0;
            } else {
                m += 1;
                n = m;
            }
            board.snakes[n].health = snake.health as i8;
            board.snakes[n].length = snake.length as u8;
            board.snakes[n].head = 11*snake.head.y as u8 + snake.head.x as u8;
            board.snakes[n].tail = 11*snake.body[snake.body.len()-1].y as u8 + snake.body[snake.body.len()-1].x as u8;
            let mut prev_mask = 1<<board.snakes[n].head;
            let mut mask;
            for bod in snake.body[1..].iter() {
                mask = 1<<(11*bod.y + bod.x);
                if mask == prev_mask {
                    board.snakes[n].curled_bodyparts += 1;
                    continue
                }    
                board.bodies[0] |= mask;
                board.bodies[1] |= mask * ((prev_mask < mask) as u128);
                board.bodies[2] |= mask * ((prev_mask & (mask | mask<<1 | mask>>1) != 0) as u128);
                prev_mask = mask;
            }
        }
        board
    }

    pub fn is_terminal(&self) -> bool {
        if !self.snakes[0].is_alive() {
            return true
        }
        for i in 1..N {
            if self.snakes[i].is_alive() {
                return false
            }
        }
        true
    }

    pub fn apply_moves(&self, moves: &[Move; N]) -> Bitboard<N> {
        let mut new = self.clone();
        let mut eaten = 0;
        for i in 0..N {
            let snake = &mut new.snakes[i];
            if !snake.is_alive() {
                continue
            }

            // move snake
            let mv = moves[i];
            let old_head_mask = 1<<snake.head;
            let mv_int = mv.to_int();
            // set new body part
            new.bodies[0] |= old_head_mask;
            new.bodies[1] |= ((mv_int&1) as u128)<<snake.head;
            new.bodies[2] |= ((mv_int>>1) as u128)<<snake.head;
            // set new head
            snake.head = (snake.head as i8 + mv.to_index(11)) as u8; // TODO: support other board sizes
            // move old tail if necessary
            if snake.curled_bodyparts == 0 {
                let tail_mask = 1<<snake.tail;
                snake.tail = (
                    snake.tail as i8 
                    + Move::int_to_index(
                        (new.bodies[1] & tail_mask != 0) as u8 
                            | (((new.bodies[2] & tail_mask != 0) as u8) << 1),
                        11
                    )
                ) as u8;
                new.bodies[0] &= !tail_mask;
                new.bodies[1] &= !tail_mask;
                new.bodies[2] &= !tail_mask;
            } else {
                snake.curled_bodyparts -= 1;
            }

            // reduce health
            let new_head = 1<<snake.head;
            let is_on_hazard = ((new.hazards & new_head) != 0) as i8;
            snake.health -= 1 + 15 * is_on_hazard;

            // feed snake
            let head_and_food = new.food & new_head;
            let is_on_food = ((new.food & new_head) != 0) as i8;
            snake.health += (100 - snake.health) * is_on_food;
            snake.curled_bodyparts += is_on_food as u8;
            snake.length += is_on_food as u8;
            eaten |= head_and_food; // remember which food has been eaten

            // starvation
            if !snake.is_alive() {
                snake.health = OUT_OF_HEALTH;
                new.remove_snake_body(i);
            }
        }

        // a 2nd iteration is needed to deal with collisions, since starved snakes cannot collide
        for i in 0..N {
            if !new.snakes[i].is_alive() {
                continue
            }
            // body collisions
            if new.bodies[0] & 1<<new.snakes[i].head != 0 {
                new.snakes[i].curled_bodyparts = 100; // marked for removal
                continue
            }
            // head to head collisions
            for j in 0..N {
                if i != j
                && new.snakes[j].is_alive()
                && new.snakes[i].head == new.snakes[j].head
                && new.snakes[i].length <= new.snakes[j].length {
                    new.snakes[i].curled_bodyparts = 101; // marked for removal
                }
            }
        }

        // remove collided snakes
        for i in 0..N {
            if new.snakes[i].curled_bodyparts == 100 {
                new.snakes[i].curled_bodyparts = 0;
                new.snakes[i].health = BODY_COLLISION;
                new.remove_snake_body(i);
            }
            if new.snakes[i].curled_bodyparts == 101 {
                new.snakes[i].curled_bodyparts = 0;
                new.snakes[i].health = HEAD_COLLISION;
                new.remove_snake_body(i);
            }
        }

        // remove eaten food
        new.food &= !eaten;

        new
    }

    pub fn remove_snake_body(&mut self, snake_index: usize) {
        let snake = &self.snakes[snake_index];
        let head_mask = 1<<snake.head;
        let mut tail_mask = 1<<snake.tail;
        while head_mask != tail_mask {
            let first_bit = self.bodies[1] & tail_mask != 0;
            let vertical = self.bodies[2] & tail_mask == 0;
            self.bodies[0] &= !tail_mask;
            self.bodies[1] &= !tail_mask;
            self.bodies[2] &= !tail_mask;
            let shift_distance = 1 + (11-1) * vertical as u8;
            if first_bit {
                tail_mask >>= shift_distance;
            } else {
                tail_mask <<= shift_distance;
            }
        }
    }

}

#[allow(unused)]
fn print_area_control(me: u128, enemies: u128, w: u8) {
    let mut debug = "".to_string();
    for i in 0..11 {
        for j in 0..11 {
            debug.push_str(if 1<<((w*(w-1-i))+j) & me != 0 { "x " } else if enemies & 1<<((w*(w-1-i))+j) != 0 { "o " } else { ". " });
        }
        debug.push_str("\n");
    }
    println!("{}", debug);
}

impl<const N: usize> std::fmt::Debug for Bitboard<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..11 {
            for j in 0..11 {
                f.write_str(if 1<<((11*(10-i))+j) & self.bodies[0] != 0 { "x" } else if self.snakes[0].head == (11*(10-i))+j { "@" } else { "." })?;
                f.write_str(if 1<<((11*(10-i))+j) & self.bodies[2] != 0 { "x" } else if self.snakes[0].head == (11*(10-i))+j { "@" } else { "." })?;
                f.write_str(if 1<<((11*(10-i))+j) & self.bodies[1] != 0 { "x " } else if self.snakes[0].head == (11*(10-i))+j { "@ " } else { ". " })?;
            }
            f.write_str("\n")?;
        }
        for snake in self.snakes {
            f.write_str(&("head: ".to_string() + &snake.head.to_string() + " tail: " + &snake.tail.to_string() + " length: " + &snake.length.to_string() + " health: " + &snake.health.to_string() + "\n"))?;
        }
        Ok(())
    }
}
