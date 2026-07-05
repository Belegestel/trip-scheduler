use csv;
use std::{error::Error, fs};
mod fetch_data;
use anyhow;
use dotenvy;
use fetch_data::{DistanceMatrix, get_matrix, PointOfInterest};
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
            result[1].parse().unwrap(),
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
    for poi in points_of_interest.iter() {
        println!("{:?}", poi);
    }
    let res = get_matrix(&points_of_interest).await;

    println!("{:#?}", res);

    Ok(())
}
