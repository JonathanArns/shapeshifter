#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate lazy_static;

mod types;
mod mailbox;
mod bitboard;
mod minimax;
mod api;

use log::info;
use rocket::config::{Config, Environment};
use std::env;

fn main() {
    println!("{:?}", std::mem::size_of::<bitboard::Bitboard<4>>());
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

    info!("Starting Battlesnake Server at http://{}:{}...", address, port);
    rocket::custom(config)
        .mount("/", routes![api::handle_index, api::handle_start, api::handle_move, api::handle_end])
        .launch();
}
