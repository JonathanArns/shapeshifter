use rocket::http::Status;
use rocket_contrib::json::{Json, JsonValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::types;
use crate::bitboard;
use crate::minimax;

#[derive(Deserialize, Serialize, Debug)]
pub struct Game {
    pub id: String,
    pub ruleset: HashMap<String, Value>,
    pub timeout: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Board {
    pub height: usize,
    pub width: usize,
    pub food: Vec<Coord>,
    pub snakes: Vec<Battlesnake>,
    pub hazards: Vec<Coord>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Battlesnake {
    pub id: String,
    pub name: String,
    pub health: u8,
    pub body: Vec<Coord>,
    pub head: Coord,
    pub length: usize,
    pub latency: String,

    // Used in non-standard game modes
    pub shout: Option<String>,
    pub squad: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Coord {
    pub x: usize,
    pub y: usize,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GameState {
    pub game: Game,
    pub turn: u32,
    pub board: Board,
    pub you: Battlesnake,
}

#[get("/")]
pub fn handle_index() -> JsonValue {
    json!({
        "apiversion": "1",
        "author": "JonathanArns",
        "color": "#c9e7ff",
        "head": "smart-caterpillar",
        "tail": "present",
    })
}

#[post("/start", format = "json", data = "<_req>")]
pub fn handle_start(_req: Json<GameState>) -> Status {
    // unsafe {
    //     crate::eval::WEIGHTS = serde_json::from_str(&std::fs::read_to_string("weights.json").unwrap()).unwrap();
    // }
    Status::Ok
}

#[post("/move", format = "json", data = "<req>")]
pub fn handle_move(req: Json<GameState>) -> JsonValue {
    let state = req.into_inner();
    let ruleset = match state.game.ruleset["name"].to_string().as_str() {
        "wrapped" => types::Ruleset::Wrapped,
        "royale" => types::Ruleset::Royale,
        "constrictor" => types::Ruleset::Constrictor,
        _ => types::Ruleset::Standard,
    };
    let mut game = types::Game{
        move_time: std::time::Duration::from_millis(state.game.timeout.into()),
        hazard_damage: 15,
        ruleset,
    };
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height) {
        (1, 7, 7) => minimax::search(&bitboard::Bitboard::<1, 7, 7>::from_gamestate(state, ruleset), &mut game),
        (2, 7, 7) => minimax::search(&bitboard::Bitboard::<2, 7, 7>::from_gamestate(state, ruleset), &mut game),
        (3, 7, 7) => minimax::search(&bitboard::Bitboard::<3, 7, 7>::from_gamestate(state, ruleset), &mut game),
        (4, 7, 7) => minimax::search(&bitboard::Bitboard::<4, 7, 7>::from_gamestate(state, ruleset), &mut game),

        (1, 11, 11) => minimax::search(&bitboard::Bitboard::<1, 11, 11>::from_gamestate(state, ruleset), &mut game),
        (2, 11, 11) => minimax::search(&bitboard::Bitboard::<2, 11, 11>::from_gamestate(state, ruleset), &mut game),
        (3, 11, 11) => minimax::search(&bitboard::Bitboard::<3, 11, 11>::from_gamestate(state, ruleset), &mut game),
        (4, 11, 11) => minimax::search(&bitboard::Bitboard::<4, 11, 11>::from_gamestate(state, ruleset), &mut game),
        (5, 11, 11) => minimax::search(&bitboard::Bitboard::<5, 11, 11>::from_gamestate(state, ruleset), &mut game),
        (6, 11, 11) => minimax::search(&bitboard::Bitboard::<6, 11, 11>::from_gamestate(state, ruleset), &mut game),
        (7, 11, 11) => minimax::search(&bitboard::Bitboard::<7, 11, 11>::from_gamestate(state, ruleset), &mut game),
        (8, 11, 11) => minimax::search(&bitboard::Bitboard::<8, 11, 11>::from_gamestate(state, ruleset), &mut game),

        (1, 19, 19) => minimax::search(&bitboard::Bitboard::<1, 19, 19>::from_gamestate(state, ruleset), &mut game),
        (2, 19, 19) => minimax::search(&bitboard::Bitboard::<2, 19, 19>::from_gamestate(state, ruleset), &mut game),
        (3, 19, 19) => minimax::search(&bitboard::Bitboard::<3, 19, 19>::from_gamestate(state, ruleset), &mut game),
        (4, 19, 19) => minimax::search(&bitboard::Bitboard::<4, 19, 19>::from_gamestate(state, ruleset), &mut game),
        (5, 19, 19) => minimax::search(&bitboard::Bitboard::<5, 19, 19>::from_gamestate(state, ruleset), &mut game),
        (6, 19, 19) => minimax::search(&bitboard::Bitboard::<6, 19, 19>::from_gamestate(state, ruleset), &mut game),
        (7, 19, 19) => minimax::search(&bitboard::Bitboard::<7, 19, 19>::from_gamestate(state, ruleset), &mut game),
        (8, 19, 19) => minimax::search(&bitboard::Bitboard::<8, 19, 19>::from_gamestate(state, ruleset), &mut game),

        (1, 25, 25) => minimax::search(&bitboard::Bitboard::<1, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (2, 25, 25) => minimax::search(&bitboard::Bitboard::<2, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (3, 25, 25) => minimax::search(&bitboard::Bitboard::<3, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (4, 25, 25) => minimax::search(&bitboard::Bitboard::<4, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (5, 25, 25) => minimax::search(&bitboard::Bitboard::<5, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (6, 25, 25) => minimax::search(&bitboard::Bitboard::<6, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (7, 25, 25) => minimax::search(&bitboard::Bitboard::<7, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (8, 25, 25) => minimax::search(&bitboard::Bitboard::<8, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (9, 25, 25) => minimax::search(&bitboard::Bitboard::<9, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (10, 25, 25) => minimax::search(&bitboard::Bitboard::<10, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (11, 25, 25) => minimax::search(&bitboard::Bitboard::<11, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (12, 25, 25) => minimax::search(&bitboard::Bitboard::<12, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (13, 25, 25) => minimax::search(&bitboard::Bitboard::<13, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (14, 25, 25) => minimax::search(&bitboard::Bitboard::<14, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (15, 25, 25) => minimax::search(&bitboard::Bitboard::<15, 25, 25>::from_gamestate(state, ruleset), &mut game),
        (16, 25, 25) => minimax::search(&bitboard::Bitboard::<16, 25, 25>::from_gamestate(state, ruleset), &mut game),
        _ => panic!("Snake count or board size not supported"),
    };
    mv.to_json()
}

#[post("/end", format = "json", data = "<_req>")]
pub fn handle_end(_req: Json<GameState>) -> Status {
    Status::Ok
}
