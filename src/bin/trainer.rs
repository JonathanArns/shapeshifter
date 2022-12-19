#![feature(test, generic_const_exprs, label_break_value, async_closure, let_chains)]

use axum::{Router, routing::get, routing::post};
use tokio::task;
use std::env;
use rand::Rng;
use rand::distributions::Distribution;
use rand::seq::SliceRandom;
use std::process::Command;
use std::fs::File;
use std::io::{Write, Error};

use shapeshifter::{api, set_training_weights};

#[tokio::main]
async fn main() {
    shapeshifter::init();

    let router = Router::new()
        .route("/0/", get(api::handle_index))
        .route("/0/start", post(api::handle_start))
        .route("/0/end", post(api::handle_end))
        .route("/0/move", post(api::training_handle_move_minimax::<0>))

        .route("/1/", get(api::handle_index))
        .route("/1/start", post(api::handle_start))
        .route("/1/end", post(api::handle_end))
        .route("/1/move", post(api::training_handle_move_minimax::<1>))

        .route("/2/", get(api::handle_index))
        .route("/2/start", post(api::handle_start))
        .route("/2/end", post(api::handle_end))
        .route("/2/move", post(api::training_handle_move_minimax::<2>))

        .route("/3/", get(api::handle_index))
        .route("/3/start", post(api::handle_start))
        .route("/3/end", post(api::handle_end))
        .route("/3/move", post(api::training_handle_move_minimax::<3>));

    let env_port = env::var("PORT").ok();
    let addr = "0.0.0.0:".to_owned() + env_port.as_ref().map(String::as_str).unwrap_or("8080");

    task::spawn(async move {
        axum::Server::bind(&addr.parse().unwrap())
            .serve(router.into_make_service())
            .await
            .unwrap();
    });

    // run genetic algorithm
    let mut gen = 0;
    let mut population = new_population();
    println!("gen {}: running games", gen);
    run_games(&mut population);
    population.sort_by(|a, b| { b.wins.partial_cmp(&a.wins).unwrap() });
    write_generation(&population, gen);
    loop {
        gen += 1;
        population = next_generation(population);
        println!("gen {}: running games", gen);
        run_games(&mut population);
        population.sort_by(|a, b| { b.wins.partial_cmp(&a.wins).unwrap() });
        write_generation(&population, gen);
    }
}

const POPULATION_SIZE: usize = 200;
const GAMES_PER_GENERATION: usize = 20;
const SNAKES_PER_GAME: usize = 2;
const MUTATIONS_PER_GENERATION: usize = 10;
const TOURNAMENT_SIZE: usize = 2;

const NUM_WEIGHTS: usize = 26;
const WEIGHT_RANGES: [(i16, i16); NUM_WEIGHTS] = [
    (0, 100), // turn progression start
    (250, 1500), // turn progression end
    (0, 10), // me health early
    (0, 10), // me health late
    (-10, 0), // lowest enemy health early
    (-10, 0), // lowest enemy health late
     (0, 0), // capped length diff early
     (0, 0), // capped length diff late
     (5, 5), // length diff cap
    (0, 10), // being longer early
    (0, 10), // being longer late
    (0, 10), // food control diff early
    (0, 10), // food control diff late
    (-5, 10), // hazard area diff early
    (-5, 10), // hazard area diff late
     (0, 0), // close area diff early
     (0, 0), // close area diff late
    (0, 10), // non hazard area diff early
    (0, 10), // non hazard area diff late
    (0, 10), // food distance early
    (0, 10), // food distance late
    (0, 30), // tail control diff early
    (0, 30), // tail control diff late
    (0, 20), // odd length early
    (0, 20), // odd length late
     (0, 0), // 
];

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
                weights: (0..NUM_WEIGHTS).map(|i| 
                    if rng.gen_ratio(2, 3) {
                        rng.gen_range(WEIGHT_RANGES[i].0..=WEIGHT_RANGES[i].1)
                    } else {
                        0
                    }.min(WEIGHT_RANGES[i].1).max(WEIGHT_RANGES[i].0)
                ).collect(),
            }
        )
    }
    population
}

fn tournament_select(population: &Vec<Entity>, rng: &mut impl Rng) -> usize {
    let mut winner = rng.gen_range(0..population.len());
    for _ in 1..TOURNAMENT_SIZE {
        let competitor = rng.gen_range(0..population.len());
        if population[competitor].wins > population[winner].wins {
            winner = competitor;
        }
    }
    winner
}

