use crate::bitboard::*;
use crate::bitboard::move_gen::*;

use arrayvec::ArrayVec;
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use std::time;
use std::rc::{Rc, Weak};
use std::cell::RefCell;

enum Moves<const S: usize> {
    Me(ArrayVec<Move, 4>),
    Enemies(Vec<[Move; S]>),
}

impl<const S: usize> Moves<S> {
    fn len(&self) -> usize {
        match self {
            Self::Me(x) => x.len(),
            Self::Enemies(x) => x.len(),
        }
    }
    fn get_my_move(&self, idx: usize) -> Move {
        match self {
            Self::Me(x) => x[idx],
            Self::Enemies(_) => panic!("tried to get my move from enemy moves"),
        }
    }
}

struct Node<const S: usize, const W: usize, const H: usize, const WRAP: bool>
where [(); (W*H+127)/128]: Sized {
    max: bool,
    board: Bitboard<S, W, H, WRAP>,
    parent: Option<Weak<RefCell<Node<S, W, H, WRAP>>>>,
    moves_idx: usize,
    moves: Moves<S>,
    children: Vec<Option<Rc<RefCell<Node<S, W, H, WRAP>>>>>,
    visits: u32,
    wins: u32,
    value: f64,
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool> Node<S, W, H, WRAP> 
where [(); (W*H+127)/128]: Sized {
    fn new(board: Bitboard<S, W, H, WRAP>, moves_idx: usize, parent: Option<Weak<RefCell<Self>>>, max: bool) -> Self {
        // this is effectively the move generation for the in memory tree
        let moves = if max {
            Moves::Me(allowed_moves(&board, board.snakes[0].head))
        } else {
            Moves::Enemies(move_combinations(&board, 1))
        };

        Node::<S, W, H, WRAP>{
            max,
            board,
            parent,
            moves_idx,
            children: vec![None; moves.len()],
            moves,
            visits: 0,
            wins: 0,
            value: 0.0,
        }
    }
}

fn expand<const S: usize, const W: usize, const H: usize, const WRAP: bool>(node: Rc<RefCell<Node<S, W, H, WRAP>>>, moves_idx: usize) -> Rc<RefCell<Node<S, W, H, WRAP>>>
where [(); (W*H+127)/128]: Sized {
    let mut board = node.borrow().board.clone();
    if !node.borrow().max {
        // get enemy moves from node
        let mut moves = if let Moves::Enemies(mvs) = &node.borrow().moves {
            mvs[moves_idx]
        } else {
            panic!("Min node does not have enemies moves");
        };
        // get my move from parent
        moves[0] = if let Some(parent) = &node.borrow().parent {
            if let Moves::Me(mvs) = &parent.upgrade().unwrap().borrow().moves {
                mvs[node.borrow().moves_idx]
            } else {
                panic!("Min node's parent does not have me moves")
            }
        } else {
            panic!("Min node does not have parent");
        };
        board.apply_moves(&moves);
    }
    let new = Rc::new(RefCell::new(Node::<S, W, H, WRAP>::new(board, moves_idx, Some(Rc::downgrade(&node)), !node.borrow().max)));
    node.borrow_mut().children[moves_idx] =  Some(Rc::clone(&new));
    new
}

pub fn search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, deadline: time::Instant) -> (Move, f64)
where [(); (W*H+127)/128]: Sized {
    let mut node_counter = 0;
    let mut rng = Pcg64Mcg::new(91825765198273048172569872943871926276_u128);
    let start_time = time::Instant::now();
    let root = Rc::new(RefCell::new(Node::<S, W, H, WRAP>::new(board.clone(), 0, None, true)));
    while time::Instant::now() < deadline {
        once(Rc::clone(&root), &mut rng, &mut node_counter);
    }
    let moves = if let Moves::Me(mvs) = &root.borrow().moves {
        mvs.clone()
    } else {
        panic!("search root does not have me moves");
    };
    let mut best_winrate = 0_f64;
    let mut best_move = Move::Up;
    for (i, child) in root.borrow().children.iter().enumerate() {
        if let Some(node) = child {
            let winrate = node.borrow().wins as f64 / node.borrow().visits as f64;
            if winrate > best_winrate {
                best_winrate = winrate;
                best_move = moves[i];
            }
            print!("({:?}:{}:{})", root.borrow().moves.get_my_move(node.borrow().moves_idx), node.borrow().wins, node.borrow().visits);
        }
    }
    println!("\n{:?} nodes total, {:?} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    (best_move, best_winrate)
}

fn once<const S: usize, const W: usize, const H: usize, const WRAP: bool>(root: Rc<RefCell<Node<S, W, H, WRAP>>>, rng: &mut impl Rng, node_counter: &mut u64)
where [(); (W*H+127)/128]: Sized {
    // select
    let mut node = root;
    let mut moves_idx = select_child(Rc::clone(&node));
    loop {
        let child;
        if let Some(tmp) = &node.borrow().children[moves_idx] {
            child = Rc::clone(&tmp);
        } else {
            break
        };
        node = child;
        moves_idx = select_child(Rc::clone(&node));
    }

    node = expand(node, moves_idx);
    moves_idx = select_child(Rc::clone(&node));

    // simulate
    let result = playout(Rc::clone(&node), moves_idx, rng, node_counter);
    
    // propagate
    loop {
        // TODO: deal with terminal nodes in the tree somehow somewhere
        // TODO: treat min nodes correctly?
        node.borrow_mut().visits += 1;
        if (result == 1 && !node.borrow().max) || (result == -1 && node.borrow().max) {
            node.borrow_mut().wins += 1;
        }
        let parent;
        if let Some(tmp) = &node.borrow().parent {
            parent = Weak::clone(tmp);
        } else {
            break
        };
        node = parent.upgrade().unwrap();
    }
}

// returns the winner's snake index
fn playout<const S: usize, const W: usize, const H:usize, const WRAP: bool>(node: Rc<RefCell<Node<S, W, H, WRAP>>>, moves_idx: usize, rng: &mut impl Rng, node_counter: &mut u64) -> i8
where [(); (W*H+127)/128]: Sized {
    let mut board = node.borrow().board.clone();
    let mut moves = match &node.borrow().moves {
        Moves::Me(mvs) => {
            let mut tmp = random_move_combination(&board, rng);
            tmp[0] = mvs[moves_idx];
            tmp
        },
        Moves::Enemies(mvs) => {
            let mut tmp = mvs[moves_idx];
            tmp[0] = if let Moves::Me(ref mvs) = node.borrow().parent.as_ref().unwrap().upgrade().unwrap().borrow().moves {
                mvs[node.borrow().moves_idx]
            } else {
                panic!("parent node of min node does not have me moves");
            };
            tmp
        },
    };
    while !board.is_terminal() {
        *node_counter += 1;
        board.apply_moves(&moves);
        moves = random_move_combination(&board, rng);
    }
    if board.snakes[0].is_alive() {
        return 1
    }
    for snake in board.snakes[1..].iter() {
        if snake.is_alive() {
            return -1
        }
    }
    return 0
}

fn select_child<const S: usize, const W: usize, const H: usize, const WRAP: bool>(node: Rc<RefCell<Node<S, W, H, WRAP>>>) -> usize
where [(); (W*H+127)/128]: Sized {
    let parent_visits = node.borrow().visits;
    let mut best_val = 0_f64;
    let mut best_moves_idx = 0;
    for (i, x) in node.borrow().children.iter().enumerate() {
        if let Some(child) = x {
            let val = ucb1(parent_visits.into(), child.borrow().visits.into(), child.borrow().wins.into());
            if best_val < val {
                best_val = val;
                best_moves_idx = i;
            }
        } else {
            return i
        }
    }
    best_moves_idx
}

fn ucb1(parent_visits: f64, child_visits: f64, child_wins: f64) -> f64 {
    const C: f64 = 1.5;
    let winrate = child_wins / child_visits;
    winrate + C * (parent_visits.ln() / child_visits).sqrt()
}
