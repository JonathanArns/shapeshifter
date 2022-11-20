use crate::api::GameState;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use colored::{Colorize, Color};
#[cfg(not(feature = "mcts"))]
use crate::minimax;

mod bitset;
mod constants;
#[macro_use]
mod rules;
pub mod moves;
pub mod move_gen;

pub use bitset::Bitset;
pub use moves::Move;

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Gamemode {
    Standard,
    StandardWithHazard,

    Wrapped,
    WrappedWithHazard,
    WrappedSpiral,
    WrappedArcadeMaze,
    WrappedSinkholes,
    WrappedIslandsBridges,

    Constrictor,
}

impl Gamemode {
    fn from_gamestate(state: &GameState) -> Self {
        match state.game.ruleset["name"].as_str() {
            Some("constrictor") | Some("wrapped-constrictor") => Self::Constrictor,
            Some("wrapped") => match state.game.map.as_str() {
                "arcade_maze" => Self::WrappedArcadeMaze,
                "hz_spiral" => Self::WrappedSpiral,
                "hz_islands_bridges" => Self::WrappedIslandsBridges,
                "sinkholes" => Self::WrappedSinkholes,
                _ if state.board.hazards.len() == 0 => Self::Wrapped,
                _ => Self::WrappedWithHazard,
            },
            _ => match state.game.map.as_str() {
                _ if state.board.hazards.len() == 0 => Self::Standard,
                _ => Self::StandardWithHazard,
            },
        }
    }
}


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

pub const fn hz_stack_len<const STACK: bool, const W: usize, const H: usize>() -> usize {
    if STACK {
        W*H
    } else {
        0
    }
}

