use csv;
use std::{error::Error, fs};
mod solver;
mod tsp;
mod utils;
mod app;
use anyhow;
use dotenvy;
use utils::{Config, PointOfInterest, file_config, get_matrix, read_csv};

use solver::Solver;
use tsp::TSPLookup;
#[tokio::main]
async fn main() -> eframe::Result<()> {
    file_config();
    app::run().unwrap();

    dotenvy::dotenv().ok();
    let cfg = Config::new().unwrap();
    let mut points_of_interest = read_csv(&String::from("./places.csv")).unwrap();
    points_of_interest.insert(
        0,
        PointOfInterest::new("Origin".to_string(), cfg.origin.0, cfg.origin.1, true, 0.0),
    );

    let matrix = get_matrix(&points_of_interest)
        .await
        .expect("File input failed (PoI)");

    println!("{}", &matrix);

    let tsp = TSPLookup::new(&matrix, &points_of_interest);

    let mut solver = Solver::new(
        &points_of_interest,
        cfg.day_weights,
        tsp,
        cfg.upper_bound_random_size,
    );
    solver.run();
    let _ = solver.save();

    Ok(())
}
