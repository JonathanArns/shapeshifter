use rocket::http::Status;
use rocket_contrib::json::{Json, JsonValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::types;
use crate::mailbox;
use crate::minimax;
use crate::bitboard;

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
        "head": "dead",
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
    // let board = create_mailbox_board(state);
    // let (mv, _, _) = minimax::iterative_deepening_search(board, &mut game);
    // mv.to_json()
    match state.board.snakes.len() {
        1 => bitboard::Bitboard::<1>::from_gamestate(state).iterative_deepening_search(&mut game).0.to_json(),
        2 => bitboard::Bitboard::<2>::from_gamestate(state).iterative_deepening_search(&mut game).0.to_json(),
        3 => bitboard::Bitboard::<3>::from_gamestate(state).iterative_deepening_search(&mut game).0.to_json(),
        4 => bitboard::Bitboard::<4>::from_gamestate(state).iterative_deepening_search(&mut game).0.to_json(),
        5 => bitboard::Bitboard::<5>::from_gamestate(state).iterative_deepening_search(&mut game).0.to_json(),
        6 => bitboard::Bitboard::<6>::from_gamestate(state).iterative_deepening_search(&mut game).0.to_json(),
        7 => bitboard::Bitboard::<7>::from_gamestate(state).iterative_deepening_search(&mut game).0.to_json(),
        8 => bitboard::Bitboard::<8>::from_gamestate(state).iterative_deepening_search(&mut game).0.to_json(),
        _ => panic!("Snake count not supported"),
    }
}

#[post("/end", format = "json", data = "<_req>")]
pub fn handle_end(_req: Json<GameState>) -> Status {
    Status::Ok
}
