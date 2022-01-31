use crate::types::*;
use crate::api::GameState;
use crate::bitset::Bitset;

use arrayvec::ArrayVec;

const BODY_COLLISION: i8 = -1;
const OUT_OF_HEALTH: i8 = -2;
const HEAD_COLLISION: i8 = -3;
const EVEN_HEAD_COLLISION: i8 = -4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Bitboard<const S: usize, const W: usize, const H: usize> 
where [(); (W*H+127)/128]: Sized {
    pub bodies: [Bitset<{W*H}>; 3],
    pub snakes: [Snake; S],
    pub food: Bitset<{W*H}>,
    pub hazards: Bitset<{W*H}>,
    pub wrap: bool,
}

// TODO: missing logic for WRAP
impl<const S: usize, const W: usize, const H: usize> Bitboard<S, W, H>
where [(); (W*H+127)/128]: Sized {
    pub const ALL_BUT_LEFT_EDGE_MASK: Bitset<{W*H}> = border_mask::<W, H>(true);
    pub const ALL_BUT_RIGHT_EDGE_MASK: Bitset<{W*H}> = border_mask::<W, H>(false);
    pub const TOP_EDGE_MASK: Bitset<{W*H}> = horizontal_edge_mask::<W, H>(true);
    pub const BOTTOM_EDGE_MASK: Bitset<{W*H}> = horizontal_edge_mask::<W, H>(false);
    pub const LEFT_EDGE_MASK: Bitset<{W*H}> = vertical_edge_mask::<W, H>(false);
    pub const RIGHT_EDGE_MASK: Bitset<{W*H}> = vertical_edge_mask::<W, H>(true);
    pub const FULL_BOARD_MASK: Bitset<{W*H}> = Bitset::<{W*H}>::with_all_bits_set();

    pub fn new() -> Self {
        Bitboard{
            bodies: [Bitset::new(); 3],
            snakes: [Snake{head: 0, tail: 0, length: 0, health: 0, curled_bodyparts: 0}; S],
            food: Bitset::new(),
            hazards: Bitset::new(),
            wrap: false,
        }
    }

    pub fn from_gamestate(state: GameState, ruleset: Ruleset) -> Self {
        let mut board = Self::new();
        board.wrap = matches!(ruleset, Ruleset::Wrapped);
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
                if pos == prev_pos + 1 || pos == prev_pos + W as u16 || prev_pos == pos + W as u16 - 1 || prev_pos == pos + (H as u16 - 1) * W as u16 {
                    board.bodies[1].set_bit(pos as usize);
                }
                if  prev_pos == pos + 1 || prev_pos + 1 == pos || prev_pos == pos + W as u16 - 1 || prev_pos + W as u16 - 1 == pos {
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

    pub fn distance(&self, from: u16, to: u16) -> u16 {
        if self.wrap {
            todo!("not implemented for wrapped boards")
        }
        let w = W as u16;
        ((from/w).max(to/w) - (from/w).min(to/w)) + ((from%w).max(to%w) - (from%w).min(to%w))
    }

    pub fn is_in_direction(&self, from: u16, to: u16, mv: Move) -> bool {
        if self.wrap {
            todo!("not implemented for wrapped boards")
        }
        let w = W as u16;
        match mv {
            Move::Left => from % w > to % w,
            Move::Right => from % w < to % w,
            Move::Down => from / w > to / w,
            Move::Up => from / w < to / w,
        }
    }

    /// Returns the last move that was made by a snake.
    /// None is returned if the snake has not made a move yet.
    pub fn get_previous_move(&self, snake_index: usize) -> Option<Move> {
        if self.wrap {
            todo!("not implemented for wrapped boards")
        }
        let head = self.snakes[snake_index].head as usize;
        if head == self.snakes[snake_index].tail as usize {
            None
        } else if head % W > 0 && !self.bodies[1].get_bit(head-1) && self.bodies[2].get_bit(head-1) {
            Some(Move::Right)
        } else if head % W < W-1 && self.bodies[1].get_bit(head+1) && self.bodies[2].get_bit(head+1) {
            Some(Move::Left)
        } else if head >= W && !self.bodies[1].get_bit(head-W) && !self.bodies[2].get_bit(head-W) {
            Some(Move::Up)
        } else {
            Some(Move::Down)
        }
    }

    pub fn apply_moves(&mut self, moves: &[Move; S], _ruleset: Ruleset) {
        let mut eaten = ArrayVec::<u16, S>::new();
        for i in 0..S {
            let snake = &mut self.snakes[i];
            if !snake.is_alive() {
                continue
            }

            // move snake
            let mv = moves[i];
            let mv_int = mv.to_int();
            // set new body part
            self.bodies[0].set_bit(snake.head as usize);
            self.bodies[1].set(snake.head as usize, (mv_int&1) != 0);
            self.bodies[2].set(snake.head as usize, (mv_int>>1) != 0);
            // set new head
            if self.wrap {
                snake.head = (snake.head as i16 + mv.to_index_wrapping(W, H, snake.head)) as u16
            } else {
                snake.head = (snake.head as i16 + mv.to_index(W)) as u16;
            }
            // move old tail if necessary
            if snake.curled_bodyparts == 0 {
                let mut tail_mask = Bitset::<{W*H}>::with_bit_set(snake.tail as usize);
                let tail_move_int = (self.bodies[1] & tail_mask).any() as u8 | ((self.bodies[2] & tail_mask).any() as u8) << 1;
                snake.tail = if self.wrap {
                    snake.tail as i16 + Move::int_to_index_wrapping(tail_move_int, W, H, snake.tail)
                } else {
                    snake.tail as i16 + Move::int_to_index(tail_move_int, W)
                } as u16;
                tail_mask = !tail_mask;
                self.bodies[0] &= tail_mask;
                self.bodies[1] &= tail_mask;
                self.bodies[2] &= tail_mask;
            } else {
                snake.curled_bodyparts -= 1;
            }

            // reduce health
            let is_on_hazard = self.hazards.get_bit(snake.head as usize) as i8;
            snake.health -= 1 + 15 * is_on_hazard;

            // feed snake
            let is_on_food = self.food.get_bit(snake.head as usize);
            snake.health += (100 - snake.health) * is_on_food as i8;
            snake.curled_bodyparts += is_on_food as u8;
            snake.length += is_on_food as u8;
            if is_on_food {
                eaten.push(snake.head); // remember which food has been eaten
            }

            // starvation
            if !snake.is_alive() {
                snake.health = OUT_OF_HEALTH;
                self.remove_snake_body(i);
            }
        }

        // sanity checks for snake movement
        for snake in self.snakes {
            if !snake.is_alive() {
                continue
            }
            debug_assert!(self.bodies[0].get_bit(snake.tail as usize), "snake tail is not set in bodies bitmap\n{:?}", self);
        }

        // a 2nd iteration is needed to deal with collisions, since starved snakes cannot collide
        for i in 0..S {
            if !self.snakes[i].is_alive() {
                continue
            }
            // body collisions
            if self.bodies[0].get_bit(self.snakes[i].head as usize) {
                self.snakes[i].curled_bodyparts = 100; // marked for removal
                continue
            }
            // head to head collisions
            for j in 0..S {
                if i != j
                && self.snakes[j].is_alive()
                && self.snakes[i].head == self.snakes[j].head {
                    if self.snakes[i].length < self.snakes[j].length {
                        self.snakes[i].curled_bodyparts = 101; // marked for removal
                    } else if self.snakes[i].length == self.snakes[j].length {
                        self.snakes[i].curled_bodyparts = 102; // marked for removal
                    }
                }
            }
        }

        // remove collided snakes
        for i in 0..S {
            if self.snakes[i].curled_bodyparts >= 100 {
                if self.snakes[i].curled_bodyparts == 100 {
                    self.snakes[i].health = BODY_COLLISION;
                } else if self.snakes[i].curled_bodyparts == 101 {
                    self.snakes[i].health = HEAD_COLLISION;
                } else if self.snakes[i].curled_bodyparts == 102 {
                    self.snakes[i].health = EVEN_HEAD_COLLISION;
                }
                self.snakes[i].curled_bodyparts = 0;
                self.remove_snake_body(i);
            }
        }

        // remove eaten food
        for food in eaten {
            self.food.unset_bit(food as usize);
        }
    }

    pub fn remove_snake_body(&mut self, snake_index: usize) {
        if S <= 2 || snake_index == 0 {
            return  // this is a terminal state, so we can ignore the dead body
        }
        let snake = &self.snakes[snake_index];
        let mut tail_pos = snake.tail;
        let mut debug_counter = 0;
        while snake.head != tail_pos {
            debug_counter += 1;
            debug_assert!(debug_counter < 10000, "endless loop in remove_snake_body\n{:?}", self);
            let move_int = self.bodies[1].get_bit(tail_pos as usize) as u8 | (self.bodies[2].get_bit(tail_pos as usize) as u8) << 1;
            self.bodies[0].unset_bit(tail_pos as usize);
            self.bodies[1].unset_bit(tail_pos as usize);
            self.bodies[2].unset_bit(tail_pos as usize);
            tail_pos = if self.wrap {
                tail_pos as i16 + Move::int_to_index_wrapping(move_int, W, H, tail_pos)
            } else {
                tail_pos as i16 + Move::int_to_index(move_int, W)
            } as u16;
        }
    }

    fn coord_string_from_index(&self, idx: u16) -> String {
        let x = idx % W as u16;
        let y = idx / W as u16;
        "(".to_string() + &x.to_string() + " " + &y.to_string() + ")"
    }
}

/// Computes ALL_BUT_LEFT_EDGE_MASK and ALL_BUT_RIGHT_EDGE_MASK
const fn border_mask<const W: usize, const H: usize>(left: bool) -> Bitset<{W*H}>
where [(); (W*H+127)/128]: Sized {
    let mut arr = [0_u128; (W*H+127)/128];
    let mut i = 0;
    let mut j;
    loop {
        if i == H {
            break
        }
        if left {
            j = 0;
        } else {
            j = 1;
        }
        loop {
            if left && j == W-1 {
                break
            } else if !left && j == W {
                break
            }
            let idx = (i*W+j)>>7;
            let offset = (i*W+j) % 128;
            arr[idx] |= 1_u128<<offset;

            j += 1;
        }
        i += 1;
    }
    Bitset::<{W*H}>::from_array(arr)
}

/// Computes LEFT_EDGE_MASK and RIGHT_EDGE_MASK
const fn vertical_edge_mask<const W: usize, const H: usize>(right: bool) -> Bitset<{W*H}>
where [(); (W*H+127)/128]: Sized {
    let mut arr = [0_u128; (W*H+127)/128];
    let mut i = 0;
    let j = if right { W-1 } else { 0 };
    loop {
        if i == W {
            break
        }
        let idx = (i*W+j) >>7;
        let offset = (i*W+j) % 128;
        arr[idx] |= 1_u128<<offset;
        i += 1;
    }
    Bitset::<{W*H}>::from_array(arr)
}

/// Computes TOP_EDGE_MASK and BOTTOM_EDGE_MASK
const fn horizontal_edge_mask<const W: usize, const H: usize>(top: bool) -> Bitset<{W*H}>
where [(); (W*H+127)/128]: Sized {
    let mut arr = [0_u128; (W*H+127)/128];
    let i = if top { H-1 } else { 0 };
    let mut j = 0;
    loop {
        if j == W {
            break
        }
        let idx = (i*W+j) >>7;
        let offset = (i*W+j) % 128;
        arr[idx] |= 1_u128<<offset;
        j += 1;
    }
    Bitset::<{W*H}>::from_array(arr)
}

impl<const S: usize, const W: usize, const H: usize> std::fmt::Debug for Bitboard<S, W, H>
where [(); (W*H+127)/128]: Sized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..H {
            for j in 0..W {
                let mut head_str = None;
                if self.snakes[0].head as usize == (W*(H-1-i))+j {
                    head_str = Some(("@", "@ "));
                } else {
                    for snake in self.snakes[1..].iter() {
                        if snake.head as usize == (W*(H-1-i))+j {
                            head_str = Some(("E", "E "));
                        }
                    }
                }
                f.write_str(if self.bodies[0].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s.0 } else { "." })?;
                f.write_str(if self.bodies[2].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s.0 } else { "." })?;
                f.write_str(if self.bodies[1].get_bit((W*(H-1-i))+j) { "x " } else if let Some(s) = head_str { s.1 } else { ". " })?;
            }
            f.write_str("\n")?;
        }
        for snake in self.snakes {
            f.write_str(&(
                "head: ".to_string() + &self.coord_string_from_index(snake.head)
                + " tail: " + &self.coord_string_from_index(snake.tail)
                + " length: " + &snake.length.to_string()
                + " health: " + &snake.health.to_string()
                + " curled: " + &snake.curled_bodyparts.to_string()
                + "\n"
            ))?;
        }
        Ok(())
    }
}
