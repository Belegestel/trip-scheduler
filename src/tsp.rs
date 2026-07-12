use crate::solver::Solver;
use crate::utils::DistanceMatrix;
use anyhow::Result;
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, atomic::AtomicU64};

#[derive(Serialize, Deserialize)]
pub struct TSPLookup(HashMap<Vec<usize>, f32>);
impl TSPLookup {
    pub fn from_file(filename: String) -> Result<Self> {
        let cached = std::fs::read_to_string(filename)?;
        let data: TSPLookup = serde_json::from_str(&cached)?;
        println!("Using cached TSP");
        Ok(data)
    }
    fn tsp_bnb(
        dm: &DistanceMatrix,
        current: usize,
        origin: usize,
        remaining: u64,
        cost: f32,
        best: &mut f32,
    ) {
        if remaining == 0 {
            let total = cost + dm.durations[current][origin];
            *best = best.min(total);
            return;
        }
        if cost >= *best {
            return;
        }
        let n = dm.durations.len() - 1;
        let mut candidates: Vec<_> = (0..n).filter(|i| remaining & (1 << i) != 0).collect();
        candidates.sort_by(|a, b| {
            dm.durations[current][*a]
                .partial_cmp(&dm.durations[current][*b])
                .unwrap()
        });
        for next in candidates {
            let bit = 1u64 << next;
            Self::tsp_bnb(
                dm,
                next,
                origin,
                remaining & !bit,
                cost + dm.durations[current][next],
                best,
            );
        }
    }
    fn tsp_hk(dm: &DistanceMatrix, origin: usize, subset: u64) -> f32 {
        let n = dm.durations.len();
        let cities = n - 1;
        let full_mask = subset as usize;
        let states = 1usize << cities;
        let inf = f32::INFINITY;

        let mut dp = vec![inf; states * n];

        #[inline(always)]
        fn idx(mask: usize, last: usize, n: usize) -> usize {
            mask * n + last
        }

        dp[idx(0, origin, n)] = 0.0;

        let mut mask = 0usize;
        while mask <= full_mask {
            if mask & !full_mask != 0 {
                mask += 1;
                continue;
            }

            for last in 0..n {
                let cur = dp[idx(mask, last, n)];

                if cur == inf {
                    continue;
                }

                let mut remaining = full_mask & !mask;

                while remaining != 0 {
                    let bit = remaining & remaining.wrapping_neg();
                    let next = bit.trailing_zeros() as usize;

                    remaining ^= bit;

                    let next_mask = mask | bit;

                    let cost = cur + dm.durations[last][next];

                    let entry = &mut dp[idx(next_mask, next, n)];

                    if cost < *entry {
                        *entry = cost;
                    }
                }
            }

            mask += 1;
        }

        let mut best = inf;

        for last in 0..cities {
            let cost = dp[idx(full_mask, last, n)] + dm.durations[last][origin];

            if cost < best {
                best = cost;
            }
        }

        best
    }

    pub fn calculate_parallel(dm: &DistanceMatrix) -> Self {
        println!("Calculating TSP...");
        let n = dm.durations.len() - 1;
        let total_cases = 1u64 << n;
        let origin_idx = n;

        let completed = Arc::new(AtomicU64::new(0));
        let completed_clone = Arc::clone(&completed);

        let res: HashMap<Vec<usize>, f32> = (0..total_cases)
            .into_par_iter()
            .map(|mask| {
                let chosen: Vec<usize> = (0..n).filter(|i| (mask & (1 << i)) != 0).collect();

                let best_time = Self::tsp_hk(dm, origin_idx, mask);

                // Self::tsp_bnb(dm, origin_idx, origin_idx, mask, 0.0, &mut best_time);

                let done = completed_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                let done_fmt = ((1000.0 * done as f32) / total_cases as f32).round() as f32 / 10.0;
                if done % 1000 == 0 {
                    print!(
                        "\r\x1b[2KProgress: {:.0}% ({} / {})",
                        done_fmt, done, total_cases
                    );
                    std::io::stdout().flush().unwrap();
                }

                (chosen, best_time)
            })
            .collect();

        Self(res)
    }
    pub fn save(&self) -> Result<()> {
        let file = std::fs::File::create("data/TSP.json")?;
        serde_json::to_writer_pretty(file, &self)?;
        Ok(())
    }
}

pub struct TSPCache {
    costs: Vec<f32>,
    n: usize,
}
impl TSPCache {
    pub fn get(&self, included: Vec<bool>) -> f32 {
        let mut mask = 0usize;

        for (i, &v) in included.iter().enumerate() {
            if v {
                mask |= 1 << i;
            }
        }
        self.costs[mask]
    }
    pub fn build_cache(dm: &DistanceMatrix) -> Self {
        let n = dm.durations.len() - 1;
        let origin = n;

        let states = 1usize << n;
        let inf = f32::INFINITY;

        let mut dp = vec![inf; states * n];

        #[inline(always)]
        fn idx(mask: usize, last: usize, n: usize) -> usize {
            mask * n + last
        }

        // Paths starting from origin to one city
        for city in 0..n {
            let mask = 1usize << city;
            dp[idx(mask, city, n)] = dm.durations[origin][city];
        }

        // Held-Karp
        for mask in 1..states {
            let mut remaining = mask;

            while remaining != 0 {
                let bit = remaining & remaining.wrapping_neg();
                let last = bit.trailing_zeros() as usize;

                remaining ^= bit;

                let current = dp[idx(mask, last, n)];

                if current == inf {
                    continue;
                }

                let mut nexts = (!mask) & (states - 1);

                while nexts != 0 {
                    let next_bit = nexts & nexts.wrapping_neg();
                    let next = next_bit.trailing_zeros() as usize;

                    nexts ^= next_bit;

                    let next_mask = mask | next_bit;

                    let candidate = current + dm.durations[last][next];

                    let entry = &mut dp[idx(next_mask, next, n)];

                    if candidate < *entry {
                        *entry = candidate;
                    }
                }
            }
        }

        let mut costs = vec![0.0; states];
        costs[0] = 0.0;
        for mask in 1..states {
            let mut best = inf;
            let mut cities = mask;
            while cities != 0 {
                let bit = cities & cities.wrapping_neg();
                let last = bit.trailing_zeros() as usize;
                cities ^= bit;
                let cost = dp[idx(mask, last, n)] + dm.durations[last][origin];
                if cost < best {
                    best = cost;
                }
            }
            costs[mask] = best;
        }
        TSPCache { costs, n }
    }
}
