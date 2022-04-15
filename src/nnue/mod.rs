use std::fs;
use serde::Deserialize;
use serde_json;
use tch::{self, nn::Module};
use crate::minimax::Score;
use crate::bitboard::Bitboard;

pub mod data;

lazy_static! {
    static ref MODEL: tch::CModule = tch::CModule::load("nnue_scripted.pt").expect("missing saved evaluation model");
    static ref LAYERS: Vec<Layer> = serde_json::from_reader(fs::File::open("nnue_model.json").unwrap()).unwrap();
}

#[derive(Deserialize, Debug)]
struct Layer {
    weight: Vec<Vec<f32>>,
    bias: Vec<f32>,
}

const NUM_FEATURES: usize = 4 * 121;
const M: usize = 256;
const K: usize = 32;

pub type Accumulator = [f32; M];

pub fn fresh_accumulator(active_features: &Vec<usize>) -> Accumulator {
    let mut acc = [0.0; M];
    for i in 0..M {
        acc[i] = LAYERS[0].bias[i];
    }

    for a in active_features {
        for i in 0..M {
            acc[i] += LAYERS[0].weight[i][*a];
        }
    }
    acc
}

pub fn update_accumulator<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    acc: &mut Accumulator,
    added_features: &Vec<u16>,
    removed_features: &Vec<u16>,
) {
    todo!()
}

fn linear(input: &[f32], output: &mut [f32], layer_id: usize) {
    for i in 0..output.len() {
        output[i] = LAYERS[layer_id].bias[i];
    }

    for i in 0..input.len() {
        for j in 0..output.len() {
            output[j] += input[i] * LAYERS[layer_id].weight[j][i];
        }
    }
}

fn clipped_relu(input: &[f32], output: &mut [f32]) {
    for i in 0..input.len() {
        output[i] = input[i].max(0.0).min(1.0);
    }
}

pub fn eval<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+127)/128]: Sized, [(); W*H*4]: Sized {
    let mut active_features = data::bitboard_to_active_features(board);
    let mut accum: Accumulator = fresh_accumulator(&active_features);
    let mut t1: [f32; M] = [0.0; M];
    clipped_relu(&accum, &mut t1);
    let mut t2: [f32; K] = [0.0; K];
    linear(&t1, &mut t2, 1);
    let mut t3: [f32; K] = [0.0; K];
    clipped_relu(&t2, &mut t3);
    let mut out: [f32; 1] = [0.0];
    linear(&t3, &mut out, 2);
    return out[0] as Score
}




pub fn eval_jit<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+127)/128]: Sized, [(); W*H*4]: Sized {
    let input = tch::Tensor::of_slice(&data::bitboard_to_slice(board));
    let x = MODEL.forward(&input);
    println!("{:?}", x);
    return 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
    use crate::bitboard::move_gen;
    use test::Bencher;

    fn c(x: usize, y: usize) -> api::Coord {
        api::Coord{x, y}
    }

    fn create_board() -> Bitboard<4, 11, 11, true> {
        let mut ruleset = std::collections::HashMap::new();
        ruleset.insert("name".to_string(), serde_json::Value::String("wrapped".to_string()));
        let state = api::GameState{
            game: api::Game{ id: "".to_string(), timeout: 100, ruleset },
            turn: 157,
            you: api::Battlesnake{
                id: "a".to_string(),
                name: "a".to_string(),
                shout: None,
                squad: None,
                health: 100,
                length: 11,
                head: c(5,2),
                body: vec![c(5,2), c(5,1), c(6, 1), c(7,1), c(7,2), c(8,2), c(8,3), c(7,3), c(7,4), c(6,4), c(6,4)],
            },
            board: api::Board{
                height: 11,
                width: 11,
                food: vec![c(3,10), c(6,0), c(10,1), c(0,10), c(3,0), c(9,5), c(10,3), c(9,4), c(8,4), c(8,10), c(0,6)],
                hazards: vec![],
                snakes: vec![
                    api::Battlesnake{
                        id: "a".to_string(),
                        name: "a".to_string(),
                        shout: None,
                        squad: None,
                        health: 100,
                        length: 11,
                        head: c(5,2),
                        body: vec![c(5,2), c(5,1), c(6, 1), c(7,1), c(7,2), c(8,2), c(8,3), c(7,3), c(7,4), c(6,4), c(6,4)],
                    },  
                    api::Battlesnake{
                        id: "b".to_string(),
                        name: "b".to_string(),
                        shout: None,
                        squad: None,
                        health: 95,
                        length: 12,
                        head: c(3,4),
                        body: vec![c(3,4), c(2,4), c(2,5), c(3, 5), c(3,6), c(3,7), c(3,8), c(4,8), c(4,7), c(4,6), c(4,5), c(4,4)],
                    },  
                ],
            },
        };
        Bitboard::<4, 11, 11, true>::from_gamestate(state)
    }
    
    #[bench]
    fn bench_eval(b: &mut Bencher) {
        let board = create_board();
        b.iter(|| {
            eval(&board)
        })
    }

    #[bench]
    fn bench_eval_jit(b: &mut Bencher) {
        let board = create_board();
        b.iter(|| {
            eval_jit(&board)
        })
    }
}
