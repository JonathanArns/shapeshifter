use crate::types::*;
use crate::bitboard::*;
use crate::move_gen::*;

struct Node<const S: usize, const W: usize, const H: usize>
where [(); (W*H+127)/128]: Sized {
    board: Bitboard<S, W, H>,
    moves: Move,
    parent: Option<Box<Node<S, W, H>>>,
    children: Vec<Node<S, W, H>>,
    wins: u32,
    visits: u32,
}

impl<const S: usize, const W: usize, const H: usize> Node<S, W, H> 
where [(); (W*H+127)/128]: Sized {
    fn new(board: Bitboard<S, W, H>) -> Self {
        Node::<S, W, H>{
            board,
            moves: Move::Up,
            parent: None,
            children: vec![],
            wins: 0,
            visits: 0,
        }
    }

    fn expand(&mut self) {
        if self.board.is_terminal() {
            return
        }
        for moves in move_combinations(&self.board, 0) {
            todo!()
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.len() == 0
    }
}

pub fn search<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>) -> (Move, Score)
where [(); (W*H+127)/128]: Sized {
    todo!()
}

// fn playout<const N: usize>(board: &Bitboard<N>) -> bool {
//     let mut board = board.clone();
//     while !board.is_terminal() {
//         board.
//     }
// }
