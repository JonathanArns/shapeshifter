use crate::types::*;
use crate::bitboard::*;

use arrayvec::ArrayVec;

/// Can never return a move that moves out of bounds on the board,
/// because that would cause a panic elsewhere.
pub fn allowed_moves<const N:usize>(board: &Bitboard<N>, pos: u8) -> ArrayVec<Move, 4> {
    let mut moves = ArrayVec::<Move, 4>::new();
    let mut some_legal_move = Move::Up;
    let mut tails = [u8::MAX; N];
    for i in 0..N {
        if board.snakes[i].is_alive() && board.snakes[i].curled_bodyparts == 0 {
            tails[i] = board.snakes[i].tail;
        }
    }
    let mask = 1<<pos;
    if pos > 10 {
        some_legal_move = Move::Down;
        if board.bodies[0] & mask >> 11 == 0 || tails.contains(&(pos - 11)) {
            moves.push(Move::Down);
        }
    }
    if pos < 110 {
        some_legal_move = Move::Up;
        if board.bodies[0] & mask << 11 == 0 || tails.contains(&(pos + 11)) {
            moves.push(Move::Up);
        }
    }
    if pos % 11 > 0 {
        some_legal_move = Move::Left;
        if board.bodies[0] & mask >> 1 == 0 || tails.contains(&(pos - 1)) {
            moves.push(Move::Left);
        }
    }
    if pos % 11 < 10 {
        some_legal_move = Move::Right;
        if board.bodies[0] & mask << 1 == 0 || tails.contains(&(pos + 1)) {
            moves.push(Move::Right);
        }
    }
    if moves.len() == 0 {
        moves.push(some_legal_move);
    }
    moves
}

/// Generates all possible move combinations from a position.
/// Can skip the first n snakes, their moves will always be Up in the result.
pub fn move_combinations<const N: usize>(board: &Bitboard<N>, skip: usize) -> Vec<[Move; N]> {
    // get moves for each enemy
    let mut moves_per_snake = ArrayVec::<ArrayVec<Move, 4>, N>::new();
    for snake in board.snakes[0+skip..].iter() {
        if snake.is_alive() {
            moves_per_snake.push(allowed_moves(board, snake.head));
        } else {
            let mut none_move = ArrayVec::<_, 4>::new();
            none_move.insert(0, Move::Up);
            moves_per_snake.push(none_move);
        }
    }

    // kartesian product of the possible moves to get the possible combinations
    let mut moves: Vec<[Move; N]> = Vec::with_capacity(1 + N.pow(N as u32));
    moves.push([Move::Up; N]);
    let mut moves_start;
    let mut moves_end = 0;
    for (i, snake_moves) in moves_per_snake.iter().enumerate() {
        moves_start = moves_end;
        moves_end = moves.len();
        for mv in snake_moves.iter() {
            for j in moves_start..moves_end {
                let mut tmp = moves[j];
                tmp[i+skip] = *mv;
                moves.push(tmp);
            }
        }
    }
    moves.drain(0..moves_end);
    moves
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
        let state = api::GameState{
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
            move_combinations(&board, 0)
        });
    }
}
