mod solver;
mod tsp;
mod utils;
mod app;
use utils::file_config;

#[tokio::main]
async fn main() {
    file_config();
    app::run().unwrap();
}
