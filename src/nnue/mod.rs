use std::fs;
use serde::Deserialize;
use serde_json;
use crate::minimax::Score;
use crate::bitboard::Bitboard;

use packed_simd::*;

pub mod data;

lazy_static! {
    static ref LAYERS: Vec<Layer> = serde_json::from_reader(fs::File::open("nnue_model.json").unwrap()).unwrap();
}

// ONLY The feature transformer's weights are transposed
#[derive(Deserialize, Debug)]
struct Layer {
    weight: Vec<Vec<i16>>,
    bias: Vec<i16>,
}

const NUM_FEATURES: usize = 4 * 121;
const M: usize = 256;
const K: usize = 32;


pub type Accumulator = [i16; M];

pub fn fresh_accumulator(active_features: &Vec<usize>) -> Accumulator {
    let mut acc = [0; M];
    for i in 0..M {
        acc[i] = LAYERS[0].bias[i];
    }

    for a in active_features {
        for i in 0..M {
            acc[i] += LAYERS[0].weight[*a][i] as i16;
        }
    }
    acc
}

pub fn fresh_accumulator_simd(active_features: &Vec<usize>) -> Accumulator {
    let num_chunks = M / 16;
    let mut regs = Vec::<i16x16>::with_capacity(num_chunks);

    // load bias to registers
    for i in 0..num_chunks {
        regs.push(i16x16::from_slice_unaligned(&LAYERS[0].bias[(i*16)..(i*16+16)]));
    }

    for a in active_features {
        for i in 0..num_chunks {
            regs[i] = regs[i] + i16x16::from_slice_unaligned(&LAYERS[0].weight[*a][(i*16)..(i*16+16)]);
        }
    }

    let mut acc = [0; M];
    for i in 0..num_chunks {
        regs[i].write_to_slice_unaligned(&mut acc[i*16..])
    }
    return acc
}

pub fn update_accumulator<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    acc: &mut Accumulator,
    added_features: &Vec<usize>,
    removed_features: &Vec<usize>,
) {
    for r in removed_features {
        for i in 0..M {
            acc[i] -= LAYERS[0].weight[*r][i];
        }
    }

    for a in added_features {
        for i in 0..M {
            acc[i] += LAYERS[0].weight[*a][i];
        }
    }
}

pub fn update_accumulator_simd<const S: usize, const W: usize, const H: usize, const WRAP: bool>(
    acc: &mut Accumulator,
    added_features: &Vec<usize>,
    removed_features: &Vec<usize>,
) {
    let num_chunks = M / 16;
    let mut regs = Vec::<i16x16>::with_capacity(num_chunks);

    // load bias to registers
    for i in 0..num_chunks {
        regs.push(i16x16::from_slice_unaligned(&acc[(i*16)..(i*16+16)]));
    }

    for r in removed_features {
        for i in 0..num_chunks {
            regs[i] = regs[i] - i16x16::from_slice_unaligned(&LAYERS[0].weight[*r][(i*16)..(i*16+16)]);
        }
    }

    for a in added_features {
        for i in 0..num_chunks {
            regs[i] = regs[i] + i16x16::from_slice_unaligned(&LAYERS[0].weight[*a][(i*16)..(i*16+16)]);
        }
    }

    let mut acc = [0; M];
    for i in 0..num_chunks {
        regs[i].write_to_slice_unaligned(&mut acc[i*16..])
    }
}

fn linear<const I: usize, const O: usize>(input: &[i8; I], output: &mut [i16; O], layer_id: usize) {
    for i in 0..O/16 {
        output[i] = LAYERS[layer_id].bias[i];
    }

    for i in 0..I {
        for j in 0..O {
            // TODO: use more efficient widening mul?
            output[j] = output[j].saturating_add(input[i] as i16 * LAYERS[layer_id].weight[j][i] as i16);
        }
    }

    for i in 0..O {
        output[i] /= 64;
    }
}

