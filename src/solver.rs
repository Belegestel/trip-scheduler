use crate::{
    tsp::TSPLookup,
    utils::{PointOfInterest, iterate_bits_as_indices, subsets},
};
use anyhow::Result;
use rand::{rng, seq::index};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;

pub struct Solver {
    pub mandatory: Vec<bool>,
    pub day_weights: Vec<f32>,
    pub best_solutions: HashMap<u8, (u64, f32)>, // Optional count, (assignment, time)
    tsp: TSPLookup,
    bases: Vec<u32>,
    base_weights: Vec<u64>,
    mandatory_idx: Vec<usize>,
    optional_idx: Vec<usize>,
    sequence_len: usize,
    optional_mask: u32,
    mandatory_mask: u32,
    upper_bound_random_size: usize,
    poi: Vec<PointOfInterest>,
}
impl Solver {
    pub fn new(
        poi: &Vec<PointOfInterest>,
        day_weights: Vec<f32>,
        tsp: TSPLookup,
        upper_bound_random_size: usize,
    ) -> Self {
        let mandatory: Vec<bool> = poi.iter().map(|x| x.obligatory).collect();
        let mandatory_idx: Vec<usize> = mandatory
            .iter()
            .skip(1)
            .enumerate()
            .filter(|(_, x)| **x)
            .map(|(idx, _)| idx)
            .collect();
        let optional_idx: Vec<usize> = mandatory
            .iter()
            .skip(1)
            .enumerate()
            .filter(|(_, x)| !**x)
            .map(|(idx, _)| idx)
            .collect();
        let sequence_len = &mandatory_idx.len() + optional_idx.len();
        let bases: Vec<u32> = mandatory
            .iter()
            .skip(1)
            .map(|x| day_weights.len() + { if *x { 0 } else { 1 } })
            .map(|x| x as u32)
            .collect();
        let base_weights = bases
            .iter()
            .scan(1, |st, elem| {
                let res = Some(*st);
                *st *= *elem as u64;
                res
            })
            .collect();
        let optional_mask = optional_idx.iter().map(|x| 1u32 << x).sum();
        let mandatory_mask = mandatory_idx.iter().map(|x| 1u32 << x).sum();
        Self {
            mandatory,
            day_weights,
            best_solutions: HashMap::new(),
            tsp,
            bases,
            base_weights,
            mandatory_idx,
            optional_idx,
            sequence_len,
            optional_mask,
            mandatory_mask,
            upper_bound_random_size,
            poi: poi.to_vec(),
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

    fn precalculate(&self, bound_size: usize, best: &mut f32, best_solution: &mut u64, mask: &u32) {
        let mut local_rng = rng();

        let days_n = self.day_weights.len();
        // Treat the included optionals as mandatory
        let total_assignments = days_n.pow(mask.count_ones() as u32);
        let base_value: u64 = iterate_bits_as_indices(&!mask)
            .take_while(|x| *x < self.base_weights.len())
            .map(|x| self.base_weights[x] * self.day_weights.len() as u64) // UNSURE
            .sum();
        let local_base_weights: Vec<_> = iterate_bits_as_indices(&mask)
            .map(|x| self.bases[x])
            .scan(1, |st, elem| {
                let res = Some(*st);
                *st *= elem as u64;
                res
            })
            .collect();
        for solution in index::sample(
            &mut local_rng,
            total_assignments as usize,
            bound_size.min(total_assignments) as usize,
        ) {
            let solution: u64 = solution as u64;
            let solution: u64 = iterate_bits_as_indices(&mask)
                .enumerate()
                .map(|(rand_idx, real_idx)| {
                    self.base_weights[real_idx]
                        * ((solution / local_base_weights[rand_idx])
                            % self.day_weights.len() as u64)
                })
                .sum();
            let solution = solution + base_value;
            let val = self.evaluate_solution(&solution);
            if val < *best {
                *best = val;
                *best_solution = solution;
            }
        }
    }

    pub fn run(&mut self) {
        // Constants
        let mut best = f32::INFINITY;
        let mut best_solution = 0;

        // Computation of best distribution consisting only
        // of mandatory locations
        self.precalculate(
            self.upper_bound_random_size,
            &mut best,
            &mut best_solution,
            &((1u32 << self.mandatory_idx.len()) - 1),
        );
        // Branch'n'bound
        self.bnb(&mut best, &mut best_solution, 0);
        let _ = self.best_solutions.insert(0, (best_solution, best));

        let res: Vec<_> = (1..=self.optional_idx.len() as u8)
            .into_par_iter()
            .map(|optional_count| {
                let mut upper_bound = f32::MAX;
                let mut best_solution = 0;
                self.bnb(&mut upper_bound, &mut best_solution, optional_count);
                (best_solution, upper_bound)
            })
            .collect();
        for (idx, val) in res.into_iter().enumerate() {
            self.best_solutions.insert((idx + 1) as u8, val);
        }
    }

    fn dfs(
        &self,
        upper_bound: &mut f32,
        best_solution: &mut u64,
        subset: &u32,
        solution: u64,
        depth: usize,
        base_value: &u64,
    ) {
        let score = self.partial_evaluate_solution(&(solution + base_value), depth);
        if score >= *upper_bound {
            // If worse than current best, stop recursion
            return;
        }
        if depth == self.sequence_len {
            // Better than current best, save and stop recursion
            // because we've already reached the end
            *upper_bound = score;
            *best_solution = solution + base_value;
            return;
        }

        // Increase by one and recursion
        if subset & (1u32 << depth) == 0 {
            self.dfs(
                upper_bound,
                best_solution,
                subset,
                solution,
                depth + 1,
                base_value,
            );
        } else {
            for i in 0..self.day_weights.len() {
                self.dfs(
                    upper_bound,
                    best_solution,
                    subset,
                    solution + (self.base_weights[depth] * (i as u64)) as u64,
                    depth + 1,
                    base_value,
                );
            }
        }
    }

    fn bnb(&self, upper_bound: &mut f32, best_solution: &mut u64, optional_count: u8) {
        let _ = subsets(self.optional_idx.len(), optional_count.into())
            .map(|x| (x << self.mandatory_idx.len()) | self.mandatory_mask)
            .for_each(|subset| {
                let base_value: u64 = iterate_bits_as_indices(&!subset)
                    .take_while(|x| *x < self.base_weights.len())
                    .map(|x| self.base_weights[x] * self.day_weights.len() as u64)
                    .sum();
                self.precalculate(
                    self.upper_bound_random_size,
                    upper_bound,
                    best_solution,
                    &subset,
                );
                self.dfs(upper_bound, best_solution, &subset, 0, 0, &base_value);
            });
        println!("Best with {} optionals: {}", optional_count, upper_bound);
    }

    fn evaluate_assignment(&self, assignment: &u32) -> f32 {
        self.tsp.get(assignment)
    }

    fn evaluate_solution(&self, solution: &u64) -> f32 {
        self.solution_to_assignment(*solution)
            .map(|x| self.evaluate_assignment(&x))
            .zip(self.day_weights.iter())
            .map(|(eval, weight)| eval / weight)
            .fold(0.0, |a: f32, b| a.max(b))
            +
            self
            .solution_to_assignment(*solution)
            .map(|x| self.evaluate_assignment(&x))
            .sum::<f32>() * 0.0001
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
            .zip(self.day_weights.iter())
            .map(|(eval, weight)| eval / weight)
            .fold(0.0, |a: f32, b| a.max(b))
            +
            self
            .solution_to_assignment(*solution)
            .map(|x| self.evaluate_assignment(&x))
            .sum::<f32>() * 0.0001
    }

    pub fn save(&self) -> Result<()> {

        #[derive(Serialize)]
        struct SaveStruct {
            assignments: HashMap<u8, (Vec<f32>, Vec<u8>)>,
            names: Vec<String>,
        }

        let filename = format!(
            "results/{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let mut save_var: HashMap<u8, (Vec<f32>, Vec<u8>)> = HashMap::new();
        for (k, (assignment, _)) in self.best_solutions.iter() {
            let display_assignment: Vec<u8> = self
                .base_weights
                .iter()
                .zip(&self.bases)
                .map(|(base_weight, base)| (assignment / base_weight) % *base as u64)
                .map(|x| x.try_into().unwrap())
                .collect();
            let times = self.solution_to_assignment(*assignment)
                .map(|x| self.evaluate_assignment(&x))
                .collect();
            save_var.insert(*k, (times, display_assignment));
        }

        println!("Save data: {:?}", save_var);

        let packed_data = SaveStruct { assignments: save_var, names: self.poi.iter().map(|x| x.name.clone()).collect() };


        let file = std::fs::File::create(filename)?;
        serde_json::to_writer_pretty(file, &packed_data)?;
        Ok(())
    }
}
