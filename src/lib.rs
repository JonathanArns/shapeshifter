#![feature(test, generic_const_exprs, let_chains)]

#[macro_use]
extern crate lazy_static;
extern crate test;


mod bitboard;

#[cfg(not(feature = "mcts"))]
mod minimax;

// #[cfg(feature = "mcts")]
mod uct;
mod mcts;

// Public stuff

pub mod api;

pub fn init() {
    #[cfg(all(feature = "tt", not(feature = "mcts")))]
    minimax::init()
}

#[cfg(feature = "training")]
pub use minimax::set_training_weights;
