use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use axum::extract::Json;
use tokio::task;
use tracing::info;
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
    pub map: String,
    pub timeout: u32,
    pub source: String,
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
        "head": "cosmic-horror",
        "tail": "cosmic-horror",
    }))
}

pub async fn handle_start(Json(_req): Json<GameState>) {}

#[tracing::instrument(
    name = "handle_end",
    skip(state),
    fields(
        game.source = state.game.source.as_str(),
        game.id = state.game.id.as_str()
    )
)]
pub async fn handle_end(Json(state): Json<GameState>) {
    let mut win = false;
    for snake in state.board.snakes {
        if snake.health > 0 {
            info!(game.winner.name = snake.name.as_str(), game.winner.id = snake.id.as_str(), game.source = state.game.source.as_str(), game.id = state.game.id.as_str(), "game_winner");
            if snake.id == state.you.id {
                info!(game.result = "win", game.source = state.game.source.as_str(), game.id = state.game.id.as_str(), "game_result");
                return
            }
        }
    }
    info!(game.result = "loss", game.source = state.game.source.as_str(), game.id = state.game.id.as_str(), "game_result");
}

fn is_wrapped(state: &GameState) -> bool {
    match state.game.ruleset["name"].as_str() {
        Some("wrapped") => true,
        _ => false,
    }
}

fn spawn_blocking_with_tracing<F, R>(f: F) -> task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    task::spawn_blocking(move || current_span.in_scope(f))
}


#[cfg(feature = "mcts")]
#[tracing::instrument(
    name = "handle_move",
    skip(state),
    fields(
        game.source = state.game.source.as_str(),
        game.id = state.game.id.as_str(),
        game.turn = state.turn,
        search.algo = "mcts"
    )
)]
pub async fn handle_move(Json(state): Json<GameState>) -> Json<Value> {
    let deadline = time::Instant::now() + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(80) - 80)).into());

    #[cfg(not(feature = "spl"))]
    let (mv, _score) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state)) {
        (1, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),

        // maze_arcade
        (1, 19, 21, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 21, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 21, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 21, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, please enable the 'spl' feature.", state.board.snakes.len(), state.board.width, state.board.height),
    };

    #[cfg(feature = "spl")]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state)) {
        (1, 7, 7, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 7, 7, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<9, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<10, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<11, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<12, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<13, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<14, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<15, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<16, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<9, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<10, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<11, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<12, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<13, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<14, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<15, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<16, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),

        // maze_arcade
        (1, 19, 21, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 21, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 21, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 21, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<9, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<10, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<11, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<12, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<13, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<14, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<15, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, true) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<16, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<9, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<10, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<11, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<12, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<13, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<14, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<15, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<16, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}", state.board.snakes.len(), state.board.width, state.board.height),
    };
    Json(mv.to_json())
}

/// Use the type parameter TT to manually override the tt_id of the created Bitboard in training mode.
/// This is used in training, since the tt_id is also used to choose eval weights there.
#[cfg(not(feature = "mcts"))]
#[tracing::instrument(
    name = "handle_move",
    skip(state),
    fields(
        game.source = state.game.source.as_str(),
        game.id = state.game.id.as_str(),
        game.turn = state.turn,
        search.algo = "minimax"
    )
)]
pub async fn handle_move<const TT: u8>(Json(mut state): Json<GameState>) -> Json<Value> {
    let deadline = time::Instant::now() + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(80) - 80)).into());

    #[cfg(feature = "training")]
    {
        state.game.id = "".to_string();
        state.you.id = TT.to_string();
    }

    #[cfg(not(feature = "spl"))]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state)) {
        (1, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),

        // maze_arcade
        (1, 19, 21, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 21, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 21, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 21, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, please enable the 'spl' feature.", state.board.snakes.len(), state.board.width, state.board.height),
    };

    #[cfg(feature = "spl")]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state)) {
        (1, 7, 7, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 7, 7, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 7, 7, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 7, 7, false>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 11, 11, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 11, 11, false>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 19, 19, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 19, 19, false>::from_gamestate(state), deadline)).await.unwrap(),

        // maze_arcade
        (1, 19, 21, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 21, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 21, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 21, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 21, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 25, 25, true>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 25, 25, false>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}", state.board.snakes.len(), state.board.width, state.board.height),
    };
    Json(mv.to_json())
}
