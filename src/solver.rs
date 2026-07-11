use crate::utils::{DistanceMatrix, PointOfInterest};
use itertools::Itertools;
use std::collections::HashMap;

pub struct Solver {
    matrix: &'static DistanceMatrix,
    mandatory: Vec<bool>,
    day_weights: Vec<f32>,
    best_solutions: HashMap<u8, ReadySolution>,
}
impl Solver {
    pub fn new(
        distance_matrix: &'static DistanceMatrix,
        poi: &Vec<PointOfInterest>,
        day_weights: Vec<f32>,
    ) -> Self {
        Self {
            matrix: distance_matrix,
            mandatory: poi.iter().map(|x| x.obligatory).collect(),
            day_weights,
            best_solutions: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        let days_total = self.day_weights.len();
        let choices: Vec<_> = self
            .mandatory
            .iter()
            .map(|x| days_total + if *x { 0 } else { 1 })
            .collect();

        let mut assignment = vec![0; choices.len()];
        loop {
            let solution = ReadySolution::new(
                assignment
                    .iter()
                    .map(|x| {
                        if *x >= days_total {
                            None
                        } else {
                            Some(*x as u8)
                        }
                    })
                    .collect(),
                self,
            );
            let current_best_score = self.best_solutions.get(&solution.number_of_additional);
            let current_score = solution.number_of_additional;
            match current_best_score {
                Some(v) => {
                    if v.max_time > solution.max_time {
                        self.best_solutions.insert(current_score, solution);
                    }
                }
                None => {
                    self.best_solutions.insert(current_score, solution);
                }
            };

            let mut i = 0;
            while i < assignment.len() {
                assignment[i] += 1;
                if assignment[i] < choices[i] {
                    break;
                } else {
                    assignment[i] = 0;
                    i += 1;
                }
            }

            if i >= assignment.len() {
                break;
            }
        }
    }
}

struct ReadySolution {
    assignments: Vec<Option<u8>>,
    max_time: f32,
    number_of_additional: u8,
}
impl ReadySolution {
    fn new(assignments: Vec<Option<u8>>, solver: &Solver) -> Self {
        let mut max_time = 0.0;
        let number_of_additional = assignments
            .iter()
            .zip(solver.mandatory.iter())
            .filter(|x| *x.1)
            .filter(|x| if let Some(_) = x.0 { true } else { false })
            .collect::<Vec<_>>()
            .len()
            .try_into()
            .unwrap();

        let days: Vec<Vec<usize>> = (0..solver.day_weights.len())
            .into_iter()
            .map(|day_idx| {
                assignments
                    .iter()
                    .enumerate()
                    .filter(|x| {
                        if let Some(v) = x.1 {
                            *v as usize == day_idx
                        } else {
                            false
                        }
                    })
                    .map(|x| x.0)
                    .collect()
            })
            .collect();

        for (day, day_weight) in days.iter().zip(solver.day_weights.iter()) {
            let mut day_time = f32::INFINITY;
            for path in day.iter().permutations(day.len()) {
                let path_time: f32 = path
                    .iter()
                    .tuple_windows()
                    .map(|(a, b)| solver.matrix.durations[**a][**b])
                    .sum::<f32>()
                    / day_weight;
                if path_time < day_time {
                    day_time = path_time;
                }
            }
            if day_time > max_time {
                max_time = day_time;
            }
        }

        Self {
            assignments,
            max_time,
            number_of_additional,
        }
    }
}
