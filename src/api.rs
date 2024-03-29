use serde_json::{Value, json};
use axum::extract::{Json, TypedHeader};
use axum::headers::{Header, HeaderName, HeaderValue};
use tokio::task;
use tracing::info;
use std::collections::HashMap;
use std::time;

use crate::bitboard;
use crate::wire_rep::GameState;
use crate::minimax;
use crate::uct;

pub struct StartTimeHeader(u64);

impl Header for StartTimeHeader {
    fn name() -> &'static HeaderName {
        static START_TIME_HEADER: HeaderName = HeaderName::from_static("x-received-at");
        &START_TIME_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(axum::headers::Error::invalid)?;

        let x = if let Ok(val) = value.to_str() {
            val.replace(".", "")
        } else {
            return Err(axum::headers::Error::invalid())
        };

        if let Ok(val) = x.parse::<u64>() {
            Ok(StartTimeHeader(val))
        } else {
            Err(axum::headers::Error::invalid())
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = HeaderValue::from_str(&self.0.to_string()).unwrap();
        values.extend(std::iter::once(value));
    }
}

pub async fn handle_index() -> Json<Value> {
    Json(json!({
        "apiversion": "1",
        "author": "JonathanArns",
        "color": "#900050",
        "head": "cosmic-horror-special",
        "tail": "cosmic-horror",
    }))
}

