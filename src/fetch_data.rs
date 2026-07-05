use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use std::{env, fmt::Display, fs, path::Path};

#[derive(Debug)]
pub struct PointOfInterest {
    pub name: String,
    pub location: (f64, f64),
    pub obligatory: bool,
}
impl PointOfInterest {
    pub fn new(name: String, lat: f64, lon: f64, obligatory: bool) -> Self {
        Self {
            name,
            location: (lat, lon),
            obligatory,
        }
    }
}

#[derive(Serialize)]
struct MatrixRequest {
    locations: Vec<[f64; 2]>,
    metrics: Vec<String>,
    units: String,
}

#[derive(Deserialize)]
pub struct MatrixResponse {
    distances: Vec<Vec<f32>>,
    durations: Vec<Vec<f32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DistanceMatrix {
    pub keys: Vec<String>,
    pub distances: Vec<Vec<f32>>,
    pub durations: Vec<Vec<f32>>,
}
impl DistanceMatrix {
    fn from(matrix: MatrixResponse, keys: Vec<String>) -> Self {
        let MatrixResponse {
            distances,
            durations,
        } = matrix;
        DistanceMatrix {
            keys,
            distances,
            durations,
        }
    }
}
impl Display for DistanceMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels_width = self.keys
            .iter()
            .enumerate()
            .map(|x| x.1.len() + format!("{}", x.0 + 1).len())
            .max()
            .unwrap();
        let table_content: Vec<Vec<String>> = self.distances
            .iter()
            .zip(self.durations.iter())
            .map(|y| {
                y.0.iter().zip(y.1.iter()).map(|x| format!("{:.1}km / {}'", x.0 / 1000.0, x.1.round())).collect()
            }).collect();
        let column_widths: Vec<usize> = table_content
            .iter()
            .map(
                |x| x
                    .iter()
                    .map(|y| y.len())
                    .max()
                    .unwrap()
                )
            .collect();
        
        let header = " ".repeat(labels_width).to_string() +
            &column_widths.iter().enumerate().map(|x| format!("| {:width$}", x.0 + 1, width = x.1)).collect::<String>() + "\n";
        let header_length = header.len();

        let res: String = header + &"-".repeat(header_length + 5) + "\n"
            + &self.keys
                .iter()
                .zip(table_content.iter())
                .map(|x|
                    format!("{:width$}", x.0, width = labels_width) +
                    &x.1.iter().zip(column_widths.iter()).map(|y| format!("| {:width$}", y.0, width = y.1)).collect::<String>() +
                    "\n"
                )
                .collect::<String>();
        write!(f, "{}", res)
    }
}

pub async fn get_matrix(points: &Vec<PointOfInterest>) -> Result<DistanceMatrix> {
    let coords: Vec<[f64; 2]> = points
        .iter()
        .map(|x| [x.location.1, x.location.0])
        .collect();
    let labels: Vec<String> = points 
        .iter()
        .map(|x| x.name.clone())
        .collect();
    let target_file = Path::new("./data/cache.json");
    if target_file.exists() {
        println!("Using cached data...");
        let cached = fs::read_to_string(target_file)?;
        let data: DistanceMatrix = serde_json::from_str(&cached)?;
        Ok(data)
    } else {
        println!("Fetching data...");
        let api_key = env::var("API_KEY").unwrap();

        let client = reqwest::Client::new();

        let req = MatrixRequest {
            locations: coords,
            metrics: vec!["distance".into(), "duration".into()],
            units: "m".into(),
        };

        let matrix_resp = client
            .post("https://api.openrouteservice.org/v2/matrix/foot-walking")
            .header("Authorization", api_key)
            .json(&req)
            .send()
            .await?
            .error_for_status()?
            .json::<MatrixResponse>()
            .await?;


        let dist_matrix = DistanceMatrix::from(matrix_resp, labels);
        fs::write(target_file, serde_json::to_string_pretty(&dist_matrix)?).unwrap();
        Ok(dist_matrix)
    }
}
