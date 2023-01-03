#![feature(test, generic_const_exprs, async_closure, let_chains)]

use shapeshifter::api;
use std::time::Instant;
use dfdx::{
    prelude::*,
    optim::{Momentum, Sgd, SgdConfig},
    data::SubsetIterator,
    flush_denormals_to_zero,
};
use indicatif::ProgressBar;
use rand::prelude::{Rng, SeedableRng, StdRng};
use std::fs::File;
use std::io::{prelude::*, BufReader};
use serde_json;

const BATCH: usize = 64;
const INPUT: usize = 11*11*7;
const EPOCHS: usize = 90;

type EvalNetwork = (
    (Linear<INPUT, 16>, ReLU),
    (Linear<16, 16>, ReLU),
    Linear<16, 1>,
);

struct EvalDataset {
    x: Vec<[f32; INPUT]>,
    y: Vec<f32>,
}

impl EvalDataset {
    fn load(path: &str) -> Self {
        let mut dataset = Self{x: Vec::with_capacity(10000), y: Vec::with_capacity(10000)};
        let file = File::open(path).expect("coudln't open file");
        let mut min_y: f32 = 1000000.0;
        let mut max_y: f32 = 0.0;
        for line in BufReader::new(file).lines() {
            let line = line.unwrap();
            let (y, x) = line.split_once(";").unwrap();
            let y = y.parse::<f32>().unwrap()+10000.0;
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            dataset.y.push(y);
            let x_vec: Vec<f32> = serde_json::from_str(x).unwrap();
            let mut x = [0.0; INPUT];
            x.copy_from_slice(&x_vec);
            dataset.x.push(x);
        }
        // z normalization for y
        for i in 0..dataset.y.len() {
            dataset.y[i] = (dataset.y[i] - min_y) / (max_y - min_y);
        }
        dataset
    }

    fn len(&self) -> usize {
        self.y.len()
    }

    fn get_batch<const B: usize>(
        &self,
        idxs: [usize; B],
    ) -> (Tensor2D<B, INPUT>, Tensor2D<B, 1>) {
        let mut input = Tensor2D::zeros();
        let mut lbl = Tensor2D::zeros();
        let input_data = input.mut_data();
        let lbl_data = lbl.mut_data();
        for (batch_i, &input_idx) in idxs.iter().enumerate() {
            input_data[batch_i].copy_from_slice(&self.x[input_idx]);
            lbl_data[batch_i][0] = self.y[input_idx];
        }
        (input, lbl)
    }
}

fn main() {
    flush_denormals_to_zero();
    let mut rng = StdRng::seed_from_u64(0);

    // init model
    let mut model: EvalNetwork = Default::default();
    // model.load("./test-model.npz").expect("coudln't load model");
    model.reset_params(&mut rng);

    // init optimizer
    let mut opt:  Adam<EvalNetwork> = Default::default();

    // load dataset
    let dataset = EvalDataset::load("./data/standard_2-11x11-NOWRAP-NOSTACK_features_train.csv");
    let test_dataset = EvalDataset::load("./data/standard_2-11x11-NOWRAP-NOSTACK_features_test.csv");

    // training loop
    for i_epoch in 0..EPOCHS {
        // learning rate decay
        if i_epoch == 30 {
            opt.cfg.lr *= 0.1;
        }
        let mut total_epoch_loss = 0.0;
        let mut num_batches = 0;
        let start = Instant::now();
        let bar = ProgressBar::new(dataset.len() as u64);
        for (x, y_true) in SubsetIterator::<BATCH>::shuffled(dataset.len(), &mut rng)
            .map(|i| dataset.get_batch(i))
        {
            let y = model.forward_mut(x.traced());
            let loss = mse_loss(y, y_true);

            total_epoch_loss += loss.data();
            num_batches += 1;
            bar.inc(BATCH as u64);

            let gradients = loss.backward();
            opt.update(&mut model, gradients);
        }
        let dur = Instant::now() - start;
        bar.finish_and_clear();

        // test
        let mut test_num_batches = 0;
        let mut total_test_loss = 0.0;
        for (x, y_true) in SubsetIterator::<BATCH>::shuffled(test_dataset.len(), &mut rng)
            .map(|i| test_dataset.get_batch(i))
        {
            let y = model.forward(x);
            let loss = mse_loss(y, y_true);

            total_test_loss += loss.data();
            test_num_batches += 1;
        }
        println!(
            "Epoch {i_epoch} in {:?} ({:.3} batches/s): avg training loss {:.5}, avg test loss {:.5}",
            dur,
            num_batches as f32 / dur.as_secs_f32(),
            BATCH as f32 * total_epoch_loss / num_batches as f32,
            BATCH as f32 * total_test_loss / test_num_batches as f32,
        );
        if (i_epoch+1) % 20 == 0 {
            model.save("./test-model.npz").expect("Could not save model");
        }
    }
    model.save("./test-model.npz").expect("Could not save model");

}
