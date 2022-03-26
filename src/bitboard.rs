use crate::types::*;
use crate::api::GameState;
use crate::bitset::Bitset;
use crate::ttable;
use std::hash::{Hash, Hasher};

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
    #[inline(always)]
    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
    
    #[inline(always)]
    pub fn is_dead(&self) -> bool {
        self.health < 1
    }
}

/// 112 Bytes for an 11x11 Board with 4 Snakes!
#[derive(Clone, PartialEq, Eq)]
pub struct Bitboard<const S: usize, const W: usize, const H: usize, const WRAP: bool> 
where [(); (W*H+127)/128]: Sized {
    pub bodies: [Bitset<{W*H}>; 3],
    pub snakes: [Snake; S],
    pub food: Bitset<{W*H}>,
    pub hazards: Bitset<{W*H}>,
    pub ruleset: Ruleset,
    pub hazard_dmg: i8,
    pub tt_id: u8,
    pub turn: u16,
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool> Hash for Bitboard<S, W, H, WRAP>
where [(); (W*H+127)/128]: Sized {
    fn hash<T: Hasher>(&self, state: &mut T) {
        self.bodies.hash(state);
        self.food.hash(state);
        self.hazards.hash(state);
        self.ruleset.hash(state);
        self.hazard_dmg.hash(state);
        for snake in self.snakes {
            if snake.is_alive() {
                snake.hash(state);
            }
        }
    }
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool> Bitboard<S, W, H, WRAP>
where [(); (W*H+127)/128]: Sized {
    pub const ALL_BUT_LEFT_EDGE_MASK: Bitset<{W*H}> = border_mask::<W, H>(true);
    pub const ALL_BUT_RIGHT_EDGE_MASK: Bitset<{W*H}> = border_mask::<W, H>(false);
    pub const TOP_EDGE_MASK: Bitset<{W*H}> = horizontal_edge_mask::<W, H>(true);
    pub const BOTTOM_EDGE_MASK: Bitset<{W*H}> = horizontal_edge_mask::<W, H>(false);
    pub const LEFT_EDGE_MASK: Bitset<{W*H}> = vertical_edge_mask::<W, H>(false);
    pub const RIGHT_EDGE_MASK: Bitset<{W*H}> = vertical_edge_mask::<W, H>(true);
    pub const FULL_BOARD_MASK: Bitset<{W*H}> = Bitset::<{W*H}>::with_all_bits_set();
    pub const MOVES_FROM_POSITION: [[Option<u16>; 4]; W*H] = precompute_moves::<S, W, H, WRAP>();
    pub const HAZARD_SPIRAL_SHIFTS: [(i8, i8); 144] = precompute_hazard_spiral();

    pub fn new() -> Self {
        Bitboard{
            bodies: [Bitset::new(); 3],
            snakes: [Snake{head: 0, tail: 0, length: 0, health: 0, curled_bodyparts: 0}; S],
            food: Bitset::new(),
            hazards: Bitset::new(),
            hazard_dmg: 14,
            ruleset: Ruleset::Standard,
            tt_id: 0,
            turn: 0,
        }
    }

    pub fn from_gamestate(state: GameState) -> Self {
        let mut ruleset = match state.game.ruleset["name"].as_str() {
            Some("wrapped") => Ruleset::Wrapped,
            Some("royale") => Ruleset::Royale,
            Some("constrictor") => Ruleset::Constrictor,
            _ => Ruleset::Standard,
        };
        if ruleset == Ruleset::Wrapped && state.board.hazards.len() != 0 {
            ruleset = Ruleset::WrappedSpiral(state.board.hazards[0].x as u16 + state.board.hazards[0].y as u16 * W as u16);
        }
        let mut board = Self::new();
        board.tt_id = ttable::get_tt_id(state.game.id);
        board.ruleset = ruleset;
        board.turn = state.turn as u16;
        if let Some(settings) = state.game.ruleset.get("settings") {
            board.hazard_dmg = if let Some(x) = settings["hazardDamagePerTurn"].as_i64() {
                x as i8
            } else {
                14
            };
        }
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
            board.bodies[0].set_bit(W*snake.head.y + snake.head.x);
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
            if board.snakes[n].curled_bodyparts == 0 && board.ruleset != Ruleset::Constrictor {
                board.bodies[0].unset_bit(board.snakes[n].tail as usize);
            }
        }
        board
    }

    /// Returns true if self is dead or the only one alive
    pub fn is_terminal(&self) -> bool {
        if self.snakes[0].is_dead() {
            return true
        }
        for i in 1..S {
            if self.snakes[i].is_alive() {
                return false
            }
        }
        true
    }

    /// Returns wether the game is over and the winner id, if there is a winner
    pub fn is_over(&self) -> (bool, Option<usize>) {
        let mut winner = None;
        for i in 0..S {
            if self.snakes[i].is_alive() {
                if winner == None {
                    winner = Some(i);
                } else {
                    return (false, None)
                }
            }
        }
        (true, winner)
    }

    pub fn distance(&self, from: u16, to: u16) -> u16 {
        let w = W as u16;
        let dist_x = (from%w).max(to%w) - (from%w).min(to%w);
        let dist_y = (from/w).max(to/w) - (from/w).min(to/w);
        if WRAP {
            dist_x.min(w - dist_x) + dist_y.min(H as u16 - dist_y)
        } else {
            dist_x + dist_y
        }
    }

    pub fn is_in_direction(&self, from: u16, to: u16, mv: Move) -> bool {
        let w = W as u16;
        if WRAP {
            let f;
            let t;
            match mv {
                Move::Left => {
                    f = from % w;
                    t = to % w;
                    (f > t && f - t < w / 2) || (f < t && t - f > w - 2)
                },
                Move::Right => {
                    f = from % w;
                    t = to % w;
                    (f > t && f - t > w / 2) || (f < t && t - f < w - 2)
                },
                Move::Down => {
                    f = from / w;
                    t = to / w;
                    (f > t && f - t < w / 2) || (f < t && t - f > w - 2)
                },
                Move::Up => {
                    f = from / w;
                    t = to / w;
                    (f > t && f - t > w / 2) || (f < t && t - f < w - 2)
                },
            }
        } else {
            match mv {
                Move::Left => from % w > to % w,
                Move::Right => from % w < to % w,
                Move::Down => from / w > to / w,
                Move::Up => from / w < to / w,
            }
        }
    }

    /// Returns the last move that was made by a snake.
    /// None is returned if the snake has not made a move yet.
    pub fn get_previous_move(&self, snake_index: usize) -> Option<Move> {
        if WRAP {
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

    pub fn apply_moves(&mut self, moves: &[Move; S]) {
        self.turn += 1;
        let mut eaten = ArrayVec::<u16, S>::new();
        for i in 0..S {
            let snake = &mut self.snakes[i];
            if snake.is_dead() {
                continue
            }

            // move snake
            let mv = moves[i];
            let mv_int = mv.to_int();
            // set direction of new body part
            self.bodies[1].set(snake.head as usize, (mv_int&1) != 0);
            self.bodies[2].set(snake.head as usize, (mv_int>>1) != 0);
            // set new head
            snake.head = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[snake.head as usize][mv.to_int() as usize].expect("move out of bounds") as u16;

            // move old tail if necessary
            if self.ruleset != Ruleset::Constrictor {
                if snake.curled_bodyparts == 0 {
                    let mut tail_mask = Bitset::<{W*H}>::with_bit_set(snake.tail as usize);
                    let tail_move_int = (self.bodies[1] & tail_mask).any() as u8 | ((self.bodies[2] & tail_mask).any() as u8) << 1;
                    snake.tail = if WRAP {
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
            }

            // reduce health
            let is_on_hazard = self.hazards.get_bit(snake.head as usize) as i8;
            snake.health -= 1 + self.hazard_dmg * is_on_hazard;

            // feed snake
            let is_on_food = self.food.get_bit(snake.head as usize);
            snake.health += (100 - snake.health) * is_on_food as i8;
            snake.curled_bodyparts += is_on_food as u8;
            snake.length += is_on_food as u8;
            if is_on_food {
                eaten.push(snake.head); // remember which food has been eaten
            }

            // starvation
            if snake.is_dead() {
                snake.health = OUT_OF_HEALTH;
                self.remove_snake_body(i);
            }
        }

        // sanity checks for snake movement
        #[cfg(debug_assertions)]
        for snake in self.snakes {
            if snake.is_dead() {
                continue
            }
            debug_assert!(self.bodies[0].get_bit(snake.tail as usize), "snake tail is not set in bodies bitmap, before it should be removed\n{:?}", self);
        }

        // a 2nd iteration is needed to deal with collisions, since starved snakes cannot collide
        for i in 0..S {
            if self.snakes[i].is_dead() {
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

        // remove collided snakes and mark new heads
        for i in 0..S {
            // remove collided snakes
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
            } else if self.snakes[i].is_alive() {
                // set snake heads in bodies
                // we do this last, since it would break collision checks earlier, but we want this info
                // for move gen on the new board, since moving into the current space of a head is illegal
                self.bodies[0].set_bit(self.snakes[i].head as usize);
                // unset tail bits for snakes that have no curled bodyparts 
                // we do this, since it is allowed to move there and we can effectively treat these
                // spaces as empty for the next move
                // we also do this last, since we need it earlier for collision checks of this turn
                if self.snakes[i].curled_bodyparts == 0 && self.ruleset != Ruleset::Constrictor {
                    self.bodies[0].unset_bit(self.snakes[i].tail as usize);
                }
            }
        }

        // remove eaten food
        for food in eaten {
            self.food.unset_bit(food as usize);
        }

        self.inc_spiral_hazards();
    }

    fn inc_spiral_hazards(&mut self) {
        if let Ruleset::WrappedSpiral(center) = self.ruleset {
            let round = self.turn % 3;
            if round != 0 || self.turn / 3 > 142 || self.turn == 0 {
                return
            }
            let (x_shift, y_shift) = Self::HAZARD_SPIRAL_SHIFTS[((self.turn/3)-1) as usize];
            let x = center as i16 % W as i16 + x_shift as i16;
            let y = center as i16 / W as i16 + y_shift as i16;
            if x >= 0 && x < W as i16 && y >= 0 && y < H as i16 {
                self.hazards.set_bit((center as i16 + x_shift as i16 + y_shift as i16 * W as i16) as usize);
            }
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
            tail_pos = if WRAP {
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

/// Computes possible moves from every position at compile time
const fn precompute_moves<const S: usize, const W: usize, const H: usize, const WRAP: bool>
() -> [[Option<u16>; 4]; W*H]
where [(); (W*H+127)/128]: Sized, [(); W*H]: Sized {
    let mut result = [[None; 4]; {W*H}];
    let mut pos = 0;
    loop {
        if pos == W*H {
            break
        }
        if WRAP {
            // up
            let move_to = (pos + W) % (W*H);
            result[pos][0] = Some(move_to as u16);
            
            // down
            let move_to = if W > pos { W*(H-1) + pos } else { pos - W };
            result[pos][1] = Some(move_to as u16);
            
            // right
            let move_to = if pos % W == W-1 { pos - (W-1) } else { pos + 1};
            result[pos][2] = Some(move_to as u16);
            
            // left
            let move_to = if pos % W == 0 { pos + (W-1) } else { pos - 1 };
            result[pos][3] = Some(move_to as u16);
        } else {
            // up
            if pos < W * (H-1) {
                let move_to = pos + W;
                result[pos][0] = Some(move_to as u16);
            }
            // down
            if pos >= W {
                let move_to = pos - W;
                result[pos][1] = Some(move_to as u16);
            }
            // right
            if pos % W < W - 1 {
                let move_to = pos + 1;
                result[pos][2] = Some(move_to as u16);
            }
            // left
            if pos % W > 0 {
                let move_to = pos - 1;
                result[pos][3] = Some(move_to as u16);
            }
        }

        pos += 1;
    }
    result
}

const fn precompute_hazard_spiral() -> [(i8, i8); 144] {
    [ (0,0), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1), (-1, 2), (0, 2), (1, 2), (2, 2), (2, 1), (2, 0), (2, -1), (2, -2), (1, -2), (0, -2), (-1, -2), (-2, -2), (-2, -1), (-2, 0), (-2, 1), (-2, 2), (-2, 3), (-1, 3), (0, 3), (1, 3), (2, 3), (3, 3), (3, 2), (3, 1), (3, 0), (3, -1), (3, -2), (3, -3), (2, -3), (1, -3), (0, -3), (-1, -3), (-2, -3), (-3, -3), (-3, -2), (-3, -1), (-3, 0), (-3, 1), (-3, 2), (-3, 3), (-3, 4), (-2, 4), (-1, 4), (0, 4), (1, 4), (2, 4), (3, 4), (4, 4), (4, 3), (4, 2), (4, 1), (4, 0), (4, -1), (4, -2), (4, -3), (4, -4), (3, -4), (2, -4), (1, -4), (0, -4), (-1, -4), (-2, -4), (-3, -4), (-4, -4), (-4, -3), (-4, -2), (-4, -1), (-4, 0), (-4, 1), (-4, 2), (-4, 3), (-4, 4), (-4, 5), (-3, 5), (-2, 5), (-1, 5), (0, 5), (1, 5), (2, 5), (3, 5), (4, 5), (5, 5), (5, 4), (5, 3), (5, 2), (5, 1), (5, 0), (5, -1), (5, -2), (5, -3), (5, -4), (5, -5), (4, -5), (3, -5), (2, -5), (1, -5), (0, -5), (-1, -5), (-2, -5), (-3, -5), (-4, -5), (-5, -5), (-5, -4), (-5, -3), (-5, -2), (-5, -1), (-5, 0), (-5, 1), (-5, 2), (-5, 3), (-5, 4), (-5, 5), (-5, 6), (-4, 6), (-3, 6), (-2, 6), (-1, 6), (0, 6), (1, 6), (2, 6), (3, 6), (4, 6), (5, 6), (6, 6), (6, 5), (6, 4), (6, 3), (6, 2), (6, 1), (6, 0), (6, -1), (6, -2), (6, -3), (6, -4), (6, -5) ]
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool> std::fmt::Debug for Bitboard<S, W, H, WRAP>
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
                f.write_str(if self.bodies[0].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s.0 } else if self.hazards.get_bit((W*(H-1-i))+j) { "+" } else { "." })?;
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
        f.write_str(&("turn: ".to_string() + &self.turn.to_string() + "\n"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
    use crate::move_gen;
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
    fn bench_simulate(b: &mut Bencher) {
        let mut board = create_board();
        b.iter(|| {
            let moves = move_gen::limited_move_combinations(&board, 0);
            board.apply_moves(&moves[0])
        })
    }

    #[bench]
    fn bench_remove_snake_body(b: &mut Bencher) {
        let mut board = create_board();
        board.snakes[1].health = 0;
        b.iter(|| {
            let mut b = board.clone();
            b.remove_snake_body(1);
            b
        })
    }
}
