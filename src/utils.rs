use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use std::{env, fmt::Display, fs, path::Path};

#[derive(Debug, Clone)]
pub struct PointOfInterest {
    pub name: String,
    pub location: (f64, f64),
    pub obligatory: bool,
    pub duration: f32,
}
impl PointOfInterest {
    pub fn new(name: String, lat: f64, lon: f64, obligatory: bool, duration: f32) -> Self {
        Self {
            name,
            location: (lat, lon),
            obligatory,
            duration,
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
        let keys: Vec<String> = self
            .keys
            .iter()
            .enumerate()
            .map(|x| format!("{}. {}", x.0 + 1, x.1))
            .collect();
        fn get_table(
            keys: &Vec<String>,
            distances: &Vec<Vec<f32>>,
            durations: &Vec<Vec<f32>>,
            is_distance: bool,
        ) -> String {
            let labels_width = keys
                .iter()
                .enumerate()
                .map(|x| x.1.len() + format!("{}", x.0 + 1).len())
                .max()
                .unwrap();
            let table_content: Vec<Vec<String>> = (if is_distance { distances } else { durations })
                .iter()
                .map(|y| {
                    y.iter()
                        .map(|x| {
                            if is_distance {
                                format!("{:.1}km", x / 1000.0)
                            } else {
                                format!("{}'", (x / 60.0).round())
                            }
                        })
                        .collect()
                })
                .collect();
            let column_widths: Vec<usize> = table_content
                .iter()
                .map(|x| x.iter().map(|y| y.len()).max().unwrap())
                .collect();

            let header = " ".repeat(labels_width).to_string()
                + &column_widths
                    .iter()
                    .enumerate()
                    .map(|x| format!("| {:width$}", x.0 + 1, width = x.1))
                    .collect::<String>()
                + "\n";
            let header_length = header.len();

            header
                + &"-".repeat(header_length + 5)
                + "\n"
                + &keys
                    .iter()
                    .zip(table_content.iter())
                    .map(|x| {
                        format!("{:width$}", x.0, width = labels_width)
                            + &x.1
                                .iter()
                                .zip(column_widths.iter())
                                .map(|y| format!("| {:width$}", y.0, width = y.1))
                                .collect::<String>()
                            + "\n"
                    })
                    .collect::<String>()
        }
        let res = get_table(&keys, &self.distances, &self.durations, true)
            + &get_table(&keys, &self.distances, &self.durations, false);
        write!(f, "{}", res)
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub origin: (f64, f64),
    pub day_weights: Vec<f32>,
    pub upper_bound_random_size: usize,
}
impl Config {
    pub fn new() -> Result<Self> {
        let file = fs::read_to_string("./config.json")?;
        Ok(serde_json::from_str(&file)?)
    }
}

pub async fn get_matrix(points: &Vec<PointOfInterest>) -> Result<DistanceMatrix> {
    let coords: Vec<[f64; 2]> = points
        .iter()
        .map(|x| [x.location.1, x.location.0])
        .collect();
    let labels: Vec<String> = points.iter().map(|x| x.name.clone()).collect();
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

        let mut matrix_resp = client
            .post("https://api.openrouteservice.org/v2/matrix/foot-walking")
            .header("Authorization", api_key)
            .json(&req)
            .send()
            .await?
            .error_for_status()?
            .json::<MatrixResponse>()
            .await?;

        // Floyd-Warshall for triangle equality
        for k in 0..matrix_resp.durations.len() {
            for i in 0..matrix_resp.durations.len() {
                for j in 0..matrix_resp.durations.len() {
                    let direct = matrix_resp.durations[i][j];
                    let via = matrix_resp.durations[i][k] + matrix_resp.durations[k][j];
                    if via < direct {
                        matrix_resp.durations[i][j] = via;
                    }
                }
            }
        }
        let dist_matrix = DistanceMatrix::from(matrix_resp, labels);
        fs::write(target_file, serde_json::to_string_pretty(&dist_matrix)?).unwrap();
        Ok(dist_matrix)
    }
}

pub fn subsets(size: usize, number: usize) -> impl Iterator<Item = u32> {
    let limit = 1u32 << size;
    let mut mask = if number == 0 { 0 } else { (1u32 << number) - 1 };
    std::iter::from_fn(move || {
        if mask >= limit {
            return None;
        }
        let result = mask as u32;
        if number == 0 {
            mask = limit;
        } else {
            let c = mask & mask.wrapping_neg();
            let r = mask + c;
            mask = (((r ^ mask) >> 2) / c) | r;
        }
        Some(result)
    })
}

pub fn iterate_bits_as_indices(num: &u32) -> impl Iterator<Item = usize> {
    let mut mask = *num;
    std::iter::from_fn(move || {
        while mask != 0 {
            let i = mask.trailing_zeros();
            mask &= mask - 1;
            return Some(i as usize);
        }
        None
    })
}
