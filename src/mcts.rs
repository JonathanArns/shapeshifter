use crate::types::*;
use crate::bitboard::*;
use crate::move_gen::*;

use rand::Rng;
use rand_pcg::Pcg64Mcg;
use std::time;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::btree_map::BTreeMap;

struct Node<const S: usize, const W: usize, const H: usize, const WRAP: bool>
where [(); (W*H+127)/128]: Sized {
    board: Bitboard<S, W, H, WRAP>,
    moves: [Move; S],
    parent: Option<Rc<RefCell<Node<S, W, H, WRAP>>>>,
    children: BTreeMap<[Move; S], Rc<RefCell<Node<S, W, H, WRAP>>>>,
    visits_and_wins_per_snake_move: [[Option<(u32, u32)>; 4]; S],
    visits: u32,
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool> Node<S, W, H, WRAP> 
where [(); (W*H+127)/128]: Sized {
    fn new(board: Bitboard<S, W, H, WRAP>, moves: [Move; S], parent: Option<Rc<RefCell<Self>>>) -> Self {
        // this is effectively the move generation for the in memory tree
        let mut visits_and_wins = [[None; 4]; S];
        for i in 0..S {
            if board.snakes[i].is_alive() {
                let moves = allowed_moves(&board, board.snakes[i].head);
                for mv in moves {
                    visits_and_wins[i][mv.to_int() as usize] = Some((0, 0));
                }
            }
        }
        Node::<S, W, H, WRAP>{
            board,
            moves,
            parent,
            children: BTreeMap::default(),
            visits_and_wins_per_snake_move: visits_and_wins,
            visits: 0,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.len() == 0
    }

    fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

fn expand<const S: usize, const W: usize, const H: usize, const WRAP: bool>(node: Rc<RefCell<Node<S, W, H, WRAP>>>, moves: [Move; S]) -> Rc<RefCell<Node<S, W, H, WRAP>>>
where [(); (W*H+127)/128]: Sized {
    let mut board = node.borrow().board.clone();
    board.apply_moves(&moves);
    if board.is_over().0 {
        return node
    }
    let new = Rc::new(RefCell::new(Node::<S, W, H, WRAP>::new(board, moves, Some(Rc::clone(&node)))));
    node.borrow_mut().children.insert(moves, Rc::clone(&new));
    new
}

pub fn search<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, deadline: time::Instant) -> (Move, f64)
where [(); (W*H+127)/128]: Sized {
    let mut node_counter = 0;
    let mut rng = Pcg64Mcg::new(91825765198273048172569872943871926276_u128);
    let start_time = time::Instant::now();
    let root = Rc::new(RefCell::new(Node::<S, W, H, WRAP>::new(board.clone(), [Move::Up; S], None)));
    while time::Instant::now() < deadline {
        once(Rc::clone(&root), &mut rng, &mut node_counter);
    }
    let mut best_winrate = 0_f64;
    let mut best_move_int = 0;
    for i in 0..4 {
        if let Some((visits, wins)) = root.borrow().visits_and_wins_per_snake_move[0][i] {
            let winrate = wins as f64 / visits as f64;
            if winrate > best_winrate {
                best_winrate = winrate;
                best_move_int = i;
            }
        }
    }
    println!("{:?}", root.borrow().visits_and_wins_per_snake_move);
    println!("{:?} nodes total, {:?} nodes per second", node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    (Move::from_int(best_move_int as u8), best_winrate)
}

fn once<const S: usize, const W: usize, const H: usize, const WRAP: bool>(root: Rc<RefCell<Node<S, W, H, WRAP>>>, rng: &mut impl Rng, node_counter: &mut u64)
where [(); (W*H+127)/128]: Sized {
    // select
    let mut node = root;
    let mut moves = select_child(Rc::clone(&node));
    loop {
        let child;
        if let Some(tmp) = node.borrow().children.get(&moves) {
            child = Rc::clone(tmp);
        } else {
            break
        };
        node = child;
        moves = select_child(Rc::clone(&node));
    }

    node = expand(node, moves);

    // simulate
    let (result, mut moves_made) = playout(&node.borrow().board, rng, node_counter);
    
    // propagate
    loop {
        node.borrow_mut().visits += 1;
        for i in 0..S {
            if let Some(vw) = node.borrow_mut().visits_and_wins_per_snake_move[i][moves_made[i].to_int() as usize].as_mut() {
                vw.0 += 1;
            }
        }
        if let Some(winner_idx) = result {
            if let Some(vw) = node.borrow_mut().visits_and_wins_per_snake_move[winner_idx][moves_made[winner_idx].to_int() as usize].as_mut() {
                vw.1 += 1;
            }
        }
        let parent;
        if let Some(tmp) = &node.borrow().parent {
            parent = Rc::clone(tmp);
        } else {
            break
        };
        moves_made = node.borrow().moves;
        node = parent;
    }
}

// returns the winner's snake index
fn playout<const S: usize, const W: usize, const H:usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>, rng: &mut impl Rng, node_counter: &mut u64) -> (Option<usize>, [Move; S])
where [(); (W*H+127)/128]: Sized {
    let mut board = board.clone();
    let mut moves = random_move_combination(&board, rng);
    let first_moves = moves;
    let mut result = board.is_over();
    while !result.0 { // is_over
        *node_counter += 1;
        board.apply_moves(&moves);
        moves = random_move_combination(&board, rng);
        result = board.is_over();
    }
    return (result.1, first_moves)
}

fn select_child<const S: usize, const W: usize, const H: usize, const WRAP: bool>(node: Rc<RefCell<Node<S, W, H, WRAP>>>) -> [Move; S]
where [(); (W*H+127)/128]: Sized {
    let node_visits = node.borrow().visits;
    let mut moves = [Move::Up; S];
    for i in 0..S {
        let mut best_val = 0_f64;
        for j in 0..4 {
            if let Some((move_visits, wins)) = node.borrow().visits_and_wins_per_snake_move[i][j] {
                if move_visits == 0 {
                    moves[i] = Move::from_int(j as u8);
                    break
                }
                let val = duct(node_visits.into(), move_visits.into(), wins.into());
                if best_val < val {
                    best_val = val;
                    moves[i] = Move::from_int(j as u8);
                }
            }
        }
    }
    moves
}

fn duct(node_visits: f64, move_visits: f64, wins: f64) -> f64 {
    const C: f64 = 2.5;
    let winrate = wins / node_visits;
    winrate + C * (node_visits.ln() / move_visits).sqrt()
}
