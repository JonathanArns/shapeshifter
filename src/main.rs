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
use tracing;
use tower_http::trace::TraceLayer;
use tracing_subscriber::prelude::*;
use tracing_honeycomb;
use std::env;

#[tokio::main]
async fn main() {
    #[cfg(all(feature = "tt", not(feature = "mcts")))]
    minimax::init();

    // setup tracing subscriber
    let subscriber = tracing_subscriber::Registry::default() // provide underlying span data store
        .with(tracing_subscriber::filter::LevelFilter::DEBUG) // filter out low-level debug tracing (eg tokio executor)
        .with(tracing_subscriber::fmt::Layer::default()); // log to stdout

    // add honeycomb layer to subscriber if the key is in the environment
    // and set as default tracing subscriber
    if let Ok(key) = env::var("HONEYCOMB_KEY") {
        let honeycomb_config = libhoney::Config {
            options: libhoney::client::Options {
                api_key: key,
                dataset: "battlesnake".to_string(),
                ..libhoney::client::Options::default()
            },
            transmission_options: libhoney::transmission::Options::default(),
        };
        let honeycomb_subscriber = subscriber.with(tracing_honeycomb::new_honeycomb_telemetry_layer("shapeshifter", honeycomb_config));
        tracing::subscriber::set_global_default(honeycomb_subscriber).expect("setting global default tracing subscriber failed");
        println!("Honeycomb subscriber initialized");
    } else {
        tracing::subscriber::set_global_default(subscriber).expect("setting global default tracing subscriber failed");
    }

    let router = Router::new()
        .route("/", get(api::handle_index))
        .route("/start", post(api::handle_start))
        .route("/end", post(api::handle_end))
        .route("/move", post(api::handle_move))
        .layer(TraceLayer::new_for_http());

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
