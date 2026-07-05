use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path};

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

#[derive(Serialize, Deserialize)]
pub struct MatrixResponse {
    distances: Vec<Vec<Option<f64>>>,
    durations: Vec<Vec<Option<f64>>>,
}

#[derive(Debug)]
pub struct DistanceMatrix {
    pub keys: Vec<String>,
    pub distances: Vec<Vec<Option<f64>>>,
    pub durations: Vec<Vec<Option<f64>>>,
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

pub async fn get_matrix(coords: Vec<[f64; 2]>, labels: Vec<String>) -> Result<DistanceMatrix> {
    let target_file = Path::new("./data/cache.json");
    if target_file.exists() {
        let cached = fs::read_to_string(target_file)?;
        let data: MatrixResponse = serde_json::from_str(&cached)?;
        Ok(DistanceMatrix::from(data, labels))
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
        fs::write(target_file, serde_json::to_string_pretty(&matrix_resp)?).unwrap();
        Ok(DistanceMatrix::from(matrix_resp, labels))
    }
}
