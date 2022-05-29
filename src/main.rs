#![feature(proc_macro_hygiene, decl_macro, test, generic_const_exprs, label_break_value, stmt_expr_attributes, stdsimd)]

#![allow(incomplete_features)]


#[macro_use]
extern crate lazy_static;
extern crate test;

mod api;
mod bitboard;

#[cfg(not(feature = "mcts"))]
mod minimax;
#[cfg(feature = "mcts")]
mod uct;

use axum::{Router, routing::get, routing::post};
use std::env;

#[tokio::main]
async fn main() {
    #[cfg(all(feature = "tt", not(feature = "mcts")))]
    minimax::init();

    let router = Router::new()
        .route("/", get(api::handle_index))
        .route("/start", post(api::handle_start))
        .route("/end", post(api::handle_end))
        .route("/move", post(api::handle_move));

    let env_port = env::var("PORT").ok();
    let env_port = env_port
        .as_ref()
        .map(String::as_str)
        .unwrap_or("8080");

    axum::Server::bind(&("0.0.0.0:".to_owned() + env_port).parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}
