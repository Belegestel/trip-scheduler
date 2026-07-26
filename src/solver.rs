use crate::{
    tsp::TSPLookup,
    utils::{DistanceMatrix, PointOfInterest, iterate_bits_as_indices, subsets},
};
use anyhow::Result;
use rand::{rng, seq::index};
use rayon::*;
use std::collections::HashMap;

pub struct Solver {
    pub matrix: DistanceMatrix,
    pub mandatory: Vec<bool>,
    pub local_durations: Vec<f32>,
    pub day_weights: Vec<f32>,
    pub best_solutions: HashMap<u16, u64>, // Optional count vs assignment
    tsp: TSPLookup,
    bases: Vec<u32>,
    base_weights: Vec<u32>,
    mandatory_idx: Vec<usize>,
    optional_idx: Vec<usize>,
    sequence_len: usize,
}
impl Solver {
    pub fn new(
        distance_matrix: DistanceMatrix,
        poi: &Vec<PointOfInterest>,
        day_weights: Vec<f32>,
        tsp: TSPLookup,
    ) -> Self {
        Self {
            matrix: distance_matrix,
            mandatory: poi.iter().map(|x| x.obligatory).collect(),
            local_durations: poi.iter().map(|x| x.duration).collect(),
            day_weights,
            best_solutions: HashMap::new(),
            tsp,
            bases: vec![],
            base_weights: vec![],
            mandatory_idx: vec![],
            optional_idx: vec![],
            sequence_len: 0,
        }
    }

    fn solution_to_assignment(&self, solution: u64) -> impl Iterator<Item = u32> {
        // Iterates over assignments for every day
        let mut masks: Vec<u32> = vec![0; self.day_weights.len()];
        for (i, (&base, &weight)) in self.bases.iter().zip(&self.base_weights).enumerate() {
            let v = (solution / (weight as u64)) % (base as u64);
            if (v as usize) < masks.len() {
                masks[v as usize] |= 1 << i;
            }
        }
        masks.into_iter()
    }

    pub fn run(&mut self) {
        // Constants
        let UPPER_BOUND_RANDOM_SIZE = 1_000_000;

        let mut local_rng = rng();
        self.bases = self
            .mandatory
            .iter()
            .skip(1)
            .map(|x| self.day_weights.len() + { if *x { 0 } else { 1 } })
            .map(|x| x as u32)
            .collect();
        self.base_weights = self.bases.iter().scan(1, |a, b| Some((*a) * (b))).collect();
        // Computation of best distribution consisting only
        // of mandatory locations
        self.mandatory_idx = self
            .mandatory
            .iter()
            .skip(1)
            .enumerate()
            .filter(|(_, x)| **x)
            .map(|(idx, _)| idx)
            .collect();
        self.optional_idx = self
            .mandatory
            .iter()
            .skip(1)
            .enumerate()
            .filter(|(_, x)| !**x)
            .map(|(idx, _)| idx)
            .collect();
        self.sequence_len = self.mandatory_idx.len() + self.optional_idx.len();
        let days_n = self.day_weights.len();
        let total_mandatory_assignments = days_n.pow(self.mandatory_idx.len() as u32);

        // Pre-bound
        let mut best = f32::INFINITY;
        let mut best_solution = 0;

        for solution in index::sample(
            &mut local_rng,
            total_mandatory_assignments as usize,
            UPPER_BOUND_RANDOM_SIZE.min(total_mandatory_assignments) as usize,
        ) {
            let solution: u64 = (solution as u64)
                + self
                    .optional_idx
                    .iter()
                    .map(|x| (self.base_weights[*x] as u64) * days_n as u64)
                    .sum::<u64>();
            let val = self.evaluate_solution(&solution);
            if val < best {
                best = val;
                best_solution = solution;
            }
            break;
        }
        println!("Best upper bound: {}", best);

        // Branch'n'bound
        self.bnb(&mut best, &mut best_solution, 0);
        println!("Best: {}", best);
    }

