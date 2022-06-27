#![feature(test, generic_const_exprs, label_break_value)]

use axum::{Router, routing::get, routing::post};
use std::env;
use rand::Rng;
use rand::seq::SliceRandom;
use std::process::Command;
use tower_http::map_request_body::MapRequestBodyLayer;

use shapeshifter::{api, set_training_weights};

#[tokio::main]
async fn main() {
    // // set up tracing subscriber
    // let subscriber = Registry::default().with(tracing_subscriber::filter::LevelFilter::INFO);

    // // add honeycomb layer to subscriber if the key is in the environment
    // // and set as default tracing subscriber
    // if let Ok(key) = env::var("HONEYCOMB_KEY") {
    //     let mut map = MetadataMap::new();
    //     map.insert("x-honeycomb-team", key.parse().unwrap());
    //     map.insert("x-honeycomb-dataset", "test".parse().unwrap());

    //     let honeycomb_tracer = opentelemetry_otlp::new_pipeline()
    //         .tracing()
    //         .with_exporter(opentelemetry_otlp::new_exporter()
    //             .tonic()
    //             .with_protocol(opentelemetry_otlp::Protocol::Grpc)
    //             .with_endpoint("https://api.honeycomb.io")
    //             .with_metadata(map)
    //         )
    //         .install_batch(opentelemetry::runtime::Tokio)
    //         .expect("setting up honeycomb tracer failed");

    //     // Create a tracing layer with the configured tracer
    //     let honeycomb_telemetry = tracing_opentelemetry::layer().with_tracer(honeycomb_tracer);

    //     // add to the subscriber and set it as global default
    //     let honeycomb_subscriber = subscriber.with(honeycomb_telemetry);
    //     tracing::subscriber::set_global_default(honeycomb_subscriber).expect("setting global default tracing subscriber failed");
    //     println!("honeycomb subscriber initialized");
    // } else {
    //     let stdout_subscriber = subscriber.with(tracing_subscriber::fmt::Layer::default());
    //     tracing::subscriber::set_global_default(stdout_subscriber).expect("setting global default tracing subscriber failed");
    // }

    shapeshifter::init();

    let router = Router::new()
        .route("/0/", get(api::handle_index))
        .route("/0/start", post(api::handle_start))
        .route("/0/end", post(api::handle_end))
        .route("/0/move", post(api::handle_move::<0>))

        .route("/1/", get(api::handle_index))
        .route("/1/start", post(api::handle_start))
        .route("/1/end", post(api::handle_end))
        .route("/1/move", post(api::handle_move::<1>))

        .route("/2/", get(api::handle_index))
        .route("/2/start", post(api::handle_start))
        .route("/2/end", post(api::handle_end))
        .route("/2/move", post(api::handle_move::<2>))

        .route("/3/", get(api::handle_index))
        .route("/3/start", post(api::handle_start))
        .route("/3/end", post(api::handle_end))
        .route("/3/move", post(api::handle_move::<3>));

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

const NUM_WEIGHTS: usize = 24;
const POPULATION_SIZE: usize = 100;
const GAMES_PER_GENERATION: usize = 100;
const SNAKES_PER_GAME: usize = 4;

struct Entity {
    weights: Vec<i16>,
    games: u16,
    wins: u16,
}

fn new_population() -> Vec<Entity> {
    let mut rng = rand::thread_rng();
    let mut population = vec![];
    for i in 0..(POPULATION_SIZE+(POPULATION_SIZE%SNAKES_PER_GAME)) {
        population.push(
            Entity{
                games: 0,
                wins: 0,
                weights: (0..NUM_WEIGHTS).map(|_| rng.gen_range(-5, 5)).collect(),
            }
        )
    }
    population
}

fn next_generation(population: Vec<Entity>) -> Vec<Entity> {
    todo!()
}

fn run_games(population: Vec<Entity>) {
    let mut rng = rand::thread_rng();
    for i in 0..GAMES_PER_GENERATION {
        population.shuffle(&mut rng);
        for j in 0..(population.len()/4) {
            let snakes = vec![
                population[j+0],
                population[j+1],
                population[j+2],
                population[j+3],
            ];
            run_game(snakes);
        }
    }
}

fn run_game(snakes: Vec<Entity>) {
    let weights = snakes.map(|entity| { return entity.weights.clone() });
    unsafe {
        set_training_weights(weights);
    }
    let cli_output = Command::new("battlesnake play")
        .arg("-t 10")
        .arg("-n zero")
        .arg("-u http://localhost:8080/0/")
        .arg("-n one")
        .arg("-u http://localhost:8080/1/")
        .arg("-n two")
        .arg("-u http://localhost:8080/2/")
        .arg("-n three")
        .arg("-u http://localhost:8080/3/")
        .output()
        .expect("failed to run game");
    let stdout = String::from_utf8_lossy(&cli_output.stdout);
    let winner = for line in stdout.lines() {
        if line.contains("Game complete") {
            if line.contains("zero") {
                break 0;
            } else if line.contains("one") {
                break 1;
            } else if line.contains("two") {
                break 2;
            } else if line.contains("three") {
                break 3;
            } else if line.contains("draw") {
                break -1;
            }
        }
        -1
    };
    for (i, entity) in snakes.iter().enumerate() {
        entity.games += 1;
        if i == winner {
            entity.wins += 1;
        }
    }
}
