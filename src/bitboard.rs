use crate::types::*;
use crate::api::GameState;
use std::time;

#[derive(Clone, Copy, Debug)]
struct Snake {
    head: u8,
    tail: u8,
    length: u8,
    health: i8,
    curled_bodyparts: u8,
}

impl Snake {
    fn is_alive(&self) -> bool {
        self.health > 0
    }
}

/// 104 Bytes for an 11x11 Board with 4 Snakes!
#[derive(Clone)]
pub struct Bitboard<const N: usize> {
    bodies: [u128; 3],
    snakes: [Snake; N],
    food: u128,
    hazards: u128,
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

    pub fn iterative_deepening_search(&self, g: &mut Game) -> (Move, Score) {
        let mut best_move = Move::Up;
        let mut best_score = Score::MIN;
        let start_time = time::Instant::now();
        let mut depth = 1;

        let mut enemy_moves = self.possible_enemy_moves();
        let my_moves = self.allowed_moves(self.snakes[0].head);
        while time::Instant::now().duration_since(start_time).lt(&g.move_time.div_f32(N as f32 * 10_f32)) {
            let mut best = Score::MIN+1;
            for mv in &my_moves {
                let score = self.alphabeta(*mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX);
                if score > best {
                    best = score;
                    best_score = score;
                    best_move = *mv;
                }
            }
            if best == Score::MAX || best == Score::MIN+1 {
                break
            }
            depth += 1;
        }
        println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move, best_score, depth, time::Instant::now().duration_since(start_time).as_millis());
        (best_move, best_score)
    }

    pub fn alphabeta(&self, mv: Move, enemy_moves: &mut Vec<[Move; N]>, depth: u8, alpha: Score, mut beta: Score) -> Score { // min call
        if depth <= 0 || self.is_terminal() {
            return self.eval()
        }

        // search
        for mvs in enemy_moves { // TODO: apply move ordering
            let score = { // max call
                let mut ialpha = alpha;
                let ibeta = beta;
                mvs[0] = mv;
                let child = self.apply_moves(mvs);
                let mut next_enemy_moves = child.possible_enemy_moves();
                for mv in child.allowed_moves(child.snakes[0].head) { // TODO: apply move ordering
                    let iscore = child.alphabeta(mv, &mut next_enemy_moves, depth-1, alpha, beta);
                    if iscore > ibeta {
                        ialpha = ibeta;
                        break // same as return beta
                    }
                    if iscore > ialpha {
                        ialpha = iscore;
                    }
                }
                ialpha
            };
            if score < alpha {
                return alpha
            }
            if score < beta {
                beta = score;
            }
        }
        beta
    }

    fn eval(&self) -> Score {
        if !self.snakes[0].is_alive() {
            return Score::MIN+1
        }
        let mut score: Score = 0;
        let mut n = 0;
        for i in 1..N {
            if self.snakes[i].is_alive() {
                score -= self.snakes[i].length as i8;
                n += 1;
            }
        }
        if n == 0 {
            return Score::MAX
        }
        score += self.snakes[0].length as Score * n as Score;
        score / n
    }

    fn possible_enemy_moves(&self) -> Vec<[Move; N]> {
        // get moves for each enemy
        let enemy_moves: Vec<Vec<Move>> = self.snakes[1..]
            .iter()
            .filter(|snake| { snake.is_alive() }) // filter out dead snakes
            .map(|snake| { self.allowed_moves(snake.head) })
            .collect();

        // generate kartesian product of the possible moves
        let mut moves: Vec<[Move; N]> = vec![[Move::Up; N]];
        for (i, snake_moves) in enemy_moves.iter().enumerate() {
            moves = snake_moves.iter().flat_map(move |mv| {
                let mut tmp = moves.clone();
                for ref mut mvs in &mut tmp {
                    mvs[i+1] = *mv
                }
                tmp
            }).collect()
        }
        moves
    }

    fn is_terminal(&self) -> bool {
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

    /// Can never return a move that moves out of bounds on the board,
    /// that would cause a panic.
    fn allowed_moves(&self, pos: u8) -> Vec<Move> {
        let mut moves = Vec::with_capacity(4);
        let mut some_legal_move = Move::Up;
        let mask = 1<<pos;
        if pos > 10 {
            some_legal_move = Move::Down;
            if self.bodies[0] & mask >> 11 == 0 {
                moves.push(Move::Down);
            }
        }
        if pos < 109 {
            some_legal_move = Move::Up;
            if self.bodies[0] & mask << 11 == 0 {
                moves.push(Move::Up);
            }
        }
        if pos % 11 > 0 {
            some_legal_move = Move::Left;
            if self.bodies[0] & mask >> 1 == 0 {
                moves.push(Move::Left);
            }
        }
        if pos % 11 < 10 {
            some_legal_move = Move::Right;
            if self.bodies[0] & mask << 1 == 0 {
                moves.push(Move::Right);
            }
        }
        if moves.len() == 0 {
            moves.push(some_legal_move);
        }
        moves
    }

    fn apply_moves(&self, moves: &[Move; N]) -> Bitboard<N> {
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
                    new.snakes[i].curled_bodyparts = 100; // marked for removal
                }
            }
        }

        // remove collided snakes
        for i in 0..N {
            if new.snakes[i].curled_bodyparts == 100 {
                new.snakes[i].curled_bodyparts = 0;
                new.snakes[i].health = -1;
                new.remove_snake_body(i);
            }
        }

        // remove eaten food
        new.food &= !eaten;

        new
    }

    fn remove_snake_body(&mut self, snake_index: usize) {
        let snake = &self.snakes[snake_index];
        let head_mask = 1<<snake.head;
        let mut tail_mask = 1<<snake.tail;
        while  head_mask != tail_mask {
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

fn floodfill(bodies: u128, start: u8, width: u8, height: u8) -> u128 {
    todo!()
}

fn voronoi<const N: usize>(bodies: u128, snake_heads: [u8; N]) -> [u128; N] {
    todo!()
}
