use csv;
use std::{error::Error, fs};
mod utils;
use anyhow;
use dotenvy;
use utils::{get_matrix, PointOfInterest};
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
    let points_of_interest = read_csv(&String::from("./places.csv")).unwrap();
    let res = get_matrix(&points_of_interest).await;

    println!("{}", res.unwrap());

    Ok(())
}