fn linear_simd<const I: usize, const O: usize>(input: &[i8; I], output: &mut [i16; O], layer_id: usize) {
    let num_in_chunks = I / 16;
    let num_out_chunks = O / 4;

    for i in 0..num_out_chunks {
        let mut sum0 = i16x16::splat(0);
        let mut sum1 = i16x16::splat(0);
        let mut sum2 = i16x16::splat(0);
        let mut sum3 = i16x16::splat(0);

        for j in 0..num_in_chunks {
            // TODO: deal with i8 instead of i16 for more fast
            let input: i16x16 = i8x16::from_slice_unaligned(&input[(j*16)..(j*16+16)]).cast();
            
            sum0 = sum0 + input * i16x16::from_slice_unaligned(&LAYERS[layer_id].weight[i*4+0][(j*16)..(j*16+16)]);
            sum1 = sum1 + input * i16x16::from_slice_unaligned(&LAYERS[layer_id].weight[i*4+1][(j*16)..(j*16+16)]);
            sum2 = sum2 + input * i16x16::from_slice_unaligned(&LAYERS[layer_id].weight[i*4+2][(j*16)..(j*16+16)]);
            sum3 = sum3 + input * i16x16::from_slice_unaligned(&LAYERS[layer_id].weight[i*4+3][(j*16)..(j*16+16)]);
        }

        let bias = i16x4::from_slice_unaligned(&LAYERS[layer_id].bias[(i*4)..(i*4+4)]);
        
        let mut sums = i16x4::from_slice_unaligned(&[sum0.wrapping_sum(), sum1.wrapping_sum(), sum2.wrapping_sum(), sum3.wrapping_sum()]);
        sums = sums + bias;
        sums = sums >> 6;
        sums.write_to_slice_unaligned(&mut output[(i*4)..]);
    }
}

fn clipped_relu<const I: usize>(input: &[i16; I], output: &mut [i8; I]) {
    for i in 0..I {
        output[i] = (input[i] as i8).max(0).min(127);
    }
}

fn clipped_relu_simd<const I: usize>(input: &[i16; I], output: &mut [i8; I]) {
    let out_chunks = I / 32;
    let zero = i8x32::splat(0);
    for i in 0..out_chunks {
        let in0 = i16x32::from_slice_unaligned(&input[(i*32)..(i*32+32)]);
        let mut out: Simd<[i8; 32]> = in0.cast();
        out = out.max(zero);
        out.write_to_slice_unaligned(&mut output[(i*32)..]);
    }
}

pub fn eval<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+127)/128]: Sized {
    let active_features = data::bitboard_to_active_features(board);
    let accum: Accumulator = fresh_accumulator(&active_features);
    let mut t1: [i8; M] = [0; M];
    clipped_relu(&accum, &mut t1);
    let mut t2: [i16; K] = [0; K];
    linear(&t1, &mut t2, 1);
    let mut t3: [i8; K] = [0; K];
    clipped_relu(&t2, &mut t3);
    let mut out: [i16; 1] = [0];
    linear(&t3, &mut out, 2);
    let model_output = out[0] as f32 / 64.0;
    let score = (model_output / (1.0 - model_output)).ln() * 1.0;
    return score as Score;
}

pub fn eval_simd<const S: usize, const W: usize, const H: usize, const WRAP: bool>(board: &Bitboard<S, W, H, WRAP>) -> Score
where [(); (W*H+127)/128]: Sized {
    let active_features = data::bitboard_to_active_features(board);
    let accum: Accumulator = fresh_accumulator_simd(&active_features);
    let mut t1: [i8; M] = [0; M];
    clipped_relu_simd(&accum, &mut t1);
    let mut t2: [i16; K] = [0; K];
    linear_simd(&t1, &mut t2, 1);
    let mut t3: [i8; K] = [0; K];
    clipped_relu_simd(&t2, &mut t3);
    let mut out: [i16; 1] = [0];
    linear(&t3, &mut out, 2);
    let model_output = out[0] as f32 / 64.0;
    let score = (model_output / (1.0 - model_output)).ln() * 1.0;
    if score as Score != 0 {
        println!("{}", score);
    }
    return score as Score;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
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
    fn bench_eval_simd(b: &mut Bencher) {
        let board = create_board();
        b.iter(|| {
            eval_simd(&board)
        })
    }

    #[test]
    fn test_eval_simd() {
        let board = create_board();
        let x = eval_simd(&board);
        let y = eval(&board);
        assert!(y == x, "{:?} {:?}", y, x);
    }

    #[bench]
    fn bench_refresh_accum_simd(b: &mut Bencher) {
        let board = create_board();
        b.iter(|| {
            let mut active_features = data::bitboard_to_active_features(&board);
            fresh_accumulator_simd(&active_features);
        })
    }
}
