use rocket::http::Status;
use rocket_contrib::json::{Json, JsonValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::types;
use crate::mailbox;
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

fn coord_idx(c: &Coord, w: &usize) -> usize {
    c.x + c.y * w
}

fn create_mailbox_board(state: GameState) -> mailbox::MailBoxBoard {
    let b = state.board;
    let mut ret = mailbox::MailBoxBoard::new(&b.width, &b.height);
    for snake in b.snakes {
        *ret.get(coord_idx(&snake.head, &b.width)) |= types::HEAD;
        *ret.get(coord_idx(&snake.body[snake.length-1], &b.width)) |= types::TAIL;
        let mut bod = Vec::new();
        for (i, body) in snake.body.iter().enumerate() {
            let x = coord_idx(body, &b.width);
            if i > 0 && i < snake.length - 1 {
                *ret.get(x) |= types::BODY;
            }
            if !bod.contains(&x) {
                bod.push(x);
            }
        }
        ret.snakes.push(mailbox::Snake{
            length: snake.length.try_into().unwrap(),
            health: snake.health,
            body: bod,
            is_enemy: snake.id != state.you.id,
        })
    }
    for hazard in b.hazards {
        *ret.get(coord_idx(&hazard, &b.width)) |= types::HAZARD;
    }
    for food in b.food {
        *ret.get(coord_idx(&food, &b.width)) |= types::FOOD;
    }
    ret
}

#[get("/")]
pub fn handle_index() -> JsonValue {
    json!({
        "apiversion": "1",
        "author": "JonathanArns",
        "color": "#B7410E",
        "head": "villain",
        "tail": "mystic-moon",
    })
}

#[post("/start", format = "json", data = "<_req>")]
pub fn handle_start(_req: Json<GameState>) -> Status {
    Status::Ok
}

#[post("/move", format = "json", data = "<req>")]
pub fn handle_move(req: Json<GameState>) -> JsonValue {
    let state = req.into_inner();
    let mut game = types::Game{move_time: std::time::Duration::from_millis(state.game.timeout.into())};
    match (state.board.snakes.len(), state.board.width, state.board.height) {
        (1, 7, 7) => minimax::search(&bitboard::Bitboard::<1, 7, 7>::from_gamestate(state), &mut game).0.to_json(),
        (2, 7, 7) => minimax::search(&bitboard::Bitboard::<2, 7, 7>::from_gamestate(state), &mut game).0.to_json(),
        (3, 7, 7) => minimax::search(&bitboard::Bitboard::<3, 7, 7>::from_gamestate(state), &mut game).0.to_json(),
        (4, 7, 7) => minimax::search(&bitboard::Bitboard::<4, 7, 7>::from_gamestate(state), &mut game).0.to_json(),

        (1, 11, 11) => minimax::search(&bitboard::Bitboard::<1, 11, 11>::from_gamestate(state), &mut game).0.to_json(),
        (2, 11, 11) => minimax::search(&bitboard::Bitboard::<2, 11, 11>::from_gamestate(state), &mut game).0.to_json(),
        (3, 11, 11) => minimax::search(&bitboard::Bitboard::<3, 11, 11>::from_gamestate(state), &mut game).0.to_json(),
        (4, 11, 11) => minimax::search(&bitboard::Bitboard::<4, 11, 11>::from_gamestate(state), &mut game).0.to_json(),
        (5, 11, 11) => minimax::search(&bitboard::Bitboard::<5, 11, 11>::from_gamestate(state), &mut game).0.to_json(),
        (6, 11, 11) => minimax::search(&bitboard::Bitboard::<6, 11, 11>::from_gamestate(state), &mut game).0.to_json(),
        (7, 11, 11) => minimax::search(&bitboard::Bitboard::<7, 11, 11>::from_gamestate(state), &mut game).0.to_json(),
        (8, 11, 11) => minimax::search(&bitboard::Bitboard::<8, 11, 11>::from_gamestate(state), &mut game).0.to_json(),

        (1, 19, 19) => minimax::search(&bitboard::Bitboard::<1, 19, 19>::from_gamestate(state), &mut game).0.to_json(),
        (2, 19, 19) => minimax::search(&bitboard::Bitboard::<2, 19, 19>::from_gamestate(state), &mut game).0.to_json(),
        (3, 19, 19) => minimax::search(&bitboard::Bitboard::<3, 19, 19>::from_gamestate(state), &mut game).0.to_json(),
        (4, 19, 19) => minimax::search(&bitboard::Bitboard::<4, 19, 19>::from_gamestate(state), &mut game).0.to_json(),
        (5, 19, 19) => minimax::search(&bitboard::Bitboard::<5, 19, 19>::from_gamestate(state), &mut game).0.to_json(),
        (6, 19, 19) => minimax::search(&bitboard::Bitboard::<6, 19, 19>::from_gamestate(state), &mut game).0.to_json(),
        (7, 19, 19) => minimax::search(&bitboard::Bitboard::<7, 19, 19>::from_gamestate(state), &mut game).0.to_json(),
        (8, 19, 19) => minimax::search(&bitboard::Bitboard::<8, 19, 19>::from_gamestate(state), &mut game).0.to_json(),

        (1, 25, 25) => minimax::search(&bitboard::Bitboard::<1, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (2, 25, 25) => minimax::search(&bitboard::Bitboard::<2, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (3, 25, 25) => minimax::search(&bitboard::Bitboard::<3, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (4, 25, 25) => minimax::search(&bitboard::Bitboard::<4, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (5, 25, 25) => minimax::search(&bitboard::Bitboard::<5, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (6, 25, 25) => minimax::search(&bitboard::Bitboard::<6, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (7, 25, 25) => minimax::search(&bitboard::Bitboard::<7, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (8, 25, 25) => minimax::search(&bitboard::Bitboard::<8, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (9, 25, 25) => minimax::search(&bitboard::Bitboard::<9, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (10, 25, 25) => minimax::search(&bitboard::Bitboard::<10, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (11, 25, 25) => minimax::search(&bitboard::Bitboard::<11, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (12, 25, 25) => minimax::search(&bitboard::Bitboard::<12, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (13, 25, 25) => minimax::search(&bitboard::Bitboard::<13, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (14, 25, 25) => minimax::search(&bitboard::Bitboard::<14, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (15, 25, 25) => minimax::search(&bitboard::Bitboard::<15, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        (16, 25, 25) => minimax::search(&bitboard::Bitboard::<16, 25, 25>::from_gamestate(state), &mut game).0.to_json(),
        _ => panic!("Snake count or board size not supported"),
    }
}

#[post("/end", format = "json", data = "<_req>")]
pub fn handle_end(_req: Json<GameState>) -> Status {
    Status::Ok
}
