use crate::types::*;
use crate::api::GameState;
use std::time;
use std::thread;
use std::env;
use crossbeam_channel::{unbounded, Sender, Receiver};
use arrayvec::ArrayVec;

const BORDER_MASK: u128 = 0b_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110_01111111110;
const BODY_COLLISION: i8 = -1;
const OUT_OF_HEALTH: i8 = -2;
const HEAD_COLLISION: i8 = -3;

lazy_static! {
    /// Weights for eval function can be loaded from environment.
    static ref WEIGHTS: [Score; 5] = if let Ok(var) = env::var("RUSPUTIN_WEIGHTS") {
        serde_json::from_str(&var).unwrap()
    } else {
        [-5, 1, 3, 1, 3]
    };
}

#[derive(Clone, Copy, Debug, Hash)]
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
        let mut best_score = Score::MIN+1;
        let mut best_depth = 1;
        let start_time = time::Instant::now();
        let soft_deadline = start_time + g.move_time / 10;
        let hard_deadline = start_time + g.move_time / 2;

        let (stop_sender, stop_receiver) = unbounded();
        let (result_sender, result_receiver) : (Sender<(Move, Score, u8)>, Receiver<(Move, Score, u8)>) = unbounded();

        let board = self.clone();
        thread::spawn(move || {
            let mut best_move = Move::Up;
            let mut best_score = Score::MIN+1;
            let mut depth = 1;
            let mut enemy_moves = board.possible_enemy_moves();
            let my_moves = board.allowed_moves(board.snakes[0].head);
            loop {
                let mut best = Score::MIN+1;
                for mv in &my_moves {
                    let score = board.alphabeta(*mv, &mut enemy_moves, depth, Score::MIN+1, Score::MAX);
                    if score > best {
                        best = score;
                        best_move = *mv;
                        best_score = best;
                    }
                }
                result_sender.try_send((best_move, best_score, depth)).ok();
                if best == Score::MAX || best < Score::MIN+4 {
                    break
                }
                if let Ok(_) = stop_receiver.try_recv() {
                    break // stop thread because time is out and response has been sent
                }
                depth += 1;
            }
        });

        // receive results
        while time::Instant::now() < soft_deadline {
            if let Ok(msg) = result_receiver.try_recv() {
                best_move = msg.0;
                best_score = msg.1;
                best_depth = msg.2
            } else {
                thread::sleep(time::Duration::from_millis(1));
            }
        }
        stop_sender.send(1).ok(); // Channel might be broken, if search returned early. We don't care.

        // wait for eventual results from still running search
        if let Ok(msg) = result_receiver.recv_timeout(hard_deadline - time::Instant::now()) {
            best_move = msg.0;
            best_score = msg.1;
            best_depth = msg.2
        }

        println!("Move: {:?}, Score: {}, Depth: {}, Time: {}", best_move, best_score, best_depth, time::Instant::now().duration_since(start_time).as_millis());
        (best_move, best_score)
    }

    pub fn alphabeta(&self, mv: Move, enemy_moves: &mut Vec<[Move; N]>, depth: u8, alpha: Score, mut beta: Score) -> Score { // min call
        if self.is_terminal() {
            return self.eval_terminal()
        }
        if depth <= 0 {
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
        let mut enemies_alive = 0;
        let mut lowest_enemy_health = 100;
        let mut largest_enemy_length = 0;
        for i in 1..N {
            if self.snakes[i].is_alive() {
                enemies_alive += 1;
                let len = self.snakes[i].length;
                if len > largest_enemy_length {
                    largest_enemy_length = len;
                }
                if self.snakes[i].health < lowest_enemy_health {
                    lowest_enemy_health = self.snakes[i].health;
                }
            }
        }
        let (my_area, enemy_area) = self.area_control();

        let mut score: Score = 0;
        // number of enemies alive
        score += WEIGHTS[0] * enemies_alive as Score;
        // difference in health to lowest enemy
        score += WEIGHTS[1] * self.snakes[0].health as Score - lowest_enemy_health as Score;
        // difference in length to longest enemy
        score += WEIGHTS[2] * self.snakes[0].length as Score - largest_enemy_length as Score;
        // difference in controlled non-hazard area
        score += WEIGHTS[3] * (my_area & !self.hazards).count_ones() as Score - (enemy_area & !self.hazards).count_ones() as Score;
        // difference in controlled food
        score += WEIGHTS[4] * (my_area & self.food).count_ones() as Score - (enemy_area & self.food).count_ones() as Score;

        score
    }

    fn eval_terminal(&self) -> Score {
        if !self.snakes[0].is_alive() {
            return Score::MIN - self.snakes[0].health as i16
        } else {
            return Score::MAX
        }
    }

    fn possible_enemy_moves(&self) -> Vec<[Move; N]> {
        // get moves for each enemy
        let enemy_moves: Vec<ArrayVec<Move, 4>> = self.snakes[1..]
            .iter()
            .map(|snake| {
                if snake.is_alive() {
                    self.allowed_moves(snake.head)
                } else {
                    let mut mvs = ArrayVec::<_, 4>::new();
                    mvs.push(Move::Up);
                    mvs
                }
            })
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
    /// because that would cause a panic elsewhere.
    fn allowed_moves(&self, pos: u8) -> ArrayVec<Move, 4> {
        let mut moves = ArrayVec::<Move, 4>::new();
        let mut some_legal_move = Move::Up;
        let mut tails = [u8::MAX; N];
        for i in 0..N {
            if self.snakes[i].is_alive() && self.snakes[i].curled_bodyparts == 0 {
                tails[i] = self.snakes[i].tail;
            }
        }
        let mask = 1<<pos;
        if pos > 10 {
            some_legal_move = Move::Down;
            if self.bodies[0] & mask >> 11 == 0 || tails.contains(&(pos - 11)) {
                moves.push(Move::Down);
            }
        }
        if pos < 110 {
            some_legal_move = Move::Up;
            if self.bodies[0] & mask << 11 == 0 || tails.contains(&(pos + 11)) {
                moves.push(Move::Up);
            }
        }
        if pos % 11 > 0 {
            some_legal_move = Move::Left;
            if self.bodies[0] & mask >> 1 == 0 || tails.contains(&(pos - 1)) {
                moves.push(Move::Left);
            }
        }
        if pos % 11 < 10 {
            some_legal_move = Move::Right;
            if self.bodies[0] & mask << 1 == 0 || tails.contains(&(pos + 1)) {
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

    fn remove_snake_body(&mut self, snake_index: usize) {
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

    fn area_control(&self) -> (u128, u128) {
        let b = !self.bodies[0];
        let mut x = (1_u128<<self.snakes[0].head, 0_u128);
        for snake in &self.snakes[1..] {
            if snake.is_alive() {
                x.1 |= 1<<snake.head;
            }
        }
        let mut y = x;
        loop {
            let me = b & (x.0 | (BORDER_MASK & x.0)<<1 | (BORDER_MASK & x.0)>>1 | x.0<<11 | x.0>>11);
            let enemies = b & (x.1 | (BORDER_MASK & x.1)<<1 | (BORDER_MASK & x.1)>>1 | x.1<<11 | x.1>>11);
            x = (me & !enemies, enemies & !me);
            if x == y {
                break
            } else {
                y = x;
            }
        }
        (x.0, x.1)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
    use test::Bencher;

    fn c(x: usize, y: usize) -> api::Coord {
        api::Coord{x, y}
    }

    #[bench]
    fn bench_enemy_move_generation(b: &mut Bencher) {
        let state = GameState{
            game: api::Game{ id: "".to_string(), timeout: 100, ruleset: std::collections::HashMap::new() },
            turn: 157,
            you: api::Battlesnake{
                id: "a".to_string(),
                name: "a".to_string(),
                latency: "".to_string(),
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
                        latency: "".to_string(),
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
                        latency: "".to_string(),
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
                        latency: "".to_string(),
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
                        latency: "".to_string(),
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
        let board = Bitboard::<4>::from_gamestate(state);
        b.iter(|| {
            board.possible_enemy_moves()
        });
    }
    
    #[test]
    fn test_weird_head_collision_deaths() {
        let state = GameState{
            game: api::Game{ id: "".to_string(), timeout: 100, ruleset: std::collections::HashMap::new() },
            turn: 157,
            you: api::Battlesnake{
                id: "a".to_string(),
                name: "a".to_string(),
                latency: "".to_string(),
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
                        latency: "".to_string(),
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
                        latency: "".to_string(),
                        shout: None,
                        squad: None,
                        health: 95,
                        length: 12,
                        head: c(3,4),
                        body: vec![c(3,4), c(2,4), c(2,5), c(3, 5), c(3,6), c(3,7), c(3,8), c(4,8), c(4,7), c(4,6), c(4,5), c(4,4)],
                    },  
                ],
            },
        };
        let mut game = Game{move_time: std::time::Duration::from_millis(state.game.timeout.into())};
        let (mv, _) = Bitboard::<2>::from_gamestate(state).iterative_deepening_search(&mut game);
        assert!(mv != Move::Up)
    }
}
