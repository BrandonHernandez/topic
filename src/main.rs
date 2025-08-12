use eframe::{egui, NativeOptions, egui::ViewportBuilder};
use directories::ProjectDirs;
use rusqlite::{params, Connection};
use std::fs;

// This method creates the data.sqlite file in C:\Users\bhernandez\AppData\Roaming\ExampleOrg...
// Why don't we just put it next to the executable? Let's do that... 

// fn db_path() -> std::path::PathBuf {
//     let proj = ProjectDirs::from("com", "ExampleOrg", "EguiTextInput").unwrap();
//     let dir = proj.data_dir();
//     fs::create_dir_all(dir).ok();
//     dir.join("data.sqlite")
// }

fn db_path() -> std::path::PathBuf {
    std::path::PathBuf::from("data.sqlite")
}

fn open_db() -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path())?;
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS notes (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            text        TEXT NOT NULL,
            topic       TEXT NOT NULL,
            content     TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
        [],
    )?;
    Ok(conn)
}

fn main() -> Result<(), eframe::Error> {    

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
        .with_inner_size([300.0, 100.0])    // Small frame
        .with_resizable(false),             // Optional: fixed size
        ..Default::default()
    };

    // let options = eframe::NativeOptions::default();
    
    // run_native() runs update() under the hood.
    eframe::run_native(
        "topic v1.0",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new().expect("db open")))),
    )
}

// #[derive(Default)]
struct MyApp {
    // counter: i32,
    text: String,
    first_frame: bool,
    conn: Connection,
    status: String,
    topic: String,
    content: String,
    delimiter: usize,
}

impl MyApp {
    fn new() -> rusqlite::Result<Self> {
        Ok(Self {
            text: String::new(),
            first_frame: false,
            conn: open_db()?,
            status: String::new(),
            topic: String::new(),
            content: String::new(),
            delimiter: 0,
        })
    }
    fn save_current(&mut self) {
        // save topic and content, not text
        // PENDING ^^
        if self.text.trim().is_empty() {
            self.status = String::from("Nothing to save.");
            return;
        }
        let trimmed = self.text.trim();
        match self.conn.execute("INSERT INTO notes (text) VALUES (?1)", params![trimmed]) {
            Ok(_) => {
                self.status = String::from("Saved");
                // self.text.clear();
            },
            Err(e) => {
                self.status = format!("Error: {e}");
            },
        }
    }
    fn delimiter(&mut self){
        let text_bytes = self.text.as_bytes();
        
        for (i, &item) in text_bytes.iter().enumerate() {
            if item == b'.' {
                self.delimiter = i;
                return;
            }
        }

        self.delimiter = 0;
    }

    fn topic(&mut self) -> &str {
        &self.text[self.delimiter + 1..]
    }
    
    fn content(&mut self) -> &str {
        &self.text[..self.delimiter]
    }
}


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for key presses
        // esc --> exit
        // enter --> save
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.save_current();
            ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
            return;
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // ui.heading("Hello from egui!");
            // ui.label("Type something:");

            // Create a persistent ID for the text field
            let id = ui.make_persistent_id("input");

            // On first frame, request focus for it
            if !self.first_frame {
                ui.memory_mut(|mem| mem.request_focus(id));
                self.first_frame = true;
            }

            // Draw the text input with the given ID
            ui.add(egui::TextEdit::singleline(&mut self.text).id(id));

            // ui.text_edit_singleline(&mut self.text);
            
            self.delimiter();

            if !self.text.is_empty() && self.delimiter > 0 {
                ui.label(format!("Topic: {}", self.topic().trim()));
                ui.label(format!("Content: {}", self.content().trim()));
            }

            // if !self.text.is_empty() {
            //     ui.label(format!("{}", self.text));
            // }

            // The status is never seen. Only gets contents when hitting save, but that closes the app, so...
            // if !self.status.is_empty() {
            //     ui.label(format!("{}", self.status));
            // }

            // if ui.button("Click me").clicked() {
            //     self.counter += 1;
            // }
            // ui.label(format!("Counter: {}", self.counter));
        });
    }
}