use crate::types::*;
use crate::bitboard::*;
use crate::move_gen::*;
use crate::eval::*;

use rand::Rng;
use std::time;

struct Node<const S: usize, const W: usize, const H: usize>
where [(); (W*H+127)/128]: Sized {
    board: Bitboard<S, W, H>,
    moves: [Move; S],
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
            moves: [Move::Up; S],
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

    fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

pub fn search<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>, g: &Game) -> (Move, f64)
where [(); (W*H+127)/128]: Sized {
    let start_time = time::Instant::now();
    let mut root = Node::<S, W, H>::new(board.clone());
    while start_time.elapsed() < g.move_time / 2 {
        once(&mut root, g.ruleset);
    }
    let mut results = [(0, 0); 4];
    for child in root.children {
        let mut pair = &mut results[child.moves[0].to_int() as usize];
        pair.0 += child.visits;
        pair.1 += child.wins;
    }
    let (mut mv, mut winrate) = (Move::Up, 0_f64);
    for (i, pair) in results.iter().enumerate() {
        let wr = pair.1 as f64 / pair.0 as f64;
        if wr > winrate {
            mv = Move::from_int(i as u8);
            winrate = wr;
        }
    }
    (mv, winrate)
}

fn once<const S: usize, const W: usize, const H: usize>(root_node: &mut Node<S, W, H>, ruleset: Ruleset)
where [(); (W*H+127)/128]: Sized {
    // select
    let mut node = root_node;
    while !node.is_leaf() {
        node = select_child(node);
    }

    // expand
    node.expand();
    node = select_child(node);

    // simulate
    let result = playout(&node.board, ruleset);

    // propagate
    node.wins += result as u32;
    node.visits += 1;
    while let Some(parent) = &mut node.parent {
        node = parent;
        node.wins += result as u32;
        node.visits += 1;
    }
}

fn playout<const S: usize, const W: usize, const H:usize>(board: &Bitboard<S, W, H>, ruleset: Ruleset) -> bool
where [(); (W*H+127)/128]: Sized {
    let mut board = board.clone();
    let mut rng = rand::thread_rng();
    let mut moves;
    while !board.is_terminal() {
        moves = limited_move_combinations(&board, 0);
        board.apply_moves(&moves[rng.gen_range(0..moves.len())], ruleset);
    }
    eval_terminal(&board) > 0
}

fn select_child<const S: usize, const W: usize, const H: usize>(node: &mut Node<S, W, H>) -> &mut Node<S, W, H>
where [(); (W*H+127)/128]: Sized {
    &mut node.children[0]
}
