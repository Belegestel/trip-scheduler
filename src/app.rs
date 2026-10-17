use eframe::egui;
use egui::Color32;
use itertools::Itertools;
use walkers;
use std::{env, fs, io::Write, sync::mpsc::{self, channel}};

use crate::{solver::Solver, tsp::TSPLookup, utils::{Config, PointOfInterest, get_matrix, read_csv}};

struct MapData {
    tiles: walkers::HttpTiles,
    map_memory: walkers::MapMemory,
    selected_position: Option<(f64, f64)>
}

#[derive(PartialEq)]
enum OpenedPopup {
    NewTrip,
    OriginMap,
    PoI(usize),
    Running,
    None,
}

#[derive(PartialEq)]
pub enum ProgressBar {
    Init,
    Matrix,
    TSP(f32),
    Solver(f32),
    Done,
    Failed,
}

struct MyApp {
    is_api_key_removed: bool,
    active_file: Option<String>,
    new_filename: String,
    opened_popup: OpenedPopup,
    config: Option<Config>,
    poi: Option<Vec<PointOfInterest>>,
    existing_files: Vec<String>,
    map: MapData,
    mpsc_receiver: Option<mpsc::Receiver<ProgressBar>>,
    computation_state: ProgressBar,
}
impl MyApp {
    fn new(creation_context: &eframe::CreationContext) -> Self{
        let mut res = Self {
            is_api_key_removed: false,
            active_file: None,
            new_filename: "".to_string(),
            opened_popup: OpenedPopup::None,
            config: None,
            poi: None,
            existing_files: vec![],
            map: MapData {
                tiles: walkers::HttpTiles::new(walkers::sources::OpenStreetMap, creation_context.egui_ctx.to_owned()),
                map_memory: walkers::MapMemory::default(),
                selected_position: None,
            },
            mpsc_receiver: None,
            computation_state: ProgressBar::Init
        };
        res.check_existing_files();
        return res;
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
        writeln!(cfg_file, "\t\"day_weights\": [{}],", self.config.as_ref().unwrap().day_weights.iter().map(|x| x.to_string()).join(", ")).unwrap();
        writeln!(cfg_file, "\t\"origin\": [{}, {}],", self.config.as_ref().unwrap().origin.0, self.config.as_ref().unwrap().origin.1).unwrap();
        writeln!(cfg_file, "\t\"upper_bound_random_size\": 1000000").unwrap();
        writeln!(cfg_file, "}}").unwrap();

        writeln!(places_file, "Location name;Lat;Lon;Obligatory;Duration").unwrap();
        if let Some(v) = &self.poi {
            for place in v.iter() {
                writeln!(
                    places_file,
                    "{};{};{};{};{}",
                    place.name,
                    place.location.0,
                    place.location.1,
                    if place.obligatory { 1 } else { 0 },
                    place.duration,
                ).unwrap();
            }
        }
    }
    fn load_trip_file(self: &mut Self, filename: &str) {
        let file = fs::read_to_string(format!("trips/{}.json", filename)).expect("Cannot read the trip file");
        self.config = serde_json::from_str(&file).expect("Trip file has an incorrect format");
        self.poi = Some(read_csv(&format!("trips/{}.csv", filename)).unwrap());
        self.active_file = Some(filename.to_string());
    }
    fn map_popup(self: &mut Self, ui: &mut egui::Ui, start_pos: Option<(f64, f64)>) -> Option<(f64, f64)> {
        // returns either None or Some((lat, lon))
        if let Some(v) = start_pos {
            if let None = self.map.selected_position {
                self.map.selected_position = Some(v);
            }
        }
        let mut res = None;
        egui::Window::new("Plan a new trip")
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                let is_enabled = if let Some(_) = self.map.selected_position { true } else { false };
                ui.horizontal(|ui| {
                    if ui.add_enabled_ui(is_enabled, |ui| ui.button("Select position")).inner.clicked() {
                        res = self.map.selected_position;
                        self.map.selected_position = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.map.selected_position = None;
                        self.opened_popup = OpenedPopup::None;
                    }
                    ui.label("Map data from OpenStreetMap");
                });
                ui.allocate_ui(egui::vec2(600.0, 400.0), |ui| {
                    let start_pos = if let Some(v) = start_pos { v } 
                    else { self.config.as_ref().unwrap().origin } ;
                    let _ = walkers::Map::new(
                            Some(&mut self.map.tiles),
                            &mut self.map.map_memory,
                            walkers::lon_lat(start_pos.1, start_pos.0)
                        ).zoom_with_ctrl(false).show(ui, |ui, response, projector, _| {
                        if let Some((lat, lon)) = self.map.selected_position {
                            let screen_pos = projector.project(
                                walkers::lon_lat(lon, lat)
                            );
                            ui.painter().circle_filled(screen_pos.to_pos2(), 2.0, Color32::RED);
                        }
                        if response.clicked() {
                            if let Some(pointer_pos) = response.interact_pointer_pos() {
                                let position = projector.unproject(pointer_pos.to_vec2());
                                let lat = position.y();
                                let lon = position.x();
                                self.map.selected_position = Some((lat, lon));
                            }
                        }
                    });
                });
        });
        res
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
                self.opened_popup = OpenedPopup::NewTrip;
                self.check_existing_files();
            }
            if self.opened_popup == OpenedPopup::NewTrip {
                egui::Window::new("Plan a new trip")
                    .resizable(false)
                    .collapsible(false)
                    .show(ui.ctx(), |ui| {
                        ui.heading("The name of your trip");
                        ui.text_edit_singleline(&mut self.new_filename);
                        let save_btn = ui.add_enabled_ui(self.new_filename.len() > 0, |ui| ui.button("Save"));
                        if ui.button("Cancel").clicked() {
                            self.opened_popup = OpenedPopup::None;
                        }
                        let does_exist = fs::exists(&self.new_filename).expect("Unable to access the filesystem"); 
                        if does_exist {
                            ui.heading("This file already exists, choose a different name.");
                        }
                        if save_btn.inner.clicked() && !does_exist{
                            self.active_file = Some(self.new_filename.clone());
                            self.new_filename = "".to_string();
                            self.save_trip_config();
                            self.opened_popup = OpenedPopup::None;
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
        if self.opened_popup == OpenedPopup::Running {
            let new_resp = self.mpsc_receiver.as_ref().unwrap().try_recv();
            match new_resp {
                Ok(v) => self.computation_state = match v {
                    ProgressBar::Solver(_) => {
                        let diff = self.poi.as_ref().unwrap().iter().filter(|x| !x.obligatory).collect::<Vec<_>>().len();
                        if let ProgressBar::Solver(v) = self.computation_state {
                            ProgressBar::Solver(v + 1.0 / (diff as f32))
                        }
                        else {
                            ProgressBar::Solver(1.0 / (diff as f32))
                        }
                    },
                    v => v,
                },
                Err(mpsc::TryRecvError::Disconnected) => {
                    if self.computation_state != ProgressBar::Done { 
                        self.computation_state = ProgressBar::Failed;
                    }
                },
                Err(mpsc::TryRecvError::Empty) => {}
            };
            egui::Window::new("Plan a new trip")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}/4", match self.computation_state {
                            ProgressBar::Init => 0,
                            ProgressBar::Matrix => 1,
                            ProgressBar::TSP(_) => 2,
                            ProgressBar::Solver(_) => 3,
                            ProgressBar::Done => 4,
                            ProgressBar::Failed => 0,
                        }));
                        ui.add(
                            egui::ProgressBar::new(match self.computation_state {
                                ProgressBar::TSP(v) => v,
                                ProgressBar::Solver(v) => v,
                                ProgressBar::Done => 1.0,
                                _ => 0.0
                            })
                        )
                    });
                    let mut button_visible = false;
                    if let ProgressBar::Failed = self.computation_state {
                        ui.heading("The computation process has failed. Consider trying again.");
                        button_visible = true;
                    }
                    if let ProgressBar::Done = self.computation_state {
                        ui.heading("Computation has completed, the file has been saved.");
                        button_visible = true;
                    }
                    if button_visible {
                        if ui.button("Close").clicked() {
                            self.opened_popup = OpenedPopup::None;
                        }
                    }
                });
        }
        egui::Panel::bottom("bottom-panel-main").show(ui, |ui| {
            ui.horizontal(|ui|{
                let is_btn_enabled = if let Some(_) = self.active_file { true } else { false } 
                    && if let Some(v) = self.poi.as_ref() { v.len() > 0 } else { false } 
                    && if let Some(v) = self.config.as_ref() { v.day_weights.len() > 0 } else { false };
                let run_calc_btn = ui.add_enabled_ui(is_btn_enabled, |ui| ui.button("Run calculation"));
                let save_btn = ui.add_enabled_ui(is_btn_enabled, |ui| ui.button("Save without running"));
                if run_calc_btn.inner.clicked() {
                    self.save_trip_config();
                    self.opened_popup = OpenedPopup::Running;
                    self.computation_state = ProgressBar::Init;
                    let cfg = self.config.as_ref().unwrap();
                    let day_weights = cfg.day_weights.clone();
                    let upper_bound_random_size = cfg.upper_bound_random_size;
                    let filename = self.active_file.as_ref().to_owned().unwrap().to_string();
                    let mut poi = self.poi.clone().unwrap();
                    poi.insert(
                        0,
                        PointOfInterest::new("Origin".to_string(), cfg.origin.0, cfg.origin.1, true, 0.0)
                    );
                    let (tx, rx) = channel();
                    self.mpsc_receiver = Some(rx);
                    std::thread::spawn(move || {
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        runtime.block_on(async move {
                            tx.send(ProgressBar::Init).unwrap();
                            let matrix = get_matrix(&poi, &filename)
                                .await
                                .expect("File input failed (PoI)");
                            tx.send(ProgressBar::Matrix).unwrap();
                            let tsp = TSPLookup::new(&matrix, &poi, &tx);
                            let mut solver = Solver::new(
                                &poi,
                                day_weights,
                                tsp,
                                upper_bound_random_size,
                                &tx,
                                &filename
                            );
                            solver.run();
                            let _ = solver.save();
                            tx.send(ProgressBar::Done).unwrap();
                        })
                    });
                }
                if save_btn.inner.clicked() {
                    self.save_trip_config();
                }
            });
        });
        egui::CentralPanel::default_margins().show(ui, |ui| {
            if let None = self.active_file {
                ui.heading("To edit a trip, create a new trip in the menu bar");
            }
            else {
                ui.heading("Configuration");
                let number_of_days = self.config.as_ref().unwrap().day_weights.len();
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Days ({})", number_of_days))
                            .on_hover_cursor(egui::CursorIcon::Help)
                            .on_hover_text("This parameter describes how much time every day has. If all days are equal, set them all to 1. However, if there is a shorter day (e.g. you're leaving in the evening or arriving midday), set it to a proportionally lower value, e.g. 0.5 for half of the day - this will assign proportionally less points of interest to that particular day.");
                        let mut idx_to_remove = None;
                        for (idx, day) in self.config.as_mut().unwrap().day_weights.iter_mut().enumerate() {
                            *day = (100.0 * *day).round() / 100.0;
                            ui.vertical(|ui|{
                                ui.add(
                                    egui::DragValue::new(day).max_decimals(2).range(0.0..=f32::INFINITY).speed(0.01)
                                );
                                if ui.button(" - ").clicked() {
                                    idx_to_remove = Some(idx);
                                }
                            });
                        }
                        if ui.button(" + ").clicked() {
                            self.config.as_mut().unwrap().day_weights.push(1.0);
                        }
                        if let Some(idx) = idx_to_remove {
                            let _ = self.config.as_mut().unwrap().day_weights.remove(idx);
                        }
                    });
                });

                if ui.button("Set origin")
                    .on_hover_text("This is the place from which you will be departing every day.")
                    .on_hover_cursor(egui::CursorIcon::Help)
                    .clicked() {
                    self.opened_popup = OpenedPopup::OriginMap;
                }
                ui.label(format!(
                    "Current origin: ({:.3}, {:.3})",
                    self.config.as_ref().unwrap().origin.0,
                    self.config.as_ref().unwrap().origin.1,
                ));
                if self.opened_popup == OpenedPopup::OriginMap {
                    let original_coords = self.config.as_ref().unwrap().origin;
                    let starting_pos = {
                        if original_coords.0 == 0.0 && original_coords.1 == 0.0 { None }
                        else { Some(original_coords) }
                    };
                    let res = self.map_popup(ui, starting_pos);
                    if let Some(v) = res {
                        self.config.as_mut().unwrap().origin = v;
                        self.opened_popup = OpenedPopup::None;
                    }
                }
                
                ui.separator();
                ui.heading("Points of Interest");
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::Grid::new("grid-poi").show(ui, |ui|{
                            let mut row_to_delete = None;
                            ui.label("Name");
                            ui.label("Location");
                            ui.label("Obligatory")
                                .on_hover_text("Do we have to stop here? Or is it just optional?")
                                .on_hover_cursor(egui::CursorIcon::Help);
                            ui.label("Duration")
                                .on_hover_text("This indicates how much time (in minutes) you intend to spend in the given place. Great for museums and cafes!")
                                .on_hover_cursor(egui::CursorIcon::Help);
                            ui.end_row();
                            for (idx, row) in self.poi.as_mut().unwrap().iter_mut().enumerate() {
                                ui.add_sized(
                                    [300.0, ui.spacing().interact_size.y],
                                    egui::TextEdit::singleline(&mut row.name),
                                );
                                if ui.button(format!("({:.2}, {:.2})", row.location.0, row.location.1)).clicked() {
                                    self.opened_popup = OpenedPopup::PoI(idx);
                                }
                                ui.checkbox(&mut row.obligatory, "");
                                ui.add(
                                    egui::DragValue::new(&mut row.duration).max_decimals(1).range(0.0..=f32::INFINITY)
                                );
                                if ui.button(" - ").clicked() {
                                    row_to_delete = Some(idx);
                                }
                                ui.end_row();
                            }
                            if ui.button(" + ").clicked() {
                                let start_pos = if let Some(v) = self.config.as_ref() {
                                    (v.origin.0, v.origin.1)
                                }
                                else { (0.0, 0.0) };
                                self.poi.as_mut().unwrap().push(PointOfInterest::new("".to_string(), start_pos.0, start_pos.1, false, 0.0))
                            }
                            if let Some(idx) = row_to_delete {
                                self.poi.as_mut().unwrap().remove(idx);
                            }
                        });
                    }
                );
                if let OpenedPopup::PoI(idx) = self.opened_popup {
                    let coords = self.poi.as_ref().unwrap().get(idx).unwrap().location;
                    let coords = (coords.0, coords.1);
                    let coords = if coords.0 == 0.0 && coords.1 == 0.0 { None } else { Some(coords) };
                    let res = self.map_popup(ui, coords);
                    if let Some((lat, lon)) = res {
                        self.poi.as_mut().unwrap().get_mut(idx).unwrap().location = (lat, lon);
                        self.opened_popup = OpenedPopup::None;
                    }
                }
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
        Box::new(|cc| Ok(Box::new(MyApp::new(cc))))
    )
}

pub fn run() -> Result<(), eframe::Error> {
    let is_api_key = get_api_key();
    if let Err(_) = is_api_key { return Ok(()); }
    if let Ok(false) = is_api_key { return Ok(()); }

    run_app()
}
