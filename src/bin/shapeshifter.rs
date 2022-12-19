#![feature(test, generic_const_exprs, let_chains)]

use axum::{Router, routing::get, routing::post};
use tower_http::trace::TraceLayer;
use tracing;
use log_panics;
use tracing_log::LogTracer;
use tracing_subscriber::{Registry, layer::SubscriberExt};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tonic::metadata::MetadataMap;
use std::env;

use shapeshifter::api;

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    // set up tracing subscriber
    if let Ok(key) = env::var("HONEYCOMB_KEY") {
        let mut map = MetadataMap::new();
        map.insert("x-honeycomb-team", key.parse().unwrap());

        let honeycomb_tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(opentelemetry_otlp::new_exporter()
                .tonic()
                .with_protocol(opentelemetry_otlp::Protocol::Grpc)
                .with_endpoint("https://api.honeycomb.io")
                .with_metadata(map)
            )
            .with_trace_config(
                opentelemetry::sdk::trace::config().with_resource(
                    opentelemetry::sdk::Resource::new(vec![KeyValue::new(
                        "service.name",
                        "shapeshifter",
                    )])
                )
            )
            .install_batch(opentelemetry::runtime::Tokio)
            .expect("setting up honeycomb tracer failed");

        // Create a tracing layer with the configured tracer
        let honeycomb_telemetry = tracing_opentelemetry::layer().with_tracer(honeycomb_tracer);

        // add to the subscriber and set it as global default
        let honeycomb_subscriber = Registry::default().with(tracing_subscriber::filter::LevelFilter::DEBUG).with(honeycomb_telemetry);
        tracing::subscriber::set_global_default(honeycomb_subscriber).expect("setting global default tracing subscriber failed");
        println!("honeycomb subscriber initialized");
    } else {
        let stdout_subscriber = Registry::default().with(tracing_subscriber::filter::LevelFilter::INFO).with(tracing_subscriber::fmt::Layer::default());
        tracing::subscriber::set_global_default(stdout_subscriber).expect("setting global default tracing subscriber failed");
    }

    // setup so that panics will be recorded
    LogTracer::init().unwrap();
    log_panics::init();

    shapeshifter::init();

    let router = Router::new()
        .route("/", get(api::handle_index))
        .route("/start", post(api::handle_start))
        .route("/end", post(api::handle_end))
        .route("/move", post(api::handle_move_minimax))

        .route("/mcts/", get(api::handle_index))
        .route("/mcts/start", post(api::handle_start))
        .route("/mcts/end", post(api::handle_end))
        .route("/mcts/move", post(api::handle_move_mcts))
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
