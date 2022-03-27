#![feature(proc_macro_hygiene, decl_macro, test, generic_const_exprs, label_break_value, stmt_expr_attributes)]

#![allow(incomplete_features)]


#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate lazy_static;
extern crate test;

mod api;
mod bitboard;

#[cfg(not(feature = "mcts"))]
mod minimax;
#[cfg(feature = "mcts")]
mod uct;

use rocket::config::{Config, Environment};
use std::env;

fn main() {
    #[cfg(all(feature = "tt", not(feature = "mcts")))]
    minimax::init();
    let address = "0.0.0.0";
    let env_port = env::var("PORT").ok();
    let env_port = env_port
        .as_ref()
        .map(String::as_str)
        .unwrap_or("8080");
    let port = env_port.parse::<u16>().unwrap();

    env_logger::init();

    let config = Config::build(Environment::Development)
      .address(address)
      .port(port)
      .finalize()
      .unwrap();

    rocket::custom(config)
        .mount("/", routes![api::handle_index, api::handle_start, api::handle_move, api::handle_end])
        .launch();
}
