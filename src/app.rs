use eframe;
use itertools::Itertools;
use std::{env, fs, io::Write};

use crate::utils::{Config, PointOfInterest, read_csv};

struct MyApp {
    is_api_key_removed: bool,
    active_file: Option<String>,
    new_filename: String,
    new_trip_popup: bool,
    config: Option<Config>,
    trip_places: Option<Vec<PointOfInterest>>,
    existing_files: Vec<String>,
}
impl MyApp {
    fn new() -> Self{
        Self {
            is_api_key_removed: false,
            active_file: None,
            new_filename: "".to_string(),
            new_trip_popup: false,
            config: None,
            trip_places: None,
            existing_files: vec![],
        }
    }
    fn check_existing_files(self: &mut Self) {
        let mut existing_files = fs::read_dir("trips")
            .unwrap()
            .map(|x| x.unwrap().path().file_stem().unwrap().to_owned())
            .collect::<Vec<_>>();
        existing_files.sort();
        self.existing_files = existing_files
            .iter()
            .tuple_windows()
            .filter(|(a, b)| a == b)
            .map(|(a, _)| a.to_str().unwrap().to_string())
            .collect();
    }
    fn save_trip_config(self: &Self) {
        let mut cfg_file = fs::File::create(format!("trips/{}.json", self.active_file.clone().unwrap())).expect("Cannot create a trip file");
        let mut places_file = fs::File::create(format!("trips/{}.csv", self.active_file.clone().unwrap())).expect("Cannot create a trip file");
        writeln!(cfg_file, "{{").unwrap();
        writeln!(cfg_file, "\t\"day_weights\": [],").unwrap();
        writeln!(cfg_file, "\t\"origin\": [0.0, 0.0],").unwrap();
        writeln!(cfg_file, "\t\"upper_bound_random_size\": 1000000").unwrap();
        writeln!(cfg_file, "}}").unwrap();

        writeln!(places_file, "Location name;Lat;Lon;Obligatory;Duration").unwrap();
        if let Some(v) = &self.trip_places {
            for place in v.iter() {
                writeln!(
                    places_file,
                    "{};{};{};{};{}",
                    place.name,
                    place.location.0,
                    place.location.1,
                    place.obligatory,
                    place.duration,
                ).unwrap();
            }
        }
    }
    fn load_trip_file(self: &mut Self, filename: &str) {
        let file = fs::read_to_string(format!("trips/{}.json", filename)).expect("Cannot read the trip file");
        self.config = serde_json::from_str(&file).expect("Trip file has an incorrect format");
        self.trip_places = Some(read_csv(&format!("trips/{}.csv", filename)).unwrap());
        self.active_file = Some(filename.to_string());
    }
}
impl eframe::App for MyApp {
    fn ui(self: &mut Self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.is_api_key_removed {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        egui::MenuBar::new().ui(ui, |ui| {
            let new_trip_btn = ui.button("New trip");
            if new_trip_btn.clicked() {
                self.new_trip_popup = true;
                self.check_existing_files();
            }
            if self.new_trip_popup {
                egui::Window::new("Plan a new trip")
                    .resizable(false)
                    .collapsible(false)
                    .show(ui.ctx(), |ui| {
                        ui.heading("The name of your trip");
                        ui.text_edit_singleline(&mut self.new_filename);
                        let save_btn = ui.add_enabled_ui(self.new_filename.len() > 0, |ui| ui.button("Save"));
                        if ui.button("Cancel").clicked() {
                            self.new_trip_popup = false;
                        }
                        let does_exist = fs::exists(&self.new_filename).expect("Unable to access the filesystem"); 
                        if does_exist {
                            ui.heading("This file already exists, choose a different name.");
                        }
                        if save_btn.inner.clicked() && !does_exist{
                            self.active_file = Some(self.new_filename.clone());
                            self.new_filename = "".to_string();
                            self.save_trip_config();
                            self.new_trip_popup = false;
                        }
                    }
                );
            }

            let open_file_btn = ui.menu_button("Open file", |ui| {
                let mut file_to_load = None;
                for file in self.existing_files.iter() {
                    if ui.button(file).clicked() {
                        file_to_load = Some(file.clone());
                    }
                }
                if let Some(filename) = file_to_load {
                    self.load_trip_file(&filename);
                }
            });
            if open_file_btn.response.clicked() {
                self.check_existing_files();
            }
            ui.separator();
            let btn = ui.button("Delete API key");
            if btn
                .on_hover_text("This will close the app and you'll have to provide the API key again")
                .clicked() {
                fs::remove_file(".env").expect("Cannot edit the .env file");
                self.is_api_key_removed = true;
            }
        });
        egui::CentralPanel::default_margins().show(ui, |ui| {
            let config = self.config.as_mut().unwrap();
            if let None = self.active_file {
                ui.heading("To edit a trip, create a new trip in the menu bar");
            }
            else {
                ui.heading("Configuration");
                let number_of_days = config.day_weights.len();
                ui.horizontal(|ui| {
                    ui.label(format!("Days ({})", number_of_days));
                    for day in config.day_weights.iter_mut() {
                        *day = (100.0 * *day).round() / 100.0;
                        let mut day_display = day.to_string();
                        ui.text_edit_singleline(&mut day_display);
                        day_display = day_display
                            .chars()
                            .filter(|x| "0123456789.".contains(*x))
                            .collect();
                        *day = day_display.parse().unwrap();
                    }
                    if ui.button(" + ").clicked() {
                        config.day_weights.push(1.0);
                    }

                });
                self.config = Some(config);
                ui.separator();
                ui.heading("Points of Interest");
                ui.separator();
                ui.horizontal(|ui|{
                    ui.button("Run calculation");
                    if ui.button("Save without running").clicked() {
                        self.save_trip_config();
                    }
                });
            }
        });
    }
}

fn get_api_key() -> Result<bool, eframe::Error> {
    dotenvy::dotenv().ok();
    let api_key = env::var("API_KEY");
    if let Ok(_) = api_key {
        return Ok(true)
    }
    let mut new_key = String::new();
    let res = eframe::run_ui_native(
        "Trip finder",
        eframe::NativeOptions::default(),
        move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("For the program to function, you require an API key from https://openrouteservice.org/\nPlease provide it in the box below to continue");
                ui.text_edit_singleline(&mut new_key);
                
                let btn = ui.add_enabled_ui(new_key.len() > 0, |ui| ui.button("Submit"));
                if btn.inner.clicked() {
                    println!("Writing!");
                    fs::write(".env", format!("API_KEY=\"{}\"", new_key)).expect("Cannot write to file");
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        },
    );
    if let Err(v) = res {
        return Err(v);
    } else {
        dotenvy::dotenv().unwrap();
        let api_key = env::var("API_KEY");
        println!("API key: {:?}", api_key);
        return Ok(
            match api_key {
                Ok(_) => true,
                Err(_) => false,
            }
        );
    }
}

fn run_app() -> Result<(), eframe::Error> {
    eframe::run_native(
        "Trip finder",
        eframe::NativeOptions::default(), 
        Box::new(|_| Ok(Box::new(MyApp::new())))
    )
}

pub fn run() -> Result<(), eframe::Error> {
    let is_api_key = get_api_key();
    if let Err(_) = is_api_key { return Ok(()); }
    if let Ok(false) = is_api_key { return Ok(()); }

    run_app()
}