#[derive(Clone)]
pub struct Bitboard<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool> 
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    pub bodies: [Bitset<{W*H}>; 3],
    pub snakes: [Snake; S],
    pub food: Bitset<{W*H}>,
    pub hazards: [u8; hz_stack_len::<HZSTACK, W, H>()],
    pub hazard_mask: Bitset<{W*H}>,
    pub hazard_dmg: i8,
    pub tt_id: u8,
    pub turn: u16,
    pub gamemode: Gamemode,
    pub apply_moves: Arc<dyn Fn(&mut Self, &[Move; S]) + Send + Sync>,
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool> Hash for Bitboard<S, W, H, WRAP, HZSTACK>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    fn hash<T: Hasher>(&self, state: &mut T) {
        self.bodies.hash(state);
        self.food.hash(state);
        self.hazards.hash(state);
        self.gamemode.hash(state);
        self.hazard_dmg.hash(state);
        for snake in self.snakes {
            if snake.is_alive() {
                snake.hash(state);
            }
        }
    }
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool> Bitboard<S, W, H, WRAP, HZSTACK>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    pub const FULL_BOARD_MASK: Bitset<{W*H}> = Bitset::<{W*H}>::with_all_bits_set();
    pub const CHECKER_BOARD_MASK: Bitset<{W*H}> = constants::checker_board_mask::<W, H>();
    pub const ALL_BUT_LEFT_EDGE_MASK: Bitset<{W*H}> = constants::border_mask::<W, H>(true);
    pub const ALL_BUT_RIGHT_EDGE_MASK: Bitset<{W*H}> = constants::border_mask::<W, H>(false);
    pub const TOP_EDGE_MASK: Bitset<{W*H}> = constants::horizontal_edge_mask::<W, H>(true);
    pub const BOTTOM_EDGE_MASK: Bitset<{W*H}> = constants::horizontal_edge_mask::<W, H>(false);
    pub const LEFT_EDGE_MASK: Bitset<{W*H}> = constants::vertical_edge_mask::<W, H>(true);
    pub const RIGHT_EDGE_MASK: Bitset<{W*H}> = constants::vertical_edge_mask::<W, H>(false);
    pub const MOVES_FROM_POSITION: [[Option<u16>; 4]; W*H] = constants::precompute_moves::<S, W, H, WRAP>();

    pub fn new() -> Self {
        Bitboard{
            bodies: [Bitset::new(); 3],
            snakes: [Snake{head: 0, tail: 0, length: 0, health: 0, curled_bodyparts: 0}; S],
            food: Bitset::new(),
            hazards: [0; hz_stack_len::<HZSTACK, W, H>()],
            hazard_mask: Bitset::new(),
            hazard_dmg: 14,
            gamemode: Gamemode::Standard,
            tt_id: 0,
            turn: 0,
            apply_moves: Arc::new(|board, mvs| {}),
        }
    }

    pub fn from_gamestate(state: GameState) -> Self {
        let mut board = Self::new();
        rules::attach_rules(&mut board, &state);
        board.gamemode = Gamemode::from_gamestate(&state);
        board.turn = state.turn as u16;
        if let Some(settings) = state.game.ruleset.get("settings") {
            board.hazard_dmg = if let Some(x) = settings["hazardDamagePerTurn"].as_i64() {
                x as i8
            } else {
                14
            };
        }
        #[cfg(not(feature = "mcts"))]
        {
            board.tt_id = minimax::get_tt_id(state.game.id + &state.you.id);
        }
        for food in state.board.food {
            board.food.set_bit(W*food.y + food.x);
        }
        for hazard in state.board.hazards {
            board.hazard_mask.set_bit(W*hazard.y + hazard.x);
            if HZSTACK {
                board.hazards[W*hazard.y + hazard.x] += 1;
            }
        }
        let mut m = 0;
        let mut n;
        for snake in state.board.snakes {
            if snake.head == state.you.head {
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
                if pos == prev_pos + 1 || pos == prev_pos + W as u16 || WRAP && prev_pos == pos + W as u16 - 1 || WRAP && prev_pos == pos + (H as u16 - 1) * W as u16 {
                    board.bodies[1].set_bit(pos as usize);
                }
                if  prev_pos == pos + 1 || prev_pos + 1 == pos || WRAP && prev_pos == pos + W as u16 - 1 || WRAP && prev_pos + W as u16 - 1 == pos {
                    board.bodies[2].set_bit(pos as usize);
                }
                prev_pos = pos;
            }
            if board.snakes[n].curled_bodyparts == 0 && board.gamemode != Gamemode::Constrictor {
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

    /// Returns the wdl value of the position, if it is terminal
    pub fn win_draw_loss(&self) -> Option<i8> {
        let me_alive = self.snakes[0].is_alive();
        let mut enemies_alive = 0;
        for i in 1..S {
            if self.snakes[i].is_alive() {
                enemies_alive += 1;
            }
        }
        if me_alive {
            if enemies_alive == 0 {
                Some(1)
            } else {
                None
            }
        } else {
            if enemies_alive != 0 {
                Some(-1)
            } else {
                Some(0)
            }
        }
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

    pub fn is_legal_move(&self, from: u16, mv: Move) -> bool {
        WRAP || None != Bitboard::<S, W, H, WRAP, HZSTACK>::MOVES_FROM_POSITION[from as usize][mv.to_int() as usize]
    }

    pub fn is_legal_enemy_moves(&self, mvs: [Move; S]) -> bool {
        for i in 1..S {
            if !self.is_legal_move(self.snakes[i].head, mvs[i]) {
                return false
            }
        }
        true
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
            let next_pos = self.next_body_segment(tail_pos);
            self.bodies[0].unset_bit(tail_pos as usize);
            self.bodies[1].unset_bit(tail_pos as usize);
            self.bodies[2].unset_bit(tail_pos as usize);
            tail_pos = next_pos;
        }
    }

    // Gets next segment in snake body in head direction.
    // Does not check if pos is actually on a snake.
    pub fn next_body_segment(&self, pos: u16) -> u16 {
        let move_int = self.bodies[1].get_bit(pos as usize) as u8 | (self.bodies[2].get_bit(pos as usize) as u8) << 1;
        if WRAP {
            (pos as i16 + Move::int_to_index_wrapping(move_int, W, H, pos)) as u16
        } else {
            (pos as i16 + Move::int_to_index(move_int, W)) as u16
        }
    }

    pub fn kill_snake(&mut self, snake_index: usize) {
        self.snakes[snake_index].health = -1;
        self.remove_snake_body(snake_index);
    }

    fn coord_string_from_index(&self, idx: u16) -> String {
        let x = idx % W as u16;
        let y = idx / W as u16;
        "(".to_string() + &x.to_string() + " " + &y.to_string() + ")"
    }
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool> std::fmt::Debug for Bitboard<S, W, H, WRAP, HZSTACK>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // draw the board
        for i in 0..H {
            for j in 0..W {
                let mut head_str = None;
                if self.snakes[0].head as usize == (W*(H-1-i))+j {
                    head_str = Some("@");
                } else {
                    for snake in self.snakes[1..].iter() {
                        if snake.head as usize == (W*(H-1-i))+j {
                            head_str = Some("E");
                        }
                    }
                }
                let mut tile = if self.bodies[0].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." }.to_string();
                tile.push_str(if self.bodies[2].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." });
                tile.push_str(if self.bodies[1].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." });
                f.write_str(&format!("{} ", tile))?;
            }
            f.write_str("\n")?;
        }

        // print metadata
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

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool> std::fmt::Display for Bitboard<S, W, H, WRAP, HZSTACK>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // decide on colors for the individual snakes
        let colors = [Color::Red, Color::Green, Color::Cyan, Color::Yellow, Color::Blue, Color::Magenta];
        let mut snake_colors: HashMap<usize, Color> = HashMap::default();
        for (i, snake) in self.snakes.iter().enumerate() {
            let mut tail_pos = snake.tail;
            while snake.head != tail_pos {
                snake_colors.insert(tail_pos as usize, colors[i % colors.len()]);
                let move_int = self.bodies[1].get_bit(tail_pos as usize) as u8 | (self.bodies[2].get_bit(tail_pos as usize) as u8) << 1;
                tail_pos = if WRAP {
                    tail_pos as i16 + Move::int_to_index_wrapping(move_int, W, H, tail_pos)
                } else {
                    tail_pos as i16 + Move::int_to_index(move_int, W)
                } as u16;
            }
            snake_colors.insert(tail_pos as usize, colors[i % colors.len()]);
        }

        // draw the board
        for i in 0..H {
            for j in 0..W {
                let mut head_str = None;
                if self.snakes[0].head as usize == (W*(H-1-i))+j {
                    head_str = Some("@");
                } else {
                    for snake in self.snakes[1..].iter() {
                        if snake.head as usize == (W*(H-1-i))+j {
                            head_str = Some("E");
                        }
                    }
                }
                let mut tile = if self.bodies[0].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." }.to_string();
                tile.push_str(if self.bodies[2].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." });
                tile.push_str(if self.bodies[1].get_bit((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." });
                let mut colored_tile = tile.color(Color::BrightWhite);
                if self.hazard_mask.get_bit(W*(H-1-i)+j) {
                    colored_tile = tile.on_color(Color::White);
                }
                if self.food.get_bit((W*(H-1-i))+j) {
                    colored_tile = tile.on_color(Color::Magenta);
                }
                if let Some(c) = snake_colors.get(&((W*(H-1-i))+j)) {
                    colored_tile = colored_tile.color(*c);
                }
                f.write_str(&format!("{} ", colored_tile))?;
            }
            f.write_str("\n")?;
        }

        // print metadata
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
    use test::Bencher;

    fn c(x: usize, y: usize) -> api::Coord {
        api::Coord{x, y}
    }

    fn create_board() -> Bitboard<4, 11, 11, true, false> {
        let mut ruleset = std::collections::HashMap::new();
        ruleset.insert("name".to_string(), serde_json::Value::String("wrapped".to_string()));
        let state = api::GameState{
            game: api::Game{ id: "".to_string(), timeout: 100, ruleset, map: "standard".to_string(), source: "".to_string() },
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
        Bitboard::<4, 11, 11, true, false>::from_gamestate(state)
    }
    
    #[bench]
    fn bench_simulate(b: &mut Bencher) {
        let mut board = create_board();
        b.iter(|| {
            let moves = move_gen::limited_move_combinations(&board, 0);
            (board.apply_moves.clone())(&mut board, &moves[0])
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
