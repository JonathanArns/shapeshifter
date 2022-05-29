use super::*;
use arrayvec::ArrayVec;

#[cfg(feature = "mcts")]
use rand::Rng;

pub fn allowed_moves<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, pos: u16) -> ArrayVec<Move, 4>
where [(); (W*H+63)/64]: Sized {
    let mut moves = ArrayVec::<Move, 4>::new();
    let mut some_legal_move = Move::Up;

    if let Some(dest) = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[pos as usize][0] {
        some_legal_move = Move::Up;
        if !board.bodies[0].get_bit(dest as usize) {
            moves.push(Move::Up);
        }
    }
    if let Some(dest) = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[pos as usize][1] {
        some_legal_move = Move::Down;
        if !board.bodies[0].get_bit(dest as usize) {
            moves.push(Move::Down);
        }
    }
    if let Some(dest) = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[pos as usize][2] {
        some_legal_move = Move::Right;
        if !board.bodies[0].get_bit(dest as usize) {
            moves.push(Move::Right);
        }
    }
    if let Some(dest) = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[pos as usize][3] {
        some_legal_move = Move::Left;
        if !board.bodies[0].get_bit(dest as usize) {
            moves.push(Move::Left);
        }
    }
    if moves.len() == 0 {
        moves.push(some_legal_move);
    }
    debug_assert!(moves == slow_allowed_moves(board, pos), "got {:?}, should get {:?}", moves, slow_allowed_moves(board, pos));
    moves
}

/// Can never return a move that moves out of bounds on the board on unrwapped boards,
/// because that would cause a panic elsewhere.
#[allow(unused)]
pub fn slow_allowed_moves<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, pos: u16) -> ArrayVec<Move, 4>
where [(); (W*H+63)/64]: Sized {
    let mut moves = ArrayVec::<Move, 4>::new();
    let mut some_legal_move = Move::Left;

    if WRAP {
        let move_to = (pos as usize + W) % (W*H);
        if !board.bodies[0].get_bit(move_to) {
            moves.push(Move::Up);
        }
        let move_to = if W > pos as usize { W*(H-1) + pos as usize } else { pos as usize - W };
        if !board.bodies[0].get_bit(move_to) {
            moves.push(Move::Down);
        }
        let move_to = if pos as usize % W == W-1 { pos as usize - (W-1) } else { pos as usize + 1};
        if !board.bodies[0].get_bit(move_to) {
            moves.push(Move::Right);
        }
        let move_to = if pos as usize % W == 0 { pos as usize + (W-1) } else { pos as usize - 1 };
        if !board.bodies[0].get_bit(move_to) {
            moves.push(Move::Left);
        }
    } else {
        if pos < (W * (H-1)) as u16 {
            some_legal_move = Move::Up;
            if !board.bodies[0].get_bit(pos as usize + W) {
                moves.push(Move::Up);
            }
        }
        if pos >= W as u16 {
            some_legal_move = Move::Down;
            if !board.bodies[0].get_bit(pos as usize - W) {
                moves.push(Move::Down);
            }
        }
        if pos % (W as u16) < (W as u16 - 1) {
            some_legal_move = Move::Right;
            if !board.bodies[0].get_bit(pos as usize + 1) {
                moves.push(Move::Right);
            }
        }
        if pos % (W as u16) > 0 {
            some_legal_move = Move::Left;
            if !board.bodies[0].get_bit(pos as usize - 1) {
                moves.push(Move::Left);
            }
        }
    }
    if moves.len() == 0 {
        moves.push(some_legal_move);
    }
    moves
}

