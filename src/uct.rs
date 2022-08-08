use crate::bitboard::*;
use crate::bitboard::move_gen::*;

use arrayvec::ArrayVec;
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use std::time;

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

struct Node<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    max: bool,
    board: Bitboard<S, W, H, WRAP, HZSTACK>,
    idx: usize,
    parent: Option<usize>,
    moves_idx: usize,
    moves: Moves<S>,
    children: Vec<Option<usize>>,
    visits: u32,
    wins: u32,
    lower_bound: i8,
    upper_bound: i8,
}

impl<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool> Node<S, W, H, WRAP, HZSTACK> 
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    fn new(board: Bitboard<S, W, H, WRAP, HZSTACK>, idx: usize, moves_idx: usize, parent: Option<usize>, max: bool) -> Self {
        // this is effectively the move generation for the in memory tree
        let moves = if max {
            Moves::Me(allowed_moves(&board, 0))
        } else {
            Moves::Enemies(move_combinations(&board, 1))
        };

        Node::<S, W, H, WRAP, HZSTACK>{
            max,
            board,
            idx,
            parent,
            moves_idx,
            children: vec![None; moves.len()],
            moves,
            visits: 0,
            wins: 0,
            lower_bound: -1,
            upper_bound: 1,
        }
    }
}

fn expand<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(tree: &mut Vec<Node<S, W, H, WRAP, HZSTACK>>, node_idx: usize, moves_idx: usize) -> usize
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut board = tree[node_idx].board.clone();
    if !tree[node_idx].max {
        // get enemy moves from node
        let mut moves = if let Moves::Enemies(mvs) = &tree[node_idx].moves {
            mvs[moves_idx]
        } else {
            panic!("Min node does not have enemies moves");
        };
        // get my move from parent
        moves[0] = if let Some(parent_idx) = tree[node_idx].parent {
            if let Moves::Me(mvs) = &tree[parent_idx].moves {
                mvs[tree[node_idx].moves_idx]
            } else {
                panic!("Min node's parent does not have me moves")
            }
        } else {
            panic!("Min node does not have parent");
        };
        (board.apply_moves.clone())(&mut board, &moves);
    }
    let idx = tree.len();
    let new = Node::<S, W, H, WRAP, HZSTACK>::new(board, idx, moves_idx, Some(tree[node_idx].idx), !tree[node_idx].max);
    tree.push(new);
    tree[node_idx].children[moves_idx] = Some(idx);
    idx
}

pub fn search<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(board: &Bitboard<S, W, H, WRAP, HZSTACK>, deadline: time::SystemTime) -> (Move, f64)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut tree = Vec::<Node<S, W, H, WRAP, HZSTACK>>::with_capacity(100000);
    let mut rng = Pcg64Mcg::new(91825765198273048172569872943871926276_u128);
    let mut node_counter = 0;
    let mut iteration_counter = 0;
    let start_time = time::Instant::now();

    // create root
    tree.push(Node::<S, W, H, WRAP, HZSTACK>::new(board.clone(), 0, 0, None, true));

    // compute
    while time::SystemTime::now() < deadline {
        iteration_counter += 1;
        once(&mut tree, &mut rng, &mut node_counter);
        if tree[0].lower_bound == tree[0].upper_bound {
            break
        }
    }

    // extract the result from the tree
    let moves = if let Moves::Me(mvs) = &tree[0].moves {
        mvs.clone()
    } else {
        panic!("search root does not have me moves");
    };
    let mut best_winrate = 0_f64;
    let mut best_move = Move::Up;
    for (i, child) in tree[0].children.iter().enumerate() {
        if let Some(node_idx) = child {
            let mut winrate = 0.0_f64;
            if tree[*node_idx].lower_bound == 0 {
                winrate = 0.5;
            } else if tree[*node_idx].lower_bound == 1 {
                winrate = 1.0;
            }
            winrate = winrate.max(tree[*node_idx].wins as f64 / tree[*node_idx].visits as f64);
            if tree[*node_idx].upper_bound == -1 {
                winrate = 0.0;
            } else if tree[*node_idx].upper_bound == 0 {
                winrate = winrate.min(0.5);
            }
            if winrate > best_winrate {
                best_winrate = winrate;
                best_move = moves[i];
            }
            print!("({:?}:{}:{} {} to {})", tree[0].moves.get_my_move(tree[*node_idx].moves_idx), tree[*node_idx].wins, tree[*node_idx].visits, tree[*node_idx].lower_bound,  tree[*node_idx].upper_bound);
        }
    }
    println!("\n{:?} iterations, {:?} nodes total, {:?} nodes per second", iteration_counter, node_counter, node_counter as u128 * (time::Duration::from_secs(1).as_nanos() / start_time.elapsed().as_nanos()));
    println!("{:?} with wr {}\n", best_move, best_winrate);
    (best_move, best_winrate)
}

