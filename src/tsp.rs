use crate::utils::{DistanceMatrix, PointOfInterest, subsets, iterate_bits_as_indices};
use std::io::Write;

pub struct TSPLookup(Vec<f32>);
impl TSPLookup {
    pub fn new(dm: &DistanceMatrix, poi: &Vec<PointOfInterest>) -> Self {
        let n = dm.durations.len();
        let mut g: Vec<f32> = vec![f32::INFINITY; (1usize << (n - 1)) * (n - 1)];
        let get_idx = |subset: usize, node: usize| subset * (n - 1) + node;
        for k in 0..(n - 1) {
            g[get_idx(1 << k, k)] = dm.durations[0][k + 1];
        }

        for size in 2..n {
            println!(
                "\x1B[1A\x1B[2KPath computation: [{}{}]",
                "#".repeat(size - 2),
                ".".repeat(n - size - 1)
            );
            std::io::stdout().flush().unwrap();
            for subset in subsets(n - 1, size) {
                for k in iterate_bits_as_indices(&subset) {
                    let v = iterate_bits_as_indices(&subset)
                        .filter(|&x| x != k)
                        .map(|x| {
                            g[get_idx((subset ^ (1 << k)).try_into().unwrap(), x)]
                                + dm.durations[x + 1][k + 1]
                        })
                        .fold(f32::INFINITY, |a, b| a.min(b));
                    g[get_idx(subset.try_into().unwrap(), k)] = v;
                }
            }
        }

        let result: Vec<_> = (0..(1u32 << (n - 1)))
            .map(|subset| {
                iterate_bits_as_indices(&subset)
                    .map(|x| g[get_idx(subset.try_into().unwrap(), x)] + dm.durations[x + 1][0])
                    .fold(f32::INFINITY, |a, b| a.min(b))
                    + iterate_bits_as_indices(&subset)
                        .map(|x| poi[x + 1].duration * 60.0)
                        .sum::<f32>()
            })
            .collect();

        TSPLookup(result)
    }

    pub fn get(&self, subset: &u32) -> f32 {
        if *subset == 0 {
            0.0
        } else {
            self.0[*subset as usize]
        }
    }
}
