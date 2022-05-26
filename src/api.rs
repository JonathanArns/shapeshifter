use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use axum::extract::Json;
use tokio::task;
use std::collections::HashMap;
use std::time;

use crate::bitboard;
#[cfg(not(feature = "mcts"))]
use crate::minimax;
#[cfg(feature = "mcts")]
use crate::uct;

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
    // pub latency: String,

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

pub async fn handle_index() -> Json<Value> {
    Json(json!({
        "apiversion": "1",
        "author": "JonathanArns",
        "color": "#900050",
        "head": "trans-rights-scarf",
        "tail": "skinny",
    }))
}

pub async fn handle_start(Json(_req): Json<GameState>) { }

pub async fn handle_end() {
    #[cfg(feature = "debug_tt")]
    ttable::write_debug_info();
}

#[cfg(feature = "mcts")]
pub async fn handle_move(state: Json<GameState>) -> Json<Value> {
    // let state = req.into_inner();
    let deadline = time::Instant::now() + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(80) - 80)).into());
    let ruleset = match state.game.ruleset["name"].as_str() {
        Some("wrapped") => bitboard::Ruleset::Wrapped,
        Some("royale") => bitboard::Ruleset::Royale,
        Some("constrictor") => bitboard::Ruleset::Constrictor,
        _ => bitboard::Ruleset::Standard,
    };

    #[cfg(not(feature = "spl"))]
    let (mv, _score) = match (state.board.snakes.len(), state.board.width, state.board.height, matches!(ruleset, bitboard::Ruleset::Wrapped)) {
        (1, 11, 11, true) => uct::search(&bitboard::Bitboard::<1, 11, 11, true>::from_gamestate(state), deadline),
        (2, 11, 11, true) => uct::search(&bitboard::Bitboard::<2, 11, 11, true>::from_gamestate(state), deadline),
        (3, 11, 11, true) => uct::search(&bitboard::Bitboard::<3, 11, 11, true>::from_gamestate(state), deadline),
        (4, 11, 11, true) => uct::search(&bitboard::Bitboard::<4, 11, 11, true>::from_gamestate(state), deadline),

        (1, 11, 11, false) => uct::search(&bitboard::Bitboard::<1, 11, 11, false>::from_gamestate(state), deadline),
        (2, 11, 11, false) => uct::search(&bitboard::Bitboard::<2, 11, 11, false>::from_gamestate(state), deadline),
        (3, 11, 11, false) => uct::search(&bitboard::Bitboard::<3, 11, 11, false>::from_gamestate(state), deadline),
        (4, 11, 11, false) => uct::search(&bitboard::Bitboard::<4, 11, 11, false>::from_gamestate(state), deadline),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, please enable the 'spl' feature.", state.board.snakes.len(), state.board.width, state.board.height),
    };

    #[cfg(feature = "spl")]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, matches!(ruleset, bitboard::Ruleset::Wrapped)) {
        (1, 7, 7, true) => uct::search(&bitboard::Bitboard::<1, 7, 7, true>::from_gamestate(state), deadline),
        (2, 7, 7, true) => uct::search(&bitboard::Bitboard::<2, 7, 7, true>::from_gamestate(state), deadline),
        (3, 7, 7, true) => uct::search(&bitboard::Bitboard::<3, 7, 7, true>::from_gamestate(state), deadline),
        (4, 7, 7, true) => uct::search(&bitboard::Bitboard::<4, 7, 7, true>::from_gamestate(state), deadline),
        (5, 7, 7, true) => uct::search(&bitboard::Bitboard::<5, 7, 7, true>::from_gamestate(state), deadline),
        (6, 7, 7, true) => uct::search(&bitboard::Bitboard::<6, 7, 7, true>::from_gamestate(state), deadline),
        (7, 7, 7, true) => uct::search(&bitboard::Bitboard::<7, 7, 7, true>::from_gamestate(state), deadline),
        (8, 7, 7, true) => uct::search(&bitboard::Bitboard::<8, 7, 7, true>::from_gamestate(state), deadline),

        (1, 7, 7, false) => uct::search(&bitboard::Bitboard::<1, 7, 7, false>::from_gamestate(state), deadline),
        (2, 7, 7, false) => uct::search(&bitboard::Bitboard::<2, 7, 7, false>::from_gamestate(state), deadline),
        (3, 7, 7, false) => uct::search(&bitboard::Bitboard::<3, 7, 7, false>::from_gamestate(state), deadline),
        (4, 7, 7, false) => uct::search(&bitboard::Bitboard::<4, 7, 7, false>::from_gamestate(state), deadline),
        (5, 7, 7, false) => uct::search(&bitboard::Bitboard::<5, 7, 7, false>::from_gamestate(state), deadline),
        (6, 7, 7, false) => uct::search(&bitboard::Bitboard::<6, 7, 7, false>::from_gamestate(state), deadline),
        (7, 7, 7, false) => uct::search(&bitboard::Bitboard::<7, 7, 7, false>::from_gamestate(state), deadline),
        (8, 7, 7, false) => uct::search(&bitboard::Bitboard::<8, 7, 7, false>::from_gamestate(state), deadline),

        (1, 11, 11, true) => uct::search(&bitboard::Bitboard::<1, 11, 11, true>::from_gamestate(state), deadline),
        (2, 11, 11, true) => uct::search(&bitboard::Bitboard::<2, 11, 11, true>::from_gamestate(state), deadline),
        (3, 11, 11, true) => uct::search(&bitboard::Bitboard::<3, 11, 11, true>::from_gamestate(state), deadline),
        (4, 11, 11, true) => uct::search(&bitboard::Bitboard::<4, 11, 11, true>::from_gamestate(state), deadline),
        (5, 11, 11, true) => uct::search(&bitboard::Bitboard::<5, 11, 11, true>::from_gamestate(state), deadline),
        (6, 11, 11, true) => uct::search(&bitboard::Bitboard::<6, 11, 11, true>::from_gamestate(state), deadline),
        (7, 11, 11, true) => uct::search(&bitboard::Bitboard::<7, 11, 11, true>::from_gamestate(state), deadline),
        (8, 11, 11, true) => uct::search(&bitboard::Bitboard::<8, 11, 11, true>::from_gamestate(state), deadline),

        (1, 11, 11, false) => uct::search(&bitboard::Bitboard::<1, 11, 11, false>::from_gamestate(state), deadline),
        (2, 11, 11, false) => uct::search(&bitboard::Bitboard::<2, 11, 11, false>::from_gamestate(state), deadline),
        (3, 11, 11, false) => uct::search(&bitboard::Bitboard::<3, 11, 11, false>::from_gamestate(state), deadline),
        (4, 11, 11, false) => uct::search(&bitboard::Bitboard::<4, 11, 11, false>::from_gamestate(state), deadline),
        (5, 11, 11, false) => uct::search(&bitboard::Bitboard::<5, 11, 11, false>::from_gamestate(state), deadline),
        (6, 11, 11, false) => uct::search(&bitboard::Bitboard::<6, 11, 11, false>::from_gamestate(state), deadline),
        (7, 11, 11, false) => uct::search(&bitboard::Bitboard::<7, 11, 11, false>::from_gamestate(state), deadline),
        (8, 11, 11, false) => uct::search(&bitboard::Bitboard::<8, 11, 11, false>::from_gamestate(state), deadline),

        (1, 19, 19, true) => uct::search(&bitboard::Bitboard::<1, 19, 19, true>::from_gamestate(state), deadline),
        (2, 19, 19, true) => uct::search(&bitboard::Bitboard::<2, 19, 19, true>::from_gamestate(state), deadline),
        (3, 19, 19, true) => uct::search(&bitboard::Bitboard::<3, 19, 19, true>::from_gamestate(state), deadline),
        (4, 19, 19, true) => uct::search(&bitboard::Bitboard::<4, 19, 19, true>::from_gamestate(state), deadline),
        (5, 19, 19, true) => uct::search(&bitboard::Bitboard::<5, 19, 19, true>::from_gamestate(state), deadline),
        (6, 19, 19, true) => uct::search(&bitboard::Bitboard::<6, 19, 19, true>::from_gamestate(state), deadline),
        (7, 19, 19, true) => uct::search(&bitboard::Bitboard::<7, 19, 19, true>::from_gamestate(state), deadline),
        (8, 19, 19, true) => uct::search(&bitboard::Bitboard::<8, 19, 19, true>::from_gamestate(state), deadline),
        (9, 19, 19, true) => uct::search(&bitboard::Bitboard::<9, 19, 19, true>::from_gamestate(state), deadline),
        (10, 19, 19, true) => uct::search(&bitboard::Bitboard::<10, 19, 19, true>::from_gamestate(state), deadline),
        (11, 19, 19, true) => uct::search(&bitboard::Bitboard::<11, 19, 19, true>::from_gamestate(state), deadline),
        (12, 19, 19, true) => uct::search(&bitboard::Bitboard::<12, 19, 19, true>::from_gamestate(state), deadline),
        (13, 19, 19, true) => uct::search(&bitboard::Bitboard::<13, 19, 19, true>::from_gamestate(state), deadline),
        (14, 19, 19, true) => uct::search(&bitboard::Bitboard::<14, 19, 19, true>::from_gamestate(state), deadline),
        (15, 19, 19, true) => uct::search(&bitboard::Bitboard::<15, 19, 19, true>::from_gamestate(state), deadline),
        (16, 19, 19, true) => uct::search(&bitboard::Bitboard::<16, 19, 19, true>::from_gamestate(state), deadline),

        (1, 19, 19, false) => uct::search(&bitboard::Bitboard::<1, 19, 19, false>::from_gamestate(state), deadline),
        (2, 19, 19, false) => uct::search(&bitboard::Bitboard::<2, 19, 19, false>::from_gamestate(state), deadline),
        (3, 19, 19, false) => uct::search(&bitboard::Bitboard::<3, 19, 19, false>::from_gamestate(state), deadline),
        (4, 19, 19, false) => uct::search(&bitboard::Bitboard::<4, 19, 19, false>::from_gamestate(state), deadline),
        (5, 19, 19, false) => uct::search(&bitboard::Bitboard::<5, 19, 19, false>::from_gamestate(state), deadline),
        (6, 19, 19, false) => uct::search(&bitboard::Bitboard::<6, 19, 19, false>::from_gamestate(state), deadline),
        (7, 19, 19, false) => uct::search(&bitboard::Bitboard::<7, 19, 19, false>::from_gamestate(state), deadline),
        (8, 19, 19, false) => uct::search(&bitboard::Bitboard::<8, 19, 19, false>::from_gamestate(state), deadline),
        (9, 19, 19, false) => uct::search(&bitboard::Bitboard::<9, 19, 19, false>::from_gamestate(state), deadline),
        (10, 19, 19, false) => uct::search(&bitboard::Bitboard::<10, 19, 19, false>::from_gamestate(state), deadline),
        (11, 19, 19, false) => uct::search(&bitboard::Bitboard::<11, 19, 19, false>::from_gamestate(state), deadline),
        (12, 19, 19, false) => uct::search(&bitboard::Bitboard::<12, 19, 19, false>::from_gamestate(state), deadline),
        (13, 19, 19, false) => uct::search(&bitboard::Bitboard::<13, 19, 19, false>::from_gamestate(state), deadline),
        (14, 19, 19, false) => uct::search(&bitboard::Bitboard::<14, 19, 19, false>::from_gamestate(state), deadline),
        (15, 19, 19, false) => uct::search(&bitboard::Bitboard::<15, 19, 19, false>::from_gamestate(state), deadline),
        (16, 19, 19, false) => uct::search(&bitboard::Bitboard::<16, 19, 19, false>::from_gamestate(state), deadline),

        (1, 25, 25, true) => uct::search(&bitboard::Bitboard::<1, 25, 25, true>::from_gamestate(state), deadline),
        (2, 25, 25, true) => uct::search(&bitboard::Bitboard::<2, 25, 25, true>::from_gamestate(state), deadline),
        (3, 25, 25, true) => uct::search(&bitboard::Bitboard::<3, 25, 25, true>::from_gamestate(state), deadline),
        (4, 25, 25, true) => uct::search(&bitboard::Bitboard::<4, 25, 25, true>::from_gamestate(state), deadline),
        (5, 25, 25, true) => uct::search(&bitboard::Bitboard::<5, 25, 25, true>::from_gamestate(state), deadline),
        (6, 25, 25, true) => uct::search(&bitboard::Bitboard::<6, 25, 25, true>::from_gamestate(state), deadline),
        (7, 25, 25, true) => uct::search(&bitboard::Bitboard::<7, 25, 25, true>::from_gamestate(state), deadline),
        (8, 25, 25, true) => uct::search(&bitboard::Bitboard::<8, 25, 25, true>::from_gamestate(state), deadline),
        (9, 25, 25, true) => uct::search(&bitboard::Bitboard::<9, 25, 25, true>::from_gamestate(state), deadline),
        (10, 25, 25, true) => uct::search(&bitboard::Bitboard::<10, 25, 25, true>::from_gamestate(state), deadline),
        (11, 25, 25, true) => uct::search(&bitboard::Bitboard::<11, 25, 25, true>::from_gamestate(state), deadline),
        (12, 25, 25, true) => uct::search(&bitboard::Bitboard::<12, 25, 25, true>::from_gamestate(state), deadline),
        (13, 25, 25, true) => uct::search(&bitboard::Bitboard::<13, 25, 25, true>::from_gamestate(state), deadline),
        (14, 25, 25, true) => uct::search(&bitboard::Bitboard::<14, 25, 25, true>::from_gamestate(state), deadline),
        (15, 25, 25, true) => uct::search(&bitboard::Bitboard::<15, 25, 25, true>::from_gamestate(state), deadline),
        (16, 25, 25, true) => uct::search(&bitboard::Bitboard::<16, 25, 25, true>::from_gamestate(state), deadline),

        (1, 25, 25, false) => uct::search(&bitboard::Bitboard::<1, 25, 25, false>::from_gamestate(state), deadline),
        (2, 25, 25, false) => uct::search(&bitboard::Bitboard::<2, 25, 25, false>::from_gamestate(state), deadline),
        (3, 25, 25, false) => uct::search(&bitboard::Bitboard::<3, 25, 25, false>::from_gamestate(state), deadline),
        (4, 25, 25, false) => uct::search(&bitboard::Bitboard::<4, 25, 25, false>::from_gamestate(state), deadline),
        (5, 25, 25, false) => uct::search(&bitboard::Bitboard::<5, 25, 25, false>::from_gamestate(state), deadline),
        (6, 25, 25, false) => uct::search(&bitboard::Bitboard::<6, 25, 25, false>::from_gamestate(state), deadline),
        (7, 25, 25, false) => uct::search(&bitboard::Bitboard::<7, 25, 25, false>::from_gamestate(state), deadline),
        (8, 25, 25, false) => uct::search(&bitboard::Bitboard::<8, 25, 25, false>::from_gamestate(state), deadline),
        (9, 25, 25, false) => uct::search(&bitboard::Bitboard::<9, 25, 25, false>::from_gamestate(state), deadline),
        (10, 25, 25, false) => uct::search(&bitboard::Bitboard::<10, 25, 25, false>::from_gamestate(state), deadline),
        (11, 25, 25, false) => uct::search(&bitboard::Bitboard::<11, 25, 25, false>::from_gamestate(state), deadline),
        (12, 25, 25, false) => uct::search(&bitboard::Bitboard::<12, 25, 25, false>::from_gamestate(state), deadline),
        (13, 25, 25, false) => uct::search(&bitboard::Bitboard::<13, 25, 25, false>::from_gamestate(state), deadline),
        (14, 25, 25, false) => uct::search(&bitboard::Bitboard::<14, 25, 25, false>::from_gamestate(state), deadline),
        (15, 25, 25, false) => uct::search(&bitboard::Bitboard::<15, 25, 25, false>::from_gamestate(state), deadline),
        (16, 25, 25, false) => uct::search(&bitboard::Bitboard::<16, 25, 25, false>::from_gamestate(state), deadline),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}", state.board.snakes.len(), state.board.width, state.board.height),
    };
    Json(mv.to_json())
}