/// Generates up to 4 move combinations from a position, such that every move for every snake has
/// been covered at least once.
/// Can skip the first n snakes, their moves will always be Up in the result.
#[allow(unused)]
pub fn limited_move_combinations<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, skip: usize) -> ArrayVec<[Move; S], 4>
where [(); (W*H+63)/64]: Sized {
    // only generate enough move combinations so that every enemy move appears at least once
    let mut moves = ArrayVec::<[Move; S], 4>::new();
    moves.push([Move::Up; S]);
    for i in skip..S {
        if board.snakes[i].is_dead() {
            continue
        }
        let mut x = 0;
        let mut some_legal_move = Move::Up;
        for j in 0..4 {
            if let Some(pos) = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[board.snakes[i].head as usize][j] {
                some_legal_move = Move::from_int(j as u8);
                if !board.bodies[0].get_bit(pos as usize) {
                    if moves.len() == x {
                        moves.push(moves[0]);
                    }
                    moves[x][i] = Move::from_int(j as u8);
                    x += 1;
                }
            }
        }
        for j in x..moves.len() {
            moves[j][i] = some_legal_move;
        }
    }
    moves
}

/// Generates up to 4 move combinations from a position, such that every move for every snake has
/// been covered at least once.
/// Can skip the first n snakes, their moves will always be Up in the result.
/// Applies move ordering to the individual moves of each enemy.
#[allow(unused)]
pub fn ordered_limited_move_combinations<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, skip: usize) -> ArrayVec<[Move; S], 4>
where [(); (W*H+63)/64]: Sized {
    // get moves for each enemy
    let mut moves_per_snake = ArrayVec::<ArrayVec<Move, 4>, S>::new();
    let mut i = 0;
    for snake in board.snakes[0+skip..].iter() {
        if snake.is_alive() {
            i += 1;
            let mut moves = allowed_moves(board, snake.head);
            moves.sort_by_key(|mv| {
                let dest = Bitboard::<S, W, H, WRAP>::MOVES_FROM_POSITION[snake.head as usize][mv.to_int() as usize].unwrap();
                let mut dist = board.distance(board.snakes[0].head, dest);
                if dist == 1 && snake.length <= board.snakes[0].length {
                    return dist + 10
                }
                dist
            });
            moves_per_snake.push(moves);
        } else {
            let mut none_move = ArrayVec::<_, 4>::new();
            none_move.insert(0, Move::Up);
            moves_per_snake.push(none_move);
        }
    }

    // only generate enough move combinations so that every enemy move appears at least once
    let mut moves = ArrayVec::<[Move; S], 4>::new();
    moves.push([Move::Up; S]);
    for (i, snake_moves) in moves_per_snake.iter().enumerate() {
        for j in 0..snake_moves.len().max(moves.len()) {
            if moves.len() <= j {
                moves.push(moves[0]);
            }
            moves[j][i+skip] = snake_moves[j.min(snake_moves.len()-1)];
        }
    }
    moves
}

/// Generates all possible move combinations from a position.
/// Can skip the first n snakes, their moves will always be Up in the result.
#[allow(unused)]
pub fn move_combinations<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, skip: usize) -> Vec<[Move; S]>
where [(); (W*H+63)/64]: Sized {
    // get moves for each enemy
    let mut moves_per_snake = ArrayVec::<ArrayVec<Move, 4>, S>::new();
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
    let mut moves: Vec<[Move; S]> = Vec::with_capacity(1 + S.pow(S as u32));
    moves.push([Move::Up; S]);
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

#[cfg(feature = "mcts")]
pub fn random_move_combination<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, rng: &mut impl Rng) -> [Move; S]
where [(); (W*H+63)/64]: Sized {
    let moves = limited_move_combinations(board, 0);
    moves[rng.gen_range(0..moves.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
    use test::Bencher;

    fn c(x: usize, y: usize) -> api::Coord {
        api::Coord{x, y}
    }

    fn create_board() -> Bitboard<4, 11, 11, true> {
        let state = api::GameState{
            game: api::Game{ id: "".to_string(), timeout: 100, ruleset: std::collections::HashMap::new() },
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
        Bitboard::<4, 11, 11, true>::from_gamestate(state)
    }

    #[bench]
    fn bench_enemy_move_generation(b: &mut Bencher) {
        let board = create_board();
        b.iter(|| {
            limited_move_combinations(&board, 1)
        });
    }
}
