use crate::types::*;
use crate::api::GameState;
use crate::bitset::Bitset;

use arrayvec::ArrayVec;

const BODY_COLLISION: i8 = -1;
const OUT_OF_HEALTH: i8 = -2;
const HEAD_COLLISION: i8 = -3;

#[derive(Clone, Copy, Debug, Hash)]
pub struct Snake {
    pub head: u16,
    pub tail: u16,
    pub length: u8,
    pub health: i8,
    pub curled_bodyparts: u8,
}

impl Snake {
    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

/// 112 Bytes for an 11x11 Board with 4 Snakes!
#[derive(Clone)]
pub struct Bitboard<const S: usize, const W: usize, const H: usize> 
where [(); (W*H+127)/128]: Sized {
    pub bodies: [Bitset<{W*H}>; 3],
    pub snakes: [Snake; S],
    pub food: Bitset<{W*H}>,
    pub hazards: Bitset<{W*H}>,
}

impl<const S: usize, const W: usize, const H: usize> Bitboard<S, W, H>
where [(); (W*H+127)/128]: Sized {
    pub fn new() -> Self {
        Bitboard{
            bodies: [Bitset::new(); 3],
            snakes: [Snake{head: 0, tail: 0, length: 0, health: 0, curled_bodyparts: 0}; S],
            food: Bitset::new(),
            hazards: Bitset::new(),
        }
    }



    pub fn from_gamestate(state: GameState) -> Self {
        let mut board = Self::new();
        for food in state.board.food {
            board.food.set_bit(W*food.y + food.x);
        }
        for hazard in state.board.hazards {
            board.hazards.set_bit(W*hazard.y + hazard.x);
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
            board.snakes[n].head = (W*snake.head.y) as u16 + snake.head.x as u16;
            board.snakes[n].tail = (W*snake.body[snake.body.len()-1].y) as u16 + snake.body[snake.body.len()-1].x as u16;
            let mut prev_pos = board.snakes[n].head;
            let mut pos;
            for bod in snake.body[1..].iter() {
                pos = (W*bod.y + bod.x) as u16;
                if pos == prev_pos {
                    board.snakes[n].curled_bodyparts += 1;
                    continue
                }    
                board.bodies[0].set_bit(pos as usize);
                if prev_pos < pos {
                    board.bodies[1].set_bit(pos as usize);
                }
                if  prev_pos == pos+1 || (pos > 0 && prev_pos == pos-1) {
                    board.bodies[2].set_bit(pos as usize);
                }
                prev_pos = pos;
            }
        }
        board
    }

    pub fn is_terminal(&self) -> bool {
        if !self.snakes[0].is_alive() {
            return true
        }
        for i in 1..S {
            if self.snakes[i].is_alive() {
                return false
            }
        }
        true
    }

    // TODO: pub fn apply_moves<const RULES: Ruleset>(&self, moves: &[Move; S]) -> Self {
    pub fn apply_moves(&self, moves: &[Move; S]) -> Self {
        let mut new = self.clone();
        let mut eaten = ArrayVec::<u16, S>::new();
        for i in 0..S {
            let snake = &mut new.snakes[i];
            if !snake.is_alive() {
                continue
            }

            // move snake
            let mv = moves[i];
            let mv_int = mv.to_int();
            // set new body part
            new.bodies[0].set_bit(snake.head as usize);
            new.bodies[1].set(snake.head as usize, (mv_int&1) != 0);
            new.bodies[2].set(snake.head as usize, (mv_int>>1) != 0);
            // set new head
            snake.head = (snake.head as i16 + mv.to_index(W)) as u16;
            // move old tail if necessary
            if snake.curled_bodyparts == 0 {
                let mut tail_mask = Bitset::<{W*H}>::with_bit_set(snake.tail as usize);
                snake.tail = (
                    snake.tail as i16 
                    + Move::int_to_index(
                        (new.bodies[1] & tail_mask).any() as u8 
                            | (((new.bodies[2] & tail_mask).any() as u8) << 1),
                        W
                    )
                ) as u16;
                tail_mask = !tail_mask;
                new.bodies[0] &= tail_mask;
                new.bodies[1] &= tail_mask;
                new.bodies[2] &= tail_mask;
            } else {
                snake.curled_bodyparts -= 1;
            }

            // reduce health
            let is_on_hazard = new.hazards.get_bit(snake.head as usize) as i8;
            snake.health -= 1 + 15 * is_on_hazard;

            // feed snake
            let is_on_food = new.food.get_bit(snake.head as usize);
            snake.health += (100 - snake.health) * is_on_food as i8;
            snake.curled_bodyparts += is_on_food as u8;
            snake.length += is_on_food as u8;
            if is_on_food {
                eaten.push(snake.head); // remember which food has been eaten
            }

            // starvation
            if !snake.is_alive() {
                snake.health = OUT_OF_HEALTH;
                new.remove_snake_body(i);
            }
        }

        // a 2nd iteration is needed to deal with collisions, since starved snakes cannot collide
        for i in 0..S {
            if !new.snakes[i].is_alive() {
                continue
            }
            // body collisions
            if new.bodies[0].get_bit(new.snakes[i].head as usize) {
                new.snakes[i].curled_bodyparts = 100; // marked for removal
                continue
            }
            // head to head collisions
            for j in 0..S {
                if i != j
                && new.snakes[j].is_alive()
                && new.snakes[i].head == new.snakes[j].head
                && new.snakes[i].length <= new.snakes[j].length {
                    new.snakes[i].curled_bodyparts = 101; // marked for removal
                }
            }
        }

        // remove collided snakes
        for i in 0..S {
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
        for food in eaten {
            new.food.unset_bit(food as usize);
        }

        new
    }

    pub fn remove_snake_body(&mut self, snake_index: usize) {
        let snake = &self.snakes[snake_index];
        let mut tail_pos = snake.tail;
        while snake.head != tail_pos {
            let first_bit = self.bodies[1].get_bit(tail_pos as usize);
            let vertical = !self.bodies[2].get_bit(tail_pos as usize);
            self.bodies[0].unset_bit(tail_pos as usize);
            self.bodies[1].unset_bit(tail_pos as usize);
            self.bodies[2].unset_bit(tail_pos as usize);
            let shift_distance = 1 + (W-1) as u16 * vertical as u16;
            if first_bit {
                tail_pos -= shift_distance;
            } else {
                tail_pos += shift_distance;
            }
        }
    }

}

impl<const S: usize, const W: usize, const H: usize> std::fmt::Debug for Bitboard<S, W, H>
where [(); (W*H+127)/128]: Sized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..H {
            for j in 0..W {
                f.write_str(if self.bodies[0].get_bit((W*(H-1-i))+j) { "x" } else if self.snakes[0].head as usize == (W*(H-1-i))+j { "@" } else { "." })?;
                f.write_str(if self.bodies[2].get_bit((W*(H-1-i))+j) { "x" } else if self.snakes[0].head as usize == (W*(H-1-i))+j { "@" } else { "." })?;
                f.write_str(if self.bodies[1].get_bit((W*(H-1-i))+j) { "x " } else if self.snakes[0].head as usize == (W*(H-1-i))+j { "@ " } else { ". " })?;
            }
            f.write_str("\n")?;
        }
        for snake in self.snakes {
            f.write_str(&("head: ".to_string() + &snake.head.to_string() + " tail: " + &snake.tail.to_string() + " length: " + &snake.length.to_string() + " health: " + &snake.health.to_string() + " curled: " + &snake.curled_bodyparts.to_string() + "\n"))?;
        }
        Ok(())
    }
}
