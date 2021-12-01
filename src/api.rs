use rocket::http::Status;
use rocket_contrib::json::{Json, JsonValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::types;
use crate::minimax;

#[derive(Deserialize, Serialize, Debug)]
pub struct Game {
    id: String,
    ruleset: HashMap<String, Value>,
    timeout: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Board {
    height: usize,
    width: usize,
    food: Vec<Coord>,
    snakes: Vec<Battlesnake>,
    hazards: Vec<Coord>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Battlesnake {
    id: String,
    name: String,
    health: u8,
    body: Vec<Coord>,
    head: Coord,
    length: usize,
    latency: String,

    // Used in non-standard game modes
    shout: Option<String>,
    squad: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Coord {
    x: usize,
    y: usize,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GameState {
    game: Game,
    turn: u32,
    board: Board,
    you: Battlesnake,
}

fn coord_idx(c: &Coord, w: &usize) -> usize {
    c.x + c.y * w
}

fn convert_board(state: GameState) -> types::Board {
    let b = state.board;
    let mut ret = types::Board::new(&b.width, &b.height);
    for snake in b.snakes {
        *ret.get(coord_idx(&snake.head, &b.width)) |= types::HEAD;
        *ret.get(coord_idx(&snake.body[snake.length-1], &b.width)) |= types::TAIL;
        let mut bod = Vec::new();
        for (i, body) in snake.body.iter().enumerate() {
            if i > 0 && i < snake.length - 1 {
                *ret.get(coord_idx(body, &b.width)) |= types::BODY;
            }
            bod.push(coord_idx(body, &b.width));
        }
        ret.snakes.push(types::Snake{
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
        "author": "",
        "color": "#888888",
        "head": "default",
        "tail": "default",
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
    let board = convert_board(state);
    let (mv, _, _) = minimax::iterative_deepening_search(&board, &mut game);
    match mv {
        types::Move::Up => json!({ "move": "up" }),
        types::Move::Down => json!({ "move": "down" }),
        types::Move::Left => json!({ "move": "left" }),
        types::Move::Right => json!({ "move": "right" }),
    }
}

#[post("/end", format = "json", data = "<req>")]
pub fn handle_end(req: Json<GameState>) -> Status {
    Status::Ok
}
