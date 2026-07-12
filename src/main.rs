use csv;
use std::{error::Error, fs};
mod utils;
mod solver;
mod tsp;
use anyhow;
use dotenvy;
use utils::{get_matrix, PointOfInterest, Config};

use solver::Solver;
use tsp::{TSPLookup, TSPCache};


fn read_csv(path: &String) -> Result<Vec<PointOfInterest>, Box<dyn Error>> {
    let mut res = vec![];
    let file_contents = fs::read_to_string(path).expect("Should have been able to read the file");

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(file_contents.as_bytes());
    for result in rdr.records() {
        let result = result?;
        let poi = PointOfInterest::new(
            result[0].to_string(),
            result[1].parse().unwrap(),
            result[2].parse().unwrap(),
            if result[3] == *"1" { true } else { false },
        );
        res.push(poi);
    }
    Ok(res)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cfg = Config::new().unwrap();
    let mut points_of_interest = read_csv(&String::from("./places.csv")).unwrap();
    points_of_interest.push(PointOfInterest::new("Origin".to_string(), cfg.origin.0, cfg.origin.1, true));

    let matrix = get_matrix(&points_of_interest).await.expect("File input failed (PoI)");

    println!("{}", &matrix);

    let tspc = TSPCache::build_cache(&matrix);
    println!("TSP cache calculated");

    let mut solver = Solver::new(matrix, &points_of_interest, cfg.day_weights.clone(), tspc);
    solver.run();
    solver.save()?;

    Ok(())
}
