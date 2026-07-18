use std::collections::HashMap;

use crate::utils::DistanceMatrix;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TSPLookup(HashMap<u32, f32>);
impl TSPLookup {
    pub fn new(dm: &DistanceMatrix) -> Self {
        let n = dm.distances.len();
        let mut g: HashMap<(u32, usize), f32> = HashMap::new();
        for k in 0..(n - 1) {
            g.insert((1u32 << k, k), dm.distances[0][k + 1]);
        }

        for size in 2..n {
            println!("Size {}", size);
            for subset in subsets(n - 1, size) {
                for k in iterate_bits_as_indices(&subset) {
                    let v = iterate_bits_as_indices(&subset)
                        .filter(|&x| x != k)
                        .map(|x| {
                            g.get(&(subset ^ (1 << k), x)).unwrap() + dm.distances[x + 1][k + 1]
                        })
                        .fold(f32::INFINITY, |a, b| a.min(b));
                    g.insert((subset, k), v);
                }
            }
        }

        let result: Vec<_> = (0..(1u32 << (n - 1)))
            .map(|subset| {
                (
                    subset,
                    iterate_bits_as_indices(&subset)
                        .map(|x| g.get(&(subset, x)).unwrap() + dm.distances[x + 1][0])
                        .fold(f32::INFINITY, |a, b| a.min(b)),
                )
            })
            .collect();

        TSPLookup(HashMap::from_iter(result))
    }

    pub fn get(&self, subset: &u32) -> f32 {
        *self.0.get(subset).unwrap()
    }
}

fn subsets(size: usize, number: usize) -> impl Iterator<Item = u32> {
    let limit = 1u32 << size;
    let mut mask = if number == 0 { 0 } else { (1u32 << number) - 1 };
    std::iter::from_fn(move || {
        if mask >= limit {
            return None;
        }
        let result = mask as u32;
        if number == 0 {
            mask = limit;
        } else {
            let c = mask & mask.wrapping_neg();
            let r = mask + c;
            mask = (((r ^ mask) >> 2) / c) | r;
        }
        Some(result)
    })
}

fn iterate_bits_as_indices(num: &u32) -> impl Iterator<Item = usize> {
    let mut mask = *num;
    std::iter::from_fn(move || {
        while mask != 0 {
            let i = mask.trailing_zeros();
            mask &= mask - 1;
            return Some(i as usize);
        }
        None
    })
}
