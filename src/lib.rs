#![feature(test, generic_const_exprs, let_chains)]

#[macro_use]
extern crate lazy_static;
extern crate test;

pub mod bitboard;
pub mod wire_rep;
pub mod api;
pub mod minimax;
pub mod uct;

pub fn init() {
    #[cfg(feature = "tt")]
    minimax::init()
}

#[cfg(feature = "training")]
pub use minimax::set_training_weights;
