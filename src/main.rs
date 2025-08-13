use eframe::{egui, NativeOptions, egui::ViewportBuilder};
use directories::ProjectDirs;
use rusqlite::{params, Connection};
use std::fs;

// Toasts
use egui_toast::{Toast, Toasts, ToastKind, ToastOptions};

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
    topic: String,
    content: String,
    delimiter: usize,
    toasts: Toasts,
}

impl MyApp {
    fn new() -> rusqlite::Result<Self> {

        let toasts = Toasts::new()
            .anchor(egui::Align2::RIGHT_BOTTOM, (-12.0, -12.0))
            .direction(egui::Direction::BottomUp);

        Ok(Self {
            text: String::new(),
            first_frame: false,
            conn: open_db()?,
            topic: String::new(),
            content: String::new(),
            delimiter: 0,
            toasts,
        })
    }
    fn save_current(&mut self) {
        
        let topic_trimmed = self.topic.trim();
        let content_trimmed = self.content.trim();

        if topic_trimmed.is_empty() || content_trimmed.is_empty() {
            // Display status on a toast!
            self.toasts.add(Toast {
                text: "Nothing to save".into(),
                kind: ToastKind::Warning,
                options: ToastOptions::default().duration_in_seconds(2.0),
                ..Default::default()
            });

            return;
        }


        match self.conn.execute("INSERT INTO notes (topic, content) VALUES (?1, ?2)", params![topic_trimmed, content_trimmed]) {
            Ok(_) => {
                self.toasts.add(Toast {
                    text: "Saved".into(),
                    kind: ToastKind::Success,
                    options: ToastOptions::default().duration_in_seconds(2.0),
                    ..Default::default()
                });

            },
            Err(e) => {
                self.toasts.add(Toast {
                    text: format!("Error: {e}").into(),
                    kind: ToastKind::Error,
                    options: ToastOptions::default().duration_in_seconds(4.0),
                    ..Default::default()
                });    

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

    fn set_topic(&mut self) {
        self.topic = String::from(&self.text[(self.delimiter + 1)..]);
    }
    
    fn set_content(&mut self) {
        self.content = String::from(&self.text[..self.delimiter]);
    }

    fn get_topic(&mut self) -> &str {
        &self.topic
    }
    
    fn get_content(&mut self) -> &str {
        &self.content
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
            ui.heading("Create a note");

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
            
            // Search for the delimiter character in input
            self.delimiter();

            if !self.text.is_empty() && self.delimiter > 0 {
                self.set_topic();
                self.set_content();
                ui.label(format!("Topic: {}", self.get_topic().trim()));
                ui.label(format!("Content: {}", self.get_content().trim()));
            }

            // if !self.text.is_empty() {
            //     ui.label(format!("{}", self.text));
            // }

            // Draw toasts each frame (must be last so they overlay UI nicely)
            self.toasts.show(ctx);
            
        });
    }
}