    fn dfs(
        &self,
        upper_bound: &mut f32,
        best_solution: &mut u64,
        subset: &u32,
        solution: u64,
        depth: usize,
    ) {
        let score = self.partial_evaluate_solution(&solution, depth);
        if score >= *upper_bound {
            // If worse than current best, stop recursion
            return;
        }
        if depth == self.sequence_len {
            // Better than current best, save and stop recursion
            // because we've already reached the end
            *upper_bound = score;
            *best_solution = solution;
            return;
        }

        // Increase by one and recursion
        if subset & (1u32 << depth) == 0 {
            self.dfs(upper_bound, best_solution, subset, solution, depth + 1);
        } else {
            for i in 0..self.bases[depth] {
                self.dfs(
                    upper_bound,
                    best_solution,
                    subset,
                    solution + (self.base_weights[depth] * i) as u64,
                    depth + 1,
                );
            }
        }
    }

    fn bnb(&self, upper_bound: &mut f32, best_solution: &mut u64, optional_count: u8) {
        let _ = subsets(self.optional_idx.len(), optional_count.into()).for_each(|mut subset| {
            println!("Starting subsets");
            let max_solution_val = self
                .base_weights
                .iter()
                .zip(&self.bases)
                .map(|(weight, base)| weight * (base - 1))
                .map(|x| x as u64)
                .sum();

            subset = subset << self.mandatory_idx.len();
            let mut solution = 0;
            solution += iterate_bits_as_indices(&subset)
                .map(|x| self.base_weights[x] * (self.bases[x] - 1))
                .map(|x| x as u64)
                .sum::<u64>();

            let mut depth = 0;
            while solution < max_solution_val {
                println!("A");
                let mut i = 0;
                // let score = self.evaluate_solution(&solution);
                let score = self.partial_evaluate_solution(&solution, depth);
                if depth == self.sequence_len {
                    let score = self.evaluate_solution(&solution);
                    if score < *upper_bound {
                        *upper_bound = score;
                        *best_solution = solution;
                    }
                } else if depth < self.sequence_len && score >= *upper_bound {
                    solution += (depth..self.sequence_len)
                        .map(|x| (self.bases[x] - 1) * self.base_weights[x])
                        .map(|x| x as u64)
                        .sum::<u64>();
                    println!("SKIP");
                    depth -= 1;
                    continue;
                }
                'incr_loop: while i < self.sequence_len {
                    if subset & (1 << i) != 0 {
                        i += 1;
                        continue;
                    }
                    let digit = (solution / (self.base_weights[i] as u64)) % (self.bases[i] as u64);
                    if digit + 1 < self.bases[i].into() {
                        solution += self.base_weights[i] as u64;
                        depth = i + 1;
                        break 'incr_loop;
                    }
                    i += 1;
                }
                if i >= self.sequence_len {
                    return;
                }
            }
        });
    }

    fn evaluate_assignment(&self, assignment: &u32) -> f32 {
        self.tsp.get(assignment)
    }

    fn evaluate_solution(&self, solution: &u64) -> f32 {
        self.solution_to_assignment(*solution)
            .map(|x| self.evaluate_assignment(&x))
            .fold(0.0, |a: f32, b| a.max(b))
    }

    fn partial_evaluate_solution(&self, solution: &u64, depth: usize) -> f32 {
        let fixed_places = if depth == 32 {
            u32::MAX
        } else {
            (1u32 << depth) - 1
        };
        self.solution_to_assignment(*solution)
            .map(|x| x & fixed_places)
            .map(|x| self.evaluate_assignment(&x))
            .fold(0.0, |a: f32, b| a.max(b))
    }

    pub fn save(&self) -> Result<()> {
        let filename = format!(
            "results/{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let file = std::fs::File::create(filename)?;
        serde_json::to_writer_pretty(file, &self.best_solutions)?;
        Ok(())
    }
}
