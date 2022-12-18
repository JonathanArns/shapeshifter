use super::*;
use arrayvec::ArrayVec;

// #[cfg(feature = "mcts")]
use rand::Rng;

pub fn allowed_moves<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, snake_index: usize) -> ArrayVec<Move, 4>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut moves = ArrayVec::<Move, 4>::new();
    let mut some_legal_move = Move::Up;
    let mut some_better_legal_move = None;
    let pos = board.snakes[snake_index].head;
    let survives_hazard = board.snakes[snake_index].health > board.hazard_dmg;

    for (mv_int, optional_dest) in Bitboard::<S, W, H, WRAP, HZSTACK>::MOVES_FROM_POSITION[pos as usize].iter().enumerate() {
        if let Some(dest) = *optional_dest {
            some_legal_move = Move::from_int(mv_int as u8);
            if survives_hazard || !board.hazard_mask.get(dest as usize) || board.food.get(dest as usize) {
                some_better_legal_move = Some(some_legal_move);
                if !board.bodies[0].get(dest as usize) {
                    moves.push(some_legal_move);
                }
            }
        }
    }
    if let Some(mv) = some_better_legal_move {
        some_legal_move = mv;
    }
    if moves.len() == 0 {
        moves.push(some_legal_move);
    }
    moves
}

/// Can never return a move that moves out of bounds on the board on unrwapped boards,
/// because that would cause a panic elsewhere.
#[allow(unused)]
pub fn old_allowed_moves<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, snake_index: usize) -> ArrayVec<Move, 4>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let pos = board.snakes[snake_index].head;
    let mut moves = ArrayVec::<Move, 4>::new();
    let mut some_legal_move = Move::Left;

    if WRAP {
        let move_to = (pos as usize + W) % (W*H);
        if !board.bodies[0].get(move_to) {
            moves.push(Move::Up);
        }
        let move_to = if W > pos as usize { W*(H-1) + pos as usize } else { pos as usize - W };
        if !board.bodies[0].get(move_to) {
            moves.push(Move::Down);
        }
        let move_to = if pos as usize % W == W-1 { pos as usize - (W-1) } else { pos as usize + 1};
        if !board.bodies[0].get(move_to) {
            moves.push(Move::Right);
        }
        let move_to = if pos as usize % W == 0 { pos as usize + (W-1) } else { pos as usize - 1 };
        if !board.bodies[0].get(move_to) {
            moves.push(Move::Left);
        }
    } else {
        if pos < (W * (H-1)) as u16 {
            some_legal_move = Move::Up;
            if !board.bodies[0].get(pos as usize + W) {
                moves.push(Move::Up);
            }
        }
        if pos >= W as u16 {
            some_legal_move = Move::Down;
            if !board.bodies[0].get(pos as usize - W) {
                moves.push(Move::Down);
            }
        }
        if pos % (W as u16) < (W as u16 - 1) {
            some_legal_move = Move::Right;
            if !board.bodies[0].get(pos as usize + 1) {
                moves.push(Move::Right);
            }
        }
        if pos % (W as u16) > 0 {
            some_legal_move = Move::Left;
            if !board.bodies[0].get(pos as usize - 1) {
                moves.push(Move::Left);
            }
        }
    }
    if moves.len() == 0 {
        moves.push(some_legal_move);
    }
    moves
}

pub fn ordered_allowed_moves<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    snake_index: usize,
    history: &[[u64; 4]; W*H]
) -> ArrayVec<Move, 4>
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut moves = allowed_moves(board, snake_index);
    moves.sort_by_key(|mv| {
        let dest = Bitboard::<S, W, H, WRAP, HZSTACK>::MOVES_FROM_POSITION[board.snakes[snake_index].head as usize][mv.to_int() as usize].unwrap();
        let mut options = 1;
        for i in 0..4 {
            if let Some(pos) = Bitboard::<S, W, H, WRAP, HZSTACK>::MOVES_FROM_POSITION[dest as usize][i] {
                options += (!board.bodies[0].get(pos as usize) && (board.hazard_dmg < 90 || !board.hazard_mask.get(pos as usize))) as u64;
            }
        }
        for snake in board.snakes {
            if snake.is_alive() && snake.tail == dest {
                options += 1;
            }
        }
        u64::MAX - 10000 - history[board.snakes[snake_index].head as usize][mv.to_int() as usize] - options
    });
    moves
}