#[cfg(not(feature = "mcts"))]
pub async fn handle_move(Json(state): Json<GameState>) -> Json<Value> {
    // let state = req.into_inner();
    let deadline = time::Instant::now() + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(80) - 80)).into());
    let ruleset = match state.game.ruleset["name"].as_str() {
        Some("wrapped") => bitboard::Ruleset::Wrapped,
        Some("royale") => bitboard::Ruleset::Royale,
        Some("constrictor") => bitboard::Ruleset::Constrictor,
        _ => bitboard::Ruleset::Standard,
    };

    #[cfg(not(feature = "spl"))]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, matches!(ruleset, bitboard::Ruleset::Wrapped)) {
        (1, 11, 11, true) => task::spawn_blocking(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true) => task::spawn_blocking(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true) => task::spawn_blocking(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true) => task::spawn_blocking(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false) => task::spawn_blocking(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false) => task::spawn_blocking(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false) => task::spawn_blocking(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false) => task::spawn_blocking(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, please enable the 'spl' feature.", state.board.snakes.len(), state.board.width, state.board.height),
    };

    #[cfg(feature = "spl")]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, matches!(ruleset, bitboard::Ruleset::Wrapped)) {
        (1, 7, 7, true) => minimax::search(&bitboard::Bitboard::<1, 7, 7, true>::from_gamestate(state), deadline),
        (2, 7, 7, true) => minimax::search(&bitboard::Bitboard::<2, 7, 7, true>::from_gamestate(state), deadline),
        (3, 7, 7, true) => minimax::search(&bitboard::Bitboard::<3, 7, 7, true>::from_gamestate(state), deadline),
        (4, 7, 7, true) => minimax::search(&bitboard::Bitboard::<4, 7, 7, true>::from_gamestate(state), deadline),
        (5, 7, 7, true) => minimax::search(&bitboard::Bitboard::<5, 7, 7, true>::from_gamestate(state), deadline),
        (6, 7, 7, true) => minimax::search(&bitboard::Bitboard::<6, 7, 7, true>::from_gamestate(state), deadline),
        (7, 7, 7, true) => minimax::search(&bitboard::Bitboard::<7, 7, 7, true>::from_gamestate(state), deadline),
        (8, 7, 7, true) => minimax::search(&bitboard::Bitboard::<8, 7, 7, true>::from_gamestate(state), deadline),

        (1, 7, 7, false) => minimax::search(&bitboard::Bitboard::<1, 7, 7, false>::from_gamestate(state), deadline),
        (2, 7, 7, false) => minimax::search(&bitboard::Bitboard::<2, 7, 7, false>::from_gamestate(state), deadline),
        (3, 7, 7, false) => minimax::search(&bitboard::Bitboard::<3, 7, 7, false>::from_gamestate(state), deadline),
        (4, 7, 7, false) => minimax::search(&bitboard::Bitboard::<4, 7, 7, false>::from_gamestate(state), deadline),
        (5, 7, 7, false) => minimax::search(&bitboard::Bitboard::<5, 7, 7, false>::from_gamestate(state), deadline),
        (6, 7, 7, false) => minimax::search(&bitboard::Bitboard::<6, 7, 7, false>::from_gamestate(state), deadline),
        (7, 7, 7, false) => minimax::search(&bitboard::Bitboard::<7, 7, 7, false>::from_gamestate(state), deadline),
        (8, 7, 7, false) => minimax::search(&bitboard::Bitboard::<8, 7, 7, false>::from_gamestate(state), deadline),

        (1, 11, 11, true) => minimax::search(&bitboard::Bitboard::<1, 11, 11, true>::from_gamestate(state), deadline),
        (2, 11, 11, true) => minimax::search(&bitboard::Bitboard::<2, 11, 11, true>::from_gamestate(state), deadline),
        (3, 11, 11, true) => minimax::search(&bitboard::Bitboard::<3, 11, 11, true>::from_gamestate(state), deadline),
        (4, 11, 11, true) => minimax::search(&bitboard::Bitboard::<4, 11, 11, true>::from_gamestate(state), deadline),
        (5, 11, 11, true) => minimax::search(&bitboard::Bitboard::<5, 11, 11, true>::from_gamestate(state), deadline),
        (6, 11, 11, true) => minimax::search(&bitboard::Bitboard::<6, 11, 11, true>::from_gamestate(state), deadline),
        (7, 11, 11, true) => minimax::search(&bitboard::Bitboard::<7, 11, 11, true>::from_gamestate(state), deadline),
        (8, 11, 11, true) => minimax::search(&bitboard::Bitboard::<8, 11, 11, true>::from_gamestate(state), deadline),

        (1, 11, 11, false) => minimax::search(&bitboard::Bitboard::<1, 11, 11, false>::from_gamestate(state), deadline),
        (2, 11, 11, false) => minimax::search(&bitboard::Bitboard::<2, 11, 11, false>::from_gamestate(state), deadline),
        (3, 11, 11, false) => minimax::search(&bitboard::Bitboard::<3, 11, 11, false>::from_gamestate(state), deadline),
        (4, 11, 11, false) => minimax::search(&bitboard::Bitboard::<4, 11, 11, false>::from_gamestate(state), deadline),
        (5, 11, 11, false) => minimax::search(&bitboard::Bitboard::<5, 11, 11, false>::from_gamestate(state), deadline),
        (6, 11, 11, false) => minimax::search(&bitboard::Bitboard::<6, 11, 11, false>::from_gamestate(state), deadline),
        (7, 11, 11, false) => minimax::search(&bitboard::Bitboard::<7, 11, 11, false>::from_gamestate(state), deadline),
        (8, 11, 11, false) => minimax::search(&bitboard::Bitboard::<8, 11, 11, false>::from_gamestate(state), deadline),

        (1, 19, 19, true) => minimax::search(&bitboard::Bitboard::<1, 19, 19, true>::from_gamestate(state), deadline),
        (2, 19, 19, true) => minimax::search(&bitboard::Bitboard::<2, 19, 19, true>::from_gamestate(state), deadline),
        (3, 19, 19, true) => minimax::search(&bitboard::Bitboard::<3, 19, 19, true>::from_gamestate(state), deadline),
        (4, 19, 19, true) => minimax::search(&bitboard::Bitboard::<4, 19, 19, true>::from_gamestate(state), deadline),
        (5, 19, 19, true) => minimax::search(&bitboard::Bitboard::<5, 19, 19, true>::from_gamestate(state), deadline),
        (6, 19, 19, true) => minimax::search(&bitboard::Bitboard::<6, 19, 19, true>::from_gamestate(state), deadline),
        (7, 19, 19, true) => minimax::search(&bitboard::Bitboard::<7, 19, 19, true>::from_gamestate(state), deadline),
        (8, 19, 19, true) => minimax::search(&bitboard::Bitboard::<8, 19, 19, true>::from_gamestate(state), deadline),
        (9, 19, 19, true) => minimax::search(&bitboard::Bitboard::<9, 19, 19, true>::from_gamestate(state), deadline),
        (10, 19, 19, true) => minimax::search(&bitboard::Bitboard::<10, 19, 19, true>::from_gamestate(state), deadline),
        (11, 19, 19, true) => minimax::search(&bitboard::Bitboard::<11, 19, 19, true>::from_gamestate(state), deadline),
        (12, 19, 19, true) => minimax::search(&bitboard::Bitboard::<12, 19, 19, true>::from_gamestate(state), deadline),
        (13, 19, 19, true) => minimax::search(&bitboard::Bitboard::<13, 19, 19, true>::from_gamestate(state), deadline),
        (14, 19, 19, true) => minimax::search(&bitboard::Bitboard::<14, 19, 19, true>::from_gamestate(state), deadline),
        (15, 19, 19, true) => minimax::search(&bitboard::Bitboard::<15, 19, 19, true>::from_gamestate(state), deadline),
        (16, 19, 19, true) => minimax::search(&bitboard::Bitboard::<16, 19, 19, true>::from_gamestate(state), deadline),

        (1, 19, 19, false) => minimax::search(&bitboard::Bitboard::<1, 19, 19, false>::from_gamestate(state), deadline),
        (2, 19, 19, false) => minimax::search(&bitboard::Bitboard::<2, 19, 19, false>::from_gamestate(state), deadline),
        (3, 19, 19, false) => minimax::search(&bitboard::Bitboard::<3, 19, 19, false>::from_gamestate(state), deadline),
        (4, 19, 19, false) => minimax::search(&bitboard::Bitboard::<4, 19, 19, false>::from_gamestate(state), deadline),
        (5, 19, 19, false) => minimax::search(&bitboard::Bitboard::<5, 19, 19, false>::from_gamestate(state), deadline),
        (6, 19, 19, false) => minimax::search(&bitboard::Bitboard::<6, 19, 19, false>::from_gamestate(state), deadline),
        (7, 19, 19, false) => minimax::search(&bitboard::Bitboard::<7, 19, 19, false>::from_gamestate(state), deadline),
        (8, 19, 19, false) => minimax::search(&bitboard::Bitboard::<8, 19, 19, false>::from_gamestate(state), deadline),
        (9, 19, 19, false) => minimax::search(&bitboard::Bitboard::<9, 19, 19, false>::from_gamestate(state), deadline),
        (10, 19, 19, false) => minimax::search(&bitboard::Bitboard::<10, 19, 19, false>::from_gamestate(state), deadline),
        (11, 19, 19, false) => minimax::search(&bitboard::Bitboard::<11, 19, 19, false>::from_gamestate(state), deadline),
        (12, 19, 19, false) => minimax::search(&bitboard::Bitboard::<12, 19, 19, false>::from_gamestate(state), deadline),
        (13, 19, 19, false) => minimax::search(&bitboard::Bitboard::<13, 19, 19, false>::from_gamestate(state), deadline),
        (14, 19, 19, false) => minimax::search(&bitboard::Bitboard::<14, 19, 19, false>::from_gamestate(state), deadline),
        (15, 19, 19, false) => minimax::search(&bitboard::Bitboard::<15, 19, 19, false>::from_gamestate(state), deadline),
        (16, 19, 19, false) => minimax::search(&bitboard::Bitboard::<16, 19, 19, false>::from_gamestate(state), deadline),

        (1, 25, 25, true) => minimax::search(&bitboard::Bitboard::<1, 25, 25, true>::from_gamestate(state), deadline),
        (2, 25, 25, true) => minimax::search(&bitboard::Bitboard::<2, 25, 25, true>::from_gamestate(state), deadline),
        (3, 25, 25, true) => minimax::search(&bitboard::Bitboard::<3, 25, 25, true>::from_gamestate(state), deadline),
        (4, 25, 25, true) => minimax::search(&bitboard::Bitboard::<4, 25, 25, true>::from_gamestate(state), deadline),
        (5, 25, 25, true) => minimax::search(&bitboard::Bitboard::<5, 25, 25, true>::from_gamestate(state), deadline),
        (6, 25, 25, true) => minimax::search(&bitboard::Bitboard::<6, 25, 25, true>::from_gamestate(state), deadline),
        (7, 25, 25, true) => minimax::search(&bitboard::Bitboard::<7, 25, 25, true>::from_gamestate(state), deadline),
        (8, 25, 25, true) => minimax::search(&bitboard::Bitboard::<8, 25, 25, true>::from_gamestate(state), deadline),
        (9, 25, 25, true) => minimax::search(&bitboard::Bitboard::<9, 25, 25, true>::from_gamestate(state), deadline),
        (10, 25, 25, true) => minimax::search(&bitboard::Bitboard::<10, 25, 25, true>::from_gamestate(state), deadline),
        (11, 25, 25, true) => minimax::search(&bitboard::Bitboard::<11, 25, 25, true>::from_gamestate(state), deadline),
        (12, 25, 25, true) => minimax::search(&bitboard::Bitboard::<12, 25, 25, true>::from_gamestate(state), deadline),
        (13, 25, 25, true) => minimax::search(&bitboard::Bitboard::<13, 25, 25, true>::from_gamestate(state), deadline),
        (14, 25, 25, true) => minimax::search(&bitboard::Bitboard::<14, 25, 25, true>::from_gamestate(state), deadline),
        (15, 25, 25, true) => minimax::search(&bitboard::Bitboard::<15, 25, 25, true>::from_gamestate(state), deadline),
        (16, 25, 25, true) => minimax::search(&bitboard::Bitboard::<16, 25, 25, true>::from_gamestate(state), deadline),

        (1, 25, 25, false) => minimax::search(&bitboard::Bitboard::<1, 25, 25, false>::from_gamestate(state), deadline),
        (2, 25, 25, false) => minimax::search(&bitboard::Bitboard::<2, 25, 25, false>::from_gamestate(state), deadline),
        (3, 25, 25, false) => minimax::search(&bitboard::Bitboard::<3, 25, 25, false>::from_gamestate(state), deadline),
        (4, 25, 25, false) => minimax::search(&bitboard::Bitboard::<4, 25, 25, false>::from_gamestate(state), deadline),
        (5, 25, 25, false) => minimax::search(&bitboard::Bitboard::<5, 25, 25, false>::from_gamestate(state), deadline),
        (6, 25, 25, false) => minimax::search(&bitboard::Bitboard::<6, 25, 25, false>::from_gamestate(state), deadline),
        (7, 25, 25, false) => minimax::search(&bitboard::Bitboard::<7, 25, 25, false>::from_gamestate(state), deadline),
        (8, 25, 25, false) => minimax::search(&bitboard::Bitboard::<8, 25, 25, false>::from_gamestate(state), deadline),
        (9, 25, 25, false) => minimax::search(&bitboard::Bitboard::<9, 25, 25, false>::from_gamestate(state), deadline),
        (10, 25, 25, false) => minimax::search(&bitboard::Bitboard::<10, 25, 25, false>::from_gamestate(state), deadline),
        (11, 25, 25, false) => minimax::search(&bitboard::Bitboard::<11, 25, 25, false>::from_gamestate(state), deadline),
        (12, 25, 25, false) => minimax::search(&bitboard::Bitboard::<12, 25, 25, false>::from_gamestate(state), deadline),
        (13, 25, 25, false) => minimax::search(&bitboard::Bitboard::<13, 25, 25, false>::from_gamestate(state), deadline),
        (14, 25, 25, false) => minimax::search(&bitboard::Bitboard::<14, 25, 25, false>::from_gamestate(state), deadline),
        (15, 25, 25, false) => minimax::search(&bitboard::Bitboard::<15, 25, 25, false>::from_gamestate(state), deadline),
        (16, 25, 25, false) => minimax::search(&bitboard::Bitboard::<16, 25, 25, false>::from_gamestate(state), deadline),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}", state.board.snakes.len(), state.board.width, state.board.height),
    };
    Json(mv.to_json())
}
