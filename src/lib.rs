#![feature(test, generic_const_exprs, label_break_value)]

#[macro_use]
extern crate lazy_static;
extern crate test;


pub mod bitboard;

#[cfg(not(feature = "mcts"))]
pub mod minimax;

#[cfg(feature = "mcts")]
pub mod uct;

// Public stuff

pub mod api;

pub fn init() {
    #[cfg(all(feature = "tt", not(feature = "mcts")))]
    minimax::ttable::init()
}

#[cfg(feature = "training")]
pub use minimax::set_training_weights;