/// Generates up to 4 move combinations from a position, such that every move for every snake has
/// been covered at least once.
/// Can skip the first n snakes, their moves will always be Up in the result.
#[allow(unused)]
pub fn limited_move_combinations<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, skip: usize) -> ArrayVec<[Move; S], 4>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
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
            if let Some(pos) = Bitboard::<S, W, H, WRAP, HZSTACK>::MOVES_FROM_POSITION[board.snakes[i].head as usize][j] {
                some_legal_move = Move::from_int(j as u8);
                if !board.bodies[0].get(pos as usize) {
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
pub fn ordered_limited_move_combinations<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, skip: usize, history: &[[u64; 4]; W*H]) -> ArrayVec<[Move; S], 4>
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    // get moves for each enemy
    let mut moves_per_snake = ArrayVec::<ArrayVec<Move, 4>, S>::new();
    let mut i = 0;
    for (j, snake) in board.snakes[skip..].iter().enumerate() {
        if snake.is_alive() {
            i += 1;
            let mut moves = ordered_allowed_moves(board, j+skip, history);
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

/// Generates enemy moves for best reply search (BRS+)
/// This function does not eliminate duplicates in the returned list
#[allow(unused)]
pub fn brs_move_combinations<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK>,
    history: &[[u64; 4]; W*H]
) -> ArrayVec<[Move; S], {(S-1)*4}>
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized, [(); (S-1)*4]: Sized {
    const skip: usize = 1;
    // get moves for each enemy
    let mut moves_per_snake = ArrayVec::<ArrayVec<Move, 4>, S>::new();
    let mut i = 0;
    for (j, snake) in board.snakes[skip..].iter().enumerate() {
        if snake.is_alive() {
            i += 1;
            let mut moves = ordered_allowed_moves(board, j+skip, history);
            moves_per_snake.push(moves);
        } else {
            let mut none_move = ArrayVec::<_, 4>::new();
            none_move.insert(0, Move::Up);
            moves_per_snake.push(none_move);
        }
    }

    // find default moves
    let mut default_moves = [Move::Up; S];
    for (i, snake_moves) in moves_per_snake.iter().enumerate() {
        default_moves[i+skip] = snake_moves[0];
    }

    // only generate enough move combinations so that every enemy move appears at least once
    let mut moves = ArrayVec::<[Move; S], {(S-1)*4}>::default();
    for (i, snake_moves) in moves_per_snake.iter().enumerate() {
        for mv in snake_moves {
            let mut mvs = default_moves.clone();
            mvs[i+skip] = *mv;
            moves.push(mvs);
        }
    }
    moves
}

/// Generates all possible move combinations from a position.
/// Can skip the first n snakes, their moves will always be Up in the result.
#[allow(unused)]
pub fn move_combinations<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, skip: usize) -> Vec<[Move; S]>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    // get moves for each enemy
    let mut moves_per_snake = ArrayVec::<ArrayVec<Move, 4>, S>::new();
    for (j, snake) in board.snakes[skip..].iter().enumerate() {
        if snake.is_alive() {
            moves_per_snake.push(allowed_moves(board, j+skip));
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

// #[cfg(feature = "mcts")]
pub fn random_move_combination<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, rng: &mut impl Rng) -> [Move; S]
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let moves = limited_move_combinations(board, 0);
    moves[rng.gen_range(0..moves.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use rand::Rng;
    use rand_pcg::Pcg64Mcg;

    fn create_board() -> Bitboard<4, 11, 11, true, false> {
        let val = r###"{"game":{"id":"7ddd5c60-e27a-42ae-985e-f056e5695836","ruleset":{"name":"wrapped","version":"?","settings":{"foodSpawnChance":15,"minimumFood":1,"hazardDamagePerTurn":100,"royale":{},"squad":{"allowBodyCollisions":false,"sharedElimination":false,"sharedHealth":false,"sharedLength":false}}},"map":"hz_islands_bridges","timeout":500,"source":"league"},"turn":445,"board":{"width":11,"height":11,"food":[{"x":1,"y":9},{"x":1,"y":8},{"x":9,"y":1},{"x":6,"y":3},{"x":7,"y":3},{"x":7,"y":4},{"x":8,"y":3},{"x":4,"y":9},{"x":10,"y":8},{"x":6,"y":6}],"hazards":[{"x":5,"y":10},{"x":5,"y":9},{"x":5,"y":7},{"x":5,"y":6},{"x":5,"y":5},{"x":5,"y":4},{"x":5,"y":3},{"x":5,"y":0},{"x":5,"y":1},{"x":6,"y":5},{"x":7,"y":5},{"x":9,"y":5},{"x":10,"y":5},{"x":4,"y":5},{"x":3,"y":5},{"x":1,"y":5},{"x":0,"y":5},{"x":1,"y":10},{"x":9,"y":10},{"x":1,"y":0},{"x":9,"y":0},{"x":10,"y":1},{"x":10,"y":0},{"x":10,"y":10},{"x":10,"y":9},{"x":0,"y":10},{"x":0,"y":9},{"x":0,"y":1},{"x":0,"y":0},{"x":0,"y":6},{"x":0,"y":4},{"x":10,"y":6},{"x":10,"y":4},{"x":6,"y":10},{"x":4,"y":10},{"x":6,"y":0},{"x":4,"y":0}],"snakes":[{"id":"gs_P3P9rW63VPgMcYFFJ9R6McrM","name":"Shapeshifter","health":91,"body":[{"x":6,"y":2},{"x":6,"y":1},{"x":7,"y":1},{"x":7,"y":0},{"x":7,"y":10},{"x":8,"y":10},{"x":8,"y":0},{"x":8,"y":1},{"x":8,"y":2},{"x":9,"y":2},{"x":9,"y":3},{"x":10,"y":3},{"x":10,"y":2},{"x":0,"y":2},{"x":0,"y":3},{"x":1,"y":3},{"x":1,"y":4},{"x":2,"y":4},{"x":3,"y":4},{"x":3,"y":3},{"x":2,"y":3},{"x":2,"y":2},{"x":1,"y":2},{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":0},{"x":3,"y":0},{"x":3,"y":1},{"x":4,"y":1},{"x":4,"y":2}],"latency":11,"head":{"x":6,"y":2},"length":30,"shout":"","squad":"","customizations":{"color":"#900050","head":"cosmic-horror-special","tail":"cosmic-horror"}},{"id":"gs_YMFKJHvJwS9VV7SgtTMVmKVQ","name":"🇺🇦 Jagwire 🇺🇦","health":76,"body":[{"x":9,"y":9},{"x":8,"y":9},{"x":7,"y":9},{"x":6,"y":9},{"x":6,"y":8},{"x":5,"y":8},{"x":4,"y":8},{"x":3,"y":8},{"x":3,"y":9},{"x":3,"y":10},{"x":2,"y":10},{"x":2,"y":9},{"x":2,"y":8},{"x":2,"y":7},{"x":3,"y":7},{"x":4,"y":7},{"x":4,"y":6},{"x":3,"y":6},{"x":2,"y":6},{"x":1,"y":6},{"x":1,"y":7},{"x":0,"y":7},{"x":10,"y":7},{"x":9,"y":7},{"x":9,"y":6},{"x":8,"y":6},{"x":7,"y":6},{"x":7,"y":7},{"x":7,"y":8},{"x":8,"y":8},{"x":9,"y":8}],"latency":23,"head":{"x":9,"y":9},"length":31,"shout":"","squad":"","customizations":{"color":"#ffd900","head":"smile","tail":"wave"}}]},"you":{"id":"gs_P3P9rW63VPgMcYFFJ9R6McrM","name":"Shapeshifter","health":91,"body":[{"x":6,"y":2},{"x":6,"y":1},{"x":7,"y":1},{"x":7,"y":0},{"x":7,"y":10},{"x":8,"y":10},{"x":8,"y":0},{"x":8,"y":1},{"x":8,"y":2},{"x":9,"y":2},{"x":9,"y":3},{"x":10,"y":3},{"x":10,"y":2},{"x":0,"y":2},{"x":0,"y":3},{"x":1,"y":3},{"x":1,"y":4},{"x":2,"y":4},{"x":3,"y":4},{"x":3,"y":3},{"x":2,"y":3},{"x":2,"y":2},{"x":1,"y":2},{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":0},{"x":3,"y":0},{"x":3,"y":1},{"x":4,"y":1},{"x":4,"y":2}],"latency":11,"head":{"x":6,"y":2},"length":30,"shout":"","squad":"","customizations":{"color":"#900050","head":"cosmic-horror-special","tail":"cosmic-horror"}}}"###;
        Bitboard::<4, 11, 11, true, false>::from_str(&val).unwrap()
    }

    #[bench]
    fn bench_enemy_move_generation(b: &mut Bencher) {
        let board = create_board();
        b.iter(|| {
            limited_move_combinations(&board, 1)
        });
    }

    #[bench]
    fn bench_random_simulate(b: &mut Bencher) {
        let mut board = create_board();
        let mut rng = Pcg64Mcg::new(91825765198273048172569872943871926276_u128);
        b.iter(|| {
            let moves = random_move_combination(&board, &mut rng);
            (board.apply_moves.clone())(&mut board, &moves);
        });
    }
}
