use crate::{
    tsp::TSPCache,
    utils::{DistanceMatrix, PointOfInterest},
};
use anyhow::Result;
use serde::Serialize;
use std::{collections::HashMap, io::Write};

pub struct Solver {
    pub matrix: DistanceMatrix,
    pub mandatory: Vec<bool>,
    pub day_weights: Vec<f32>,
    pub best_solutions: HashMap<u8, ReadySolution>,
    tsp: TSPCache,
}
impl Solver {
    pub fn new(
        distance_matrix: DistanceMatrix,
        poi: &Vec<PointOfInterest>,
        day_weights: Vec<f32>,
        tsp: TSPCache,
    ) -> Self {
        Self {
            matrix: distance_matrix,
            mandatory: poi.iter().map(|x| x.obligatory).collect(),
            day_weights,
            best_solutions: HashMap::new(),
            tsp,
        }
    }

    pub fn run(&mut self) {
        let days_total = self.day_weights.len();
        let choices: Vec<_> = self
            .mandatory
            .iter()
            .map(|x| days_total + if *x { 0 } else { 1 })
            .collect();
        let choices: Vec<_> = choices[..choices.len() - 1].to_vec();

        let mut total_done = 0;
        let total_to_calculate = {
            let mandatory_count = self.mandatory.iter().filter(|&&x| x).count();
            let optional_count = self.mandatory.iter().filter(|&&x| !x).count();
            (days_total as u128).pow(mandatory_count as u32)
                * ((days_total + 1) as u128).pow(optional_count as u32)
        };
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
            total_done += 1;
            if total_done % 100000 == 0 {
                let progress = total_done as f64 / total_to_calculate as f64;
                print!(
                    "\r\x1b[2KProgress: [{}{}] {:.1}% ({} / {})",
                    "#".repeat((progress * 20.0).round() as usize),
                    ".".repeat(20 - (progress * 20.0).round() as usize),
                    progress * 100.0,
                    total_done,
                    total_to_calculate
                );
                std::io::stdout().flush().unwrap();
            }
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

#[derive(Serialize)]
pub struct ReadySolution {
    assignments: Vec<Option<u8>>,
    max_time: f32,
    number_of_additional: u8,
}
impl ReadySolution {
    fn new(assignments: Vec<Option<u8>>, solver: &Solver) -> Self {
        let number_of_additional = assignments
            .iter()
            .zip(solver.mandatory.iter())
            .filter(|(a, b)| if let Some(_) = a { **b } else { false })
            .map(|(_, b)| if *b { 1 } else { 0 })
            .sum();
        let days: Vec<_> = (0..solver.day_weights.len())
            .map(|day_idx| {
                assignments
                    .iter()
                    .map(|value| {
                        if let Some(v) = value {
                            *v as usize == day_idx
                        } else {
                            false
                        }
                    })
                    .collect()
            })
            .collect();

        let max_time = days
            .iter()
            .map(|day_assignment| solver.tsp.get(day_assignment))
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        Self {
            assignments,
            max_time,
            number_of_additional,
        }
    }
}
