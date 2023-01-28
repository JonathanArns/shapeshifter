use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use colored::{Colorize, Color};
use serde_json;
use std::fs::File;
use std::io::prelude::*;
use bitssset::Bitset;

use crate::minimax;
use crate::wire_rep;

mod constants;
mod rules;
pub mod moves;
pub mod move_gen;

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
    /// Returns the appropriate gamemode for a gamestate.
    fn from_gamestate(state: &wire_rep::GameState) -> Self {
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

    // Returns the gamemode's name as a string.
    pub fn get_name(&self) -> String {
        match *self {
            Gamemode::Standard => "standard".to_string(),
            Gamemode::Wrapped => "wrapped".to_string(),
            Gamemode::Constrictor => "constrictor".to_string(),
            Gamemode::WrappedSpiral => "wrapped-spiral".to_string(),
            Gamemode::WrappedSinkholes => "wrapped-sinkholes".to_string(),
            Gamemode::WrappedWithHazard => "wrapped-with-hazard".to_string(),
            Gamemode::StandardWithHazard => "standard-with-hazard".to_string(),
            Gamemode::WrappedArcadeMaze => "wrapped-arcade-maze".to_string(),
            Gamemode::WrappedIslandsBridges => "wrapped-islands-bridges".to_string(),
        }
    }

    /// Returns the battlesnake map name associated with this gamemode.
    pub fn get_map_name(&self) -> String {
        match *self {
            Gamemode::Standard | Gamemode::Wrapped | Gamemode::Constrictor => "standard".to_string(),
            Gamemode::WrappedSpiral => "hz_spiral".to_string(),
            Gamemode::WrappedSinkholes => "hz_spiral".to_string(),
            Gamemode::WrappedWithHazard | Gamemode::StandardWithHazard => "royale".to_string(),
            Gamemode::WrappedArcadeMaze => "arcade_maze".to_string(),
            Gamemode::WrappedIslandsBridges => "hz_islands_bridges".to_string(),
        }
    }

    /// Returns the ruleset name associated with this gamemode.
    pub fn get_ruleset_name(&self) -> String {
        match *self {
            Gamemode::Standard | Gamemode::StandardWithHazard => "standard".to_string(),
            Gamemode::Constrictor => "constrictor".to_string(),
            Gamemode::Wrapped 
            | Gamemode::WrappedSpiral 
            | Gamemode::WrappedSinkholes 
            | Gamemode::WrappedWithHazard 
            | Gamemode::WrappedArcadeMaze 
            | Gamemode::WrappedIslandsBridges => "wrapped".to_string(),
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
pub struct Bitboard<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8>
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

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8> Hash for Bitboard<S, W, H, WRAP, HZSTACK, SILLY>
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

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8> Bitboard<S, W, H, WRAP, HZSTACK, SILLY>
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

    /// Deserializes a json move request string to a Bitboard.
    pub fn from_str(s: &str) -> Result<Self, serde_json::Error> {
        let state_result = serde_json::from_str::<wire_rep::GameState>(s);
        state_result.map(|state| {Self::from_gamestate(state)})
    }

    /// Serializes the Bitboard to a move request string.
    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.to_gamestate())
    }

    /// Transforms this bitboard into a gamestate.
    /// This is lossy and mostly useful for serialization.
    pub fn to_gamestate(&self) -> wire_rep::GameState {
        // food and hazards
        let mut wire_food = vec![];
        let mut wire_hazards = vec![];
        for x in 0..W {
            for y in 0..H {
                if self.food.get(x+(y*W)) {
                    wire_food.push(wire_rep::Coord{x: x.into(), y: y.into()});
                }
                if self.hazard_mask.get(x+(y*W)) {
                    wire_hazards.push(wire_rep::Coord{x: x.into(), y: y.into()});
                    if HZSTACK {
                        for _ in 0..(self.hazards[x+(y*W)]-1) {
                            wire_hazards.push(wire_rep::Coord{x: x.into(), y: y.into()});
                        }
                    }
                }
            }
        }

        // snakes
        let mut wire_snakes = vec![];
        for (i, snake) in self.snakes.iter().enumerate() {
            if snake.is_dead() {
                continue
            }
            let mut wire_snake = wire_rep::Battlesnake{
                id: "".to_string(),
                name: "".to_string(),
                health: snake.health.into(),
                length: snake.length.into(),
                head: wire_rep::Coord{ x: (snake.head as usize%W).into(), y: (snake.head as usize/W).into() },
                shout: None,
                squad: None,
                body: vec![],
            };
            let mut tail_pos = snake.tail;
            while snake.head != tail_pos {
                let next_pos = self.next_body_segment(tail_pos);
                wire_snake.body.insert(0, wire_rep::Coord{x: (tail_pos as usize%W).into(), y: (tail_pos as usize/W).into()});
                tail_pos = next_pos;
            }
            wire_snake.body.insert(0, wire_rep::Coord{x: (tail_pos as usize%W).into(), y: (tail_pos as usize/W).into()});
            wire_snakes.push(wire_snake);
        }

        // ruleset
        let mut settings = serde_json::Map::<String, serde_json::Value>::default();
        settings.insert("hazardDamagePerTurn".to_string(), self.hazard_dmg.into());
        let mut ruleset = serde_json::Map::<String, serde_json::Value>::default();
        ruleset.insert("name".to_string(), self.gamemode.get_ruleset_name().into());
        ruleset.insert("settings".to_string(), settings.into());

        wire_rep::GameState{
            turn: self.turn.into(),
            game: wire_rep::Game{
                id: "".to_string(),
                timeout: 500,
                source: "local".to_string(),
                map: self.gamemode.get_map_name(),
                ruleset,
            },
            you: wire_snakes[0].clone(),
            board: wire_rep::Board{
                height: H,
                width: W,
                snakes: wire_snakes,
                food: wire_food,
                hazards: wire_hazards,
            },
        }
    }

    pub fn from_gamestate(state: wire_rep::GameState) -> Self {
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
        board.tt_id = minimax::get_tt_id(state.game.id + &state.you.id);
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
                // at the beginning of the loop, set head direction (used for silly move gen)
                if prev_pos == board.snakes[n].head {
                    board.bodies[1].set(board.snakes[n].head as usize, board.bodies[1].get(pos as usize));
                    board.bodies[2].set(board.snakes[n].head as usize, board.bodies[2].get(pos as usize));
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
        WRAP || None != Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::MOVES_FROM_POSITION[from as usize][mv.to_int() as usize]
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
        let move_int = self.bodies[1].get(pos as usize) as u8 | (self.bodies[2].get(pos as usize) as u8) << 1;
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

    /// Generates input features for neural networks
    pub fn get_nn_input(&self) -> [u8; W*H*7] {
        let MY_HEAD: usize = 0*W*H;
        let MY_TAIL: usize = 1*W*H;
        let ENEMY_HEADS: usize = 2*W*H;
        let ENEMY_TAILS: usize = 3*W*H;
        let BODIES: usize = 4*W*H;
        let FOOD: usize = 5*W*H;
        let HAZARDS: usize = 6*W*H;
        let MY_HEALTH: usize = 7*W*H;
        let ENEMY_HEALTH: usize = 7*W*H+1;
        let MY_LENGTH: usize = 7*W*H+2;
        let ENEMY_LENGTH: usize = 7*W*H+3;

        let mut features = [0; W*H*7];

        let me = self.snakes[0];
        features[MY_HEAD + me.head as usize] = 1;
        features[MY_TAIL + me.tail as usize] = 1;
        // features[MY_HEALTH] = me.health as u8;
        // features[MY_LENGTH] = me.length as u8;

        // features[ENEMY_HEALTH] = u8::MAX; 
        for snake in self.snakes[1..].iter() {
            if snake.is_dead() {
                continue
            }
            features[ENEMY_HEADS + snake.head as usize] = 1;
            features[ENEMY_TAILS + snake.tail as usize] = 1;
            // if snake.length > features[ENEMY_LENGTH] {
            //     features[ENEMY_LENGTH] = snake.length as u8;
            // }
            // if snake.health < features[ENEMY_HEALTH] as i8 {
            //     features[ENEMY_HEALTH] = snake.health as u8; 
            // }
        }

        for i in 0..(W*H) {
            features[BODIES + i] = self.bodies[0].get(i) as u8;
            features[FOOD + i] = self.food.get(i) as u8;
            features[HAZARDS + i] = self.hazard_mask.get(i) as u8;
        }
        features
    }
    
    /// Appends the board in json format and the score to file.
    pub fn write_to_file_with_score(&self, score: minimax::Score, file_name_suffix: &str)
    where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
        let path = format!(
            "./data/{}_{}-{}x{}-{}-{}_boards_{}.csv",
            self.gamemode.get_name(),
            S, W, H,
            if WRAP { "WRAP" } else { "NOWRAP" },
            if HZSTACK { "STACK" } else { "NOSTACK" },
            file_name_suffix
        );
        let mut file = File::options()
            .create(true)
            .append(true)
            .open(path)
            .expect("coudln't create file");
        if let Err(e) = writeln!(file, "{};{}", score, self.to_string().unwrap()) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8> std::fmt::Debug for Bitboard<S, W, H, WRAP, HZSTACK, SILLY>
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
                let mut tile = if self.bodies[0].get((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." }.to_string();
                tile.push_str(if self.bodies[2].get((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." });
                tile.push_str(if self.bodies[1].get((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." });
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

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool, const SILLY: u8> std::fmt::Display for Bitboard<S, W, H, WRAP, HZSTACK, SILLY>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // decide on colors for the individual snakes
        let colors = [Color::Red, Color::Green, Color::Cyan, Color::Yellow, Color::Blue, Color::Magenta];
        let mut snake_colors: HashMap<usize, Color> = HashMap::default();
        for (i, snake) in self.snakes.iter().enumerate() {
            let mut tail_pos = snake.tail;
            while snake.head != tail_pos {
                snake_colors.insert(tail_pos as usize, colors[i % colors.len()]);
                let move_int = self.bodies[1].get(tail_pos as usize) as u8 | (self.bodies[2].get(tail_pos as usize) as u8) << 1;
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
                let mut tile = if self.bodies[0].get((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." }.to_string();
                tile.push_str(if self.bodies[2].get((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." });
                tile.push_str(if self.bodies[1].get((W*(H-1-i))+j) { "x" } else if let Some(s) = head_str { s } else { "." });
                let mut colored_tile = tile.color(Color::BrightWhite);
                if self.hazard_mask.get(W*(H-1-i)+j) {
                    colored_tile = tile.on_color(Color::White);
                }
                if self.food.get((W*(H-1-i))+j) {
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
    use crate::wire_rep;
    use test::Bencher;

    fn create_board() -> Bitboard<4, 11, 11, true, false, 0> {
        let val = r###"{"game":{"id":"7ddd5c60-e27a-42ae-985e-f056e5695836","ruleset":{"name":"wrapped","version":"?","settings":{"foodSpawnChance":15,"minimumFood":1,"hazardDamagePerTurn":100,"royale":{},"squad":{"allowBodyCollisions":false,"sharedElimination":false,"sharedHealth":false,"sharedLength":false}}},"map":"hz_islands_bridges","timeout":500,"source":"league"},"turn":445,"board":{"width":11,"height":11,"food":[{"x":1,"y":9},{"x":1,"y":8},{"x":9,"y":1},{"x":6,"y":3},{"x":7,"y":3},{"x":7,"y":4},{"x":8,"y":3},{"x":4,"y":9},{"x":10,"y":8},{"x":6,"y":6}],"hazards":[{"x":5,"y":10},{"x":5,"y":9},{"x":5,"y":7},{"x":5,"y":6},{"x":5,"y":5},{"x":5,"y":4},{"x":5,"y":3},{"x":5,"y":0},{"x":5,"y":1},{"x":6,"y":5},{"x":7,"y":5},{"x":9,"y":5},{"x":10,"y":5},{"x":4,"y":5},{"x":3,"y":5},{"x":1,"y":5},{"x":0,"y":5},{"x":1,"y":10},{"x":9,"y":10},{"x":1,"y":0},{"x":9,"y":0},{"x":10,"y":1},{"x":10,"y":0},{"x":10,"y":10},{"x":10,"y":9},{"x":0,"y":10},{"x":0,"y":9},{"x":0,"y":1},{"x":0,"y":0},{"x":0,"y":6},{"x":0,"y":4},{"x":10,"y":6},{"x":10,"y":4},{"x":6,"y":10},{"x":4,"y":10},{"x":6,"y":0},{"x":4,"y":0}],"snakes":[{"id":"gs_P3P9rW63VPgMcYFFJ9R6McrM","name":"Shapeshifter","health":91,"body":[{"x":6,"y":2},{"x":6,"y":1},{"x":7,"y":1},{"x":7,"y":0},{"x":7,"y":10},{"x":8,"y":10},{"x":8,"y":0},{"x":8,"y":1},{"x":8,"y":2},{"x":9,"y":2},{"x":9,"y":3},{"x":10,"y":3},{"x":10,"y":2},{"x":0,"y":2},{"x":0,"y":3},{"x":1,"y":3},{"x":1,"y":4},{"x":2,"y":4},{"x":3,"y":4},{"x":3,"y":3},{"x":2,"y":3},{"x":2,"y":2},{"x":1,"y":2},{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":0},{"x":3,"y":0},{"x":3,"y":1},{"x":4,"y":1},{"x":4,"y":2}],"latency":11,"head":{"x":6,"y":2},"length":30,"shout":"","squad":"","customizations":{"color":"#900050","head":"cosmic-horror-special","tail":"cosmic-horror"}},{"id":"gs_YMFKJHvJwS9VV7SgtTMVmKVQ","name":"🇺🇦 Jagwire 🇺🇦","health":76,"body":[{"x":9,"y":9},{"x":8,"y":9},{"x":7,"y":9},{"x":6,"y":9},{"x":6,"y":8},{"x":5,"y":8},{"x":4,"y":8},{"x":3,"y":8},{"x":3,"y":9},{"x":3,"y":10},{"x":2,"y":10},{"x":2,"y":9},{"x":2,"y":8},{"x":2,"y":7},{"x":3,"y":7},{"x":4,"y":7},{"x":4,"y":6},{"x":3,"y":6},{"x":2,"y":6},{"x":1,"y":6},{"x":1,"y":7},{"x":0,"y":7},{"x":10,"y":7},{"x":9,"y":7},{"x":9,"y":6},{"x":8,"y":6},{"x":7,"y":6},{"x":7,"y":7},{"x":7,"y":8},{"x":8,"y":8},{"x":9,"y":8}],"latency":23,"head":{"x":9,"y":9},"length":31,"shout":"","squad":"","customizations":{"color":"#ffd900","head":"smile","tail":"wave"}}]},"you":{"id":"gs_P3P9rW63VPgMcYFFJ9R6McrM","name":"Shapeshifter","health":91,"body":[{"x":6,"y":2},{"x":6,"y":1},{"x":7,"y":1},{"x":7,"y":0},{"x":7,"y":10},{"x":8,"y":10},{"x":8,"y":0},{"x":8,"y":1},{"x":8,"y":2},{"x":9,"y":2},{"x":9,"y":3},{"x":10,"y":3},{"x":10,"y":2},{"x":0,"y":2},{"x":0,"y":3},{"x":1,"y":3},{"x":1,"y":4},{"x":2,"y":4},{"x":3,"y":4},{"x":3,"y":3},{"x":2,"y":3},{"x":2,"y":2},{"x":1,"y":2},{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":0},{"x":3,"y":0},{"x":3,"y":1},{"x":4,"y":1},{"x":4,"y":2}],"latency":11,"head":{"x":6,"y":2},"length":30,"shout":"","squad":"","customizations":{"color":"#900050","head":"cosmic-horror-special","tail":"cosmic-horror"}}}"###;
        Bitboard::<4, 11, 11, true, false, 0>::from_str(&val).unwrap()
    }

    #[test]
    fn test_bitboard_serde() {
        let board = create_board();
        let copy = Bitboard::<4, 11, 11, true, false, 0>::from_str(&board.to_string().unwrap()).unwrap();
        assert_eq!(board.food, copy.food);
        assert_eq!(board.hazards, copy.hazards);
        assert_eq!(board.hazard_mask, copy.hazard_mask);
        assert_eq!(board.snakes, copy.snakes);
        assert_eq!(board.bodies, copy.bodies);
        assert_eq!(board.turn, copy.turn);
        assert_eq!(board.gamemode, copy.gamemode);
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