fn crossover(left: &Entity, right: &Entity, rng: &mut impl Rng) -> (Entity, Entity) {
    let middle = rng.gen_range(1..(NUM_WEIGHTS-1));
    let mut l = Entity{
        wins: 0,
        games: 0,
        weights: left.weights[0..middle].to_vec(),
    };
    l.weights.append(&mut right.weights.to_owned()[middle..NUM_WEIGHTS].to_vec());
    let mut r = Entity{
        wins: 0,
        games: 0,
        weights: right.weights[0..middle].to_vec(),
    };
    r.weights.append(&mut left.weights.to_owned()[middle..NUM_WEIGHTS].to_vec());
    (l, r)
}

fn next_generation(mut population: Vec<Entity>) -> Vec<Entity> {
    let mut rng = rand::thread_rng();
    let sum_fitness = population.iter().fold(0, |x, y| x + y.wins) as usize;

    // selection
    population.sort_by(|a, b| { b.wins.partial_cmp(&a.wins).unwrap() });
    let mut next_population = vec![];
    for entity in &population[0..1] {
        next_population.push(Entity{
            wins: 0,
            games: 0,
            weights: entity.weights.clone(),
        })
    }

    // crossover
    while next_population.len() < population.len() {
        let i = tournament_select(&population, &mut rng);
        let j = tournament_select(&population, &mut rng);
        let cross = crossover(&population[i], &population[j], &mut rng);
        next_population.push(cross.0);
        next_population.push(cross.1);
    }
    if next_population.len() > population.len() {
        next_population.remove(next_population.len()-1);
    }

    // mutation
    for _ in 0..MUTATIONS_PER_GENERATION {
        let i = rng.gen_range(0..population.len());
        let j = rng.gen_range(0..NUM_WEIGHTS);
        next_population[i].weights[j] = if rng.gen_ratio(2, 3) {
            rng.gen_range(WEIGHT_RANGES[j].0..=WEIGHT_RANGES[j].1)
        } else {
            0
        }.min(WEIGHT_RANGES[j].1).max(WEIGHT_RANGES[j].0);
    }
    next_population
}

fn write_generation(population: &Vec<Entity>, generation: usize) -> Result<(), std::io::Error> {
    let mut file = File::create(format!("training-output-{}.txt", generation))?;
    for entity in population {
        writeln!(file, "games: {}, wins: {}, weights: {:?}", entity.games, entity.wins, entity.weights)?;
    };
    Ok(())
}

fn run_games(population: &mut Vec<Entity>) {
    let mut rng = rand::thread_rng();
    for _ in 0..GAMES_PER_GENERATION {
        population.shuffle(&mut rng);
        for j in 0..(population.len()/SNAKES_PER_GAME) {
            run_game(&mut population[(j*SNAKES_PER_GAME)..(j*SNAKES_PER_GAME+SNAKES_PER_GAME)]);
        }
    }
}

fn run_game(snakes: &mut [Entity]) {
    let mut weights: Vec<Vec<i16>> = snakes.iter().map(|entity| { return entity.weights.clone() }).collect();
    while weights.len() < 2 {
        // current standard weights
        weights.push(vec![0, 1500, 0, 0, 0, 0, 2, 1, 5, 2, 10, 8, 1, 0, 8, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 5])
    }
    if weights.len() == 2 {
        weights = [weights.clone(), weights.clone()].concat();
    }
    unsafe {
        set_training_weights(weights);
    }
    let cli_output = Command::new("battlesnake")
        .arg("play")
        // game settings
        .arg("-t").arg("10")
        .arg("-g").arg("wrapped")
        .arg("-m").arg("royale")
        // .arg("--hazardDamagePerTurn").arg("100")
        
        // snakes
        .arg("-n").arg("zero")
        .arg("-u").arg("http://localhost:8080/0/")
        .arg("-n").arg("one")
        .arg("-u").arg("http://localhost:8080/1/")
        .arg("-n").arg("zero_")
        .arg("-u").arg("http://localhost:8080/2/")
        .arg("-n").arg("one_")
        .arg("-u").arg("http://localhost:8080/3/")
        .output()
        .expect("failed to run game");
    let output = String::from_utf8_lossy(&cli_output.stderr);
    let mut winner = -1;
    for line in output.lines() {
        if line.contains("Game complete") {
            if line.contains("zero") {
                winner = 0;
            } else if line.contains("one") {
                winner = 1;
            } else if line.contains("two") {
                winner = 2;
            } else if line.contains("three") {
                winner = 3;
            }
        }
    }
    for (i, entity) in snakes.iter_mut().enumerate() {
        entity.games += 1;
        if i as isize == winner {
            entity.wins += 1;
        }
    }
}