pub async fn handle_index_silly() -> Json<Value> {
    Json(json!({
        "apiversion": "1",
        "author": "JonathanArns",
        "color": "#ccffcc",
        "head": "silly",
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

fn spawn_blocking_with_tracing<F, R>(f: F) -> task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    task::spawn_blocking(move || current_span.in_scope(f))
}

fn is_wrapped(state: &GameState) -> bool {
    if let Some(name) = state.game.ruleset["name"].as_str() && name.contains("wrapped") {
        true
    } else {
        false
    }
}

fn is_hazard_stacking(state: &GameState) -> bool {
    match state.game.ruleset["name"].as_str() {
        Some("sinkholes") => true,
        _ => {
            match state.game.map.as_str() {
                "snail_mode" => true,
                x => {
                    let mut hazards = vec![false; state.board.width*state.board.height];
                    for hz in &state.board.hazards {
                        if hazards[hz.x+state.board.width*hz.y] {
                            return true
                        }
                        hazards[hz.x+state.board.width*hz.y] = true;
                    }
                    false
                },
            }
        },
    }
}

#[tracing::instrument(
    name = "handle_move",
    skip(state, start_time_header),
    fields(
        game.source = state.game.source.as_str(),
        game.id = state.game.id.as_str(),
        game.turn = state.turn,
        search.algo = "mcts"
    )
)]
pub async fn handle_move_mcts(start_time_header: Option<TypedHeader<StartTimeHeader>>, Json(state): Json<GameState>) -> Json<Value> {
    let deadline = if let Some(TypedHeader(StartTimeHeader(value))) = start_time_header {
        time::UNIX_EPOCH + time::Duration::from_millis(value) + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(100) - 100)).into())
    } else {
        time::SystemTime::now() + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(100) - 100)).into())
    };

    #[cfg(not(feature = "spl"))]
    let (mv, _score) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)) {
        (1, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, please enable the 'spl' feature.", state.board.snakes.len(), state.board.width, state.board.height),
    };

    #[cfg(feature = "spl")]
    let (mv, _score) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)) {
        (1, 7, 7, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 7, 7, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<9, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<10, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<11, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<12, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<13, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<14, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<15, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<16, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<9, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<10, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<11, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<12, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<13, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<14, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<15, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<16, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        // maze_arcade
        (1, 19, 21, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 19, 21, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 21, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 19, 21, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 21, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 19, 21, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 21, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 19, 21, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<9, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<10, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<11, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<12, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<13, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<14, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<15, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, true, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<16, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<1, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<2, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<3, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<4, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<5, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<6, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<7, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<8, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<9, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<10, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<11, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<12, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<13, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<14, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<15, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, false, false) => spawn_blocking_with_tracing(move || uct::search(&bitboard::Bitboard::<16, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}", state.board.snakes.len(), state.board.width, state.board.height),
    };
    Json(mv.to_json())
}

#[tracing::instrument(
    name = "handle_move",
    skip(state, start_time_header),
    fields(
        game.source = state.game.source.as_str(),
        game.id = state.game.id.as_str(),
        game.turn = state.turn,
        search.algo = "minimax"
    )
)]
pub async fn handle_move_minimax(start_time_header: Option<TypedHeader<StartTimeHeader>>, Json(mut state): Json<GameState>) -> Json<Value> {
    let deadline = if let Some(TypedHeader(StartTimeHeader(value))) = start_time_header {
        // we are playing behind a proxy with "accurate" timing information
        time::UNIX_EPOCH + time::Duration::from_millis(value) + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(60) - 60)).into())
    } else {
        time::SystemTime::now() + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(100) - 100)).into())
    };

    #[cfg(not(feature = "spl"))]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)) {
        (1, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, WRAP: {:?}, HZSTACK: {:?}, please enable the 'spl' feature.", state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)),
    };

    #[cfg(feature = "spl")]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)) {
        (1, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 7, 7, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 7, 7, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        // stacking hazards
        (1, 11, 11, true, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true, true, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false, true, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false, true, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 11, 11, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 11, 11, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 19, 19, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 19, 19, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        // maze_arcade
        (1, 19, 21, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 21, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 21, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 21, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 21, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 21, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 21, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 21, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 25, 25, true, false, 0>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 25, 25, false, false, 0>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, WRAP: {:?}, HZSTACK: {:?}", state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)),
    };
    Json(mv.to_json())
}

#[tracing::instrument(
    name = "handle_move",
    skip(state, start_time_header),
    fields(
        game.source = state.game.source.as_str(),
        game.id = state.game.id.as_str(),
        game.turn = state.turn,
        search.algo = "silly"
    )
)]
pub async fn handle_move_silly(start_time_header: Option<TypedHeader<StartTimeHeader>>, Json(mut state): Json<GameState>) -> Json<Value> {
    let deadline = if let Some(TypedHeader(StartTimeHeader(value))) = start_time_header {
        // we are playing behind a proxy with "accurate" timing information
        time::UNIX_EPOCH + time::Duration::from_millis(value) + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(60) - 60)).into())
    } else {
        time::SystemTime::now() + time::Duration::from_millis(((state.game.timeout / 2).max(state.game.timeout.max(100) - 100)).into())
    };

    #[cfg(not(feature = "spl"))]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)) {
        (1, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, WRAP: {:?}, HZSTACK: {:?}, please enable the 'spl' feature.", state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)),
    };

    #[cfg(feature = "spl")]
    let (mv, _score, _depth) = match (state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)) {
        (1, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 7, 7, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 7, 7, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 7, 7, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 7, 7, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 7, 7, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 7, 7, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 7, 7, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 7, 7, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 7, 7, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 7, 7, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 7, 7, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 7, 7, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 7, 7, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 7, 7, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 7, 7, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 7, 7, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 7, 7, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        // stacking hazards
        (1, 11, 11, true, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true, true, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true, true, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true, true, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true, true, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false, true, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false, true, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false, true, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, true) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false, true, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 11, 11, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 11, 11, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 11, 11, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 19, 19, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 19, 19, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 19, 19, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        // maze_arcade
        (1, 19, 21, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 19, 21, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 19, 21, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 19, 21, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 19, 21, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 19, 21, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 19, 21, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 19, 21, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, true, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 25, 25, true, false, 1>::from_gamestate(state), deadline)).await.unwrap(),

        (1, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<1, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (2, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<2, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (3, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<3, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (4, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<4, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (5, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<5, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (6, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<6, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (7, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<7, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (8, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<8, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (9, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<9, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (10, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<10, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (11, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<11, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (12, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<12, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (13, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<13, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (14, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<14, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (15, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<15, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        (16, 25, 25, false, false) => spawn_blocking_with_tracing(move || minimax::search(&bitboard::Bitboard::<16, 25, 25, false, false, 1>::from_gamestate(state), deadline)).await.unwrap(),
        _ => panic!("Snake count or board size not supported S: {}, W: {}, H: {}, WRAP: {:?}, HZSTACK: {:?}", state.board.snakes.len(), state.board.width, state.board.height, is_wrapped(&state), is_hazard_stacking(&state)),
    };
    Json(mv.to_json())
}

/// Use the type parameter TT to manually override the tt_id of the created Bitboard in training mode.
/// This is used in training, since the tt_id is also used to choose eval weights there.
#[cfg(feature = "training")]
pub async fn training_handle_move_minimax<const TT: u8>(Json(mut state): Json<GameState>, start_time_header: Option<TypedHeader<StartTimeHeader>>) -> Json<Value> {
    state.game.id = "".to_string();
    state.you.id = TT.to_string();
    handle_move_minimax(Json(state), start_time_header).await
}