fn update_bounds<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(
    tree: &mut Vec<Node<S, W, H, WRAP, HZSTACK>>,
    node_idx: usize,
    mut lower: Option<i8>,
    mut upper: Option<i8>
)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    if let Some(x) = lower {
        if tree[node_idx].lower_bound < x {
            if tree[node_idx].max {
                tree[node_idx].lower_bound = x;
            } else {
                let mut new_lower = x;
                for child in &tree[node_idx].children {
                    if let Some(child_idx) = child {
                        if tree[*child_idx].lower_bound < new_lower {
                            new_lower = tree[*child_idx].lower_bound;
                        }
                    } else {
                        // child has not been expanded yet, so it has default bounds
                        new_lower = -1;
                        break
                    }
                }
                if tree[node_idx].lower_bound != new_lower {
                    tree[node_idx].lower_bound = new_lower;
                    // if !tree[node_idx].max {
                    //     println!("{}lower_bound: {} max: {:?}", tree[node_idx].board, new_lower, tree[node_idx].max);
                    // }
                    lower = Some(new_lower);
                } else {
                    lower = None; // don't propagate further
                }
            }
        }
    }
    if let Some(x) = upper {
        if tree[node_idx].upper_bound > x {
            if !tree[node_idx].max {
                tree[node_idx].upper_bound = x;
            } else {
                let mut new_upper = x;
                for child in &tree[node_idx].children {
                    if let Some(child_idx) = child {
                        if tree[*child_idx].upper_bound > new_upper {
                            new_upper = tree[*child_idx].upper_bound;
                        }
                    } else {
                        // child has not been expanded yet, so it has default bounds
                        new_upper = 1;
                        break
                    }
                }
                if tree[node_idx].upper_bound != new_upper {
                    tree[node_idx].upper_bound = new_upper;
                    if !tree[node_idx].max {
                        println!("{}upper_bound: {} max: {:?}", tree[node_idx].board, new_upper, tree[node_idx].max);
                    }
                    upper = Some(new_upper);
                } else {
                    upper = None; // don't propagate further
                }
            }
        }
    }
    if node_idx == 0 {
        return
    }
    update_bounds(tree, tree[node_idx].parent.unwrap(), lower, upper);
}

fn once<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(tree: &mut Vec<Node<S, W, H, WRAP, HZSTACK>>, rng: &mut impl Rng, node_counter: &mut u64)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    // select
    let mut node_idx = 0;
    let mut moves_idx = select_child(tree, node_idx);
    loop {
        if let Some(idx) = tree[node_idx].children[moves_idx] {
            // return early, if the node has been solved
            if tree[node_idx].lower_bound == tree[node_idx].upper_bound {
                propagate(tree, node_idx, tree[node_idx].lower_bound);
                return
            }
            node_idx = idx;
        } else {
            break
        };
        moves_idx = select_child(tree, node_idx);
    }

    node_idx = expand(tree, node_idx, moves_idx);
    let result = if let Some(value) = tree[node_idx].board.win_draw_loss() {
        update_bounds(tree, node_idx, Some(value), Some(value));
        value
    } else {
        // simulate
        moves_idx = select_child(tree, node_idx);
        playout(tree, node_idx, moves_idx, rng, node_counter)
    };
    propagate(tree, node_idx, result);
}

fn propagate<const S: usize, const W: usize, const H:usize, const WRAP: bool, const HZSTACK: bool>(tree: &mut Vec<Node<S, W, H, WRAP, HZSTACK>>, mut node_idx: usize, result: i8)
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    // propagate
    loop {
        // TODO: deal with terminal nodes in the tree somehow somewhere
        // TODO: treat min nodes correctly?
        tree[node_idx].visits += 1;
        if (result == 1 && !tree[node_idx].max) || (result == -1 && tree[node_idx].max) {
            tree[node_idx].wins += 1;
        }
        if let Some(parent_idx) = tree[node_idx].parent {
            node_idx = parent_idx;
        } else {
            break
        };
    }
}

// returns the winner's snake index
fn playout<const S: usize, const W: usize, const H:usize, const WRAP: bool, const HZSTACK: bool>(tree: &mut Vec<Node<S, W, H, WRAP, HZSTACK>>, node_idx: usize, moves_idx: usize, rng: &mut impl Rng, node_counter: &mut u64) -> i8
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let mut board = tree[node_idx].board.clone();
    let mut moves = match &tree[node_idx].moves {
        Moves::Me(mvs) => {
            let mut tmp = random_move_combination(&board, rng);
            tmp[0] = mvs[moves_idx];
            tmp
        },
        Moves::Enemies(mvs) => {
            let mut tmp = mvs[moves_idx];
            tmp[0] = if let Moves::Me(ref mvs) = tree[tree[node_idx].parent.unwrap()].moves {
                mvs[tree[node_idx].moves_idx]
            } else {
                panic!("parent node of min node does not have me moves");
            };
            tmp
        },
    };
    while !board.is_terminal() {
        *node_counter += 1;
        (board.apply_moves.clone())(&mut board, &moves);
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

fn select_child<const S: usize, const W: usize, const H: usize, const WRAP: bool, const HZSTACK: bool>(tree: &Vec<Node<S, W, H, WRAP, HZSTACK>>, node: usize) -> usize
where [(); (W*H+63)/64]: Sized, [(); hz_stack_len::<HZSTACK, W, H>()]: Sized {
    let parent_visits = tree[node].visits;
    let parent_lower = tree[node].lower_bound;
    let parent_upper = tree[node].upper_bound;
    let parent_max = tree[node].max;

    let mut best_val = 0_f64;
    let mut best_moves_idx = 0;
    for (i, x) in tree[node].children.iter().enumerate() {
        if let Some(child_idx) = x {
            let child = &tree[*child_idx];

            // // this is basically alpha beta pruning
            // if (parent_max && parent_lower >= child.upper_bound) || (!parent_max && parent_upper <= child.lower_bound) {
            //     continue
            // }

            let val = ucb1(parent_visits.into(), child.visits.into(), child.wins.into());
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
