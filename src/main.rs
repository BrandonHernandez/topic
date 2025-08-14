use eframe::{egui, NativeOptions, egui::ViewportBuilder};
// use directories::ProjectDirs;
use rusqlite::{params, Connection};
// use std::fs;

use std::time::{Instant};

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
    std::path::PathBuf::from("data_test.db")
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
// --------------------------- App ---------------------------
fn main() -> Result<(), eframe::Error> {    

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([300.0, 150.0])    
            .with_resizable(true)
            .with_visible(false),                 // this does nothing
        ..Default::default()
    };

    // run_native() runs update() under the hood.
    eframe::run_native(
        "topic v1.0",
        options,
        Box::new(|_cc| Ok(Box::new(TopicApp::new().expect("db open")))),
    )
}

// #[derive(Default)]
struct TopicApp {
    // counter: i32,
    raw_text: String,
    first_frame: bool,
    conn: Connection,
    topic: String,
    content: String,
    topic_index: usize,
    contents_index: usize,
    toasts: Toasts,
    close_after: Option<Instant>,
    cmd_mode: bool,
    cmd: CMD,
}

enum CMD {
    Show,
    Exit,
    None,
}

impl TopicApp {
    fn new() -> rusqlite::Result<Self> {

        let toasts = Toasts::new()
            .anchor(egui::Align2::RIGHT_BOTTOM, (-12.0, -12.0))
            .direction(egui::Direction::BottomUp);

        Ok(Self {
            raw_text: String::new(),
            first_frame: false,
            conn: open_db()?,
            topic: String::new(),
            content: String::new(),
            topic_index: 0,
            contents_index: 0,
            toasts,
            close_after: None,
            cmd_mode: false,
            cmd: CMD::None,
        })
    }
    fn save_current(&mut self) {

        if self.get_topic().is_empty() || self.get_content().is_empty() {
            // Display status on a toast!
            let mut toast_text: String = String::new();

            if self.get_topic().is_empty() {
                toast_text = String::from("Provide a topic!");
            }
            if self.get_content().is_empty() {
                toast_text = String::from("Your note needs content!");
            }

            self.toasts.add(Toast {
                text: toast_text.into(),
                kind: ToastKind::Warning,
                options: ToastOptions::default().duration_in_seconds(1.25),
                ..Default::default()
            });

            // Set first_frame to false so focus can be brought back into the text input.
            self.first_frame = false;

            return;
        }

        let topic = self.get_topic();
        let content = self.get_content();

        match self.conn.execute("INSERT INTO notes (topic, content) VALUES (?1, ?2)", params![topic, content]) {
            Ok(_) => {
                self.toasts.add(Toast {
                    text: "Saved".into(),
                    kind: ToastKind::Success,
                    options: ToastOptions::default().duration_in_seconds(0.75),
                    ..Default::default()
                });
                self.close_after = Some(std::time::Instant::now() + std::time::Duration::from_millis(750));

                // If saved, clear the raw text in self
                self.clear_raw_text();

                // // These are to avoid saving an empty note after having saved a good note.
                // self.clear_topic();
                // self.clear_content();

                // Set first_frame to false so focus can be brought back into the text input.
                self.first_frame = false;

            },
            Err(e) => {
                self.toasts.add(Toast {
                    text: format!("Error: {e}").into(),
                    kind: ToastKind::Error,
                    options: ToastOptions::default().duration_in_seconds(4.0),
                    ..Default::default()
                });    
                self.first_frame = false;
            },
        }



    }

    fn search_separator(&mut self) {
        let text_bytes = self.raw_text.as_bytes();

        let mut separator: [u8; 3] = [0, 0, 0];
        
        self.topic_index = 0;
        self.contents_index = 0;
        self.cmd_mode = false;

        for i in 0..text_bytes.len() {
            // We are in cmd mode anytime `:` is the first char.
            if text_bytes[0] == b':' {
                self.cmd_mode = true;
                return;
            }
            // not the most elegant solution...
            if i + 2 < text_bytes.len()  {
                separator[0] = text_bytes[i];
                separator[1] = text_bytes[i + 1];
                separator[2] = text_bytes[i + 2];
            }

            if separator[0] == b':' && separator[1] == b':' && separator[2] == b':' {
                self.topic_index = i + 3;
                self.contents_index = i;
                return;
            }

        }
    }

    fn set_topic(&mut self) {
        self.topic = String::from(String::from(&self.raw_text[self.topic_index..]).trim());
    }
    
    fn set_content(&mut self) {
        self.content = String::from(String::from(&self.raw_text[..self.contents_index]).trim());
    }

    fn get_topic(&mut self) -> String {
        self.topic.clone()
    }
    
    fn get_content(&mut self) -> String {
        self.content.clone()
    }

    fn clear_raw_text(&mut self) {
        self.raw_text = String::new();
    }

    fn clear_topic(&mut self) {
        self.topic = String::new();
    }
    
    fn clear_content(&mut self) {
        self.content = String::new();
    }

    fn get_set_cmd(&mut self) {
        // Avoid invalid index
        if self.raw_text.len() > 4 {
            let cmd = &self.raw_text[1..5]; //Get 4 chars after ":" at [0]
            let cmd = cmd.to_lowercase();
            let cmd = cmd.as_str();
            match cmd {
                "exit" => {
                    self.cmd = CMD::Exit;
                    return;
                },
                "show" => {
                    self.cmd = CMD::Show;
                    return;
                },
                _ => (),
            }
        }
        // If above code did not run
        self.cmd = CMD::None;
    }




}

impl eframe::App for TopicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for key presses
        // esc --> exit
        // enter --> save
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
            // ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Minimized(true));
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !self.cmd_mode {
            self.save_current(); 
            return;
        }
        
        egui::CentralPanel::default().show(&ctx, |ui| {
            // title
            ui.heading("content:::topic");

            // Create a persistent ID for the text field
            let id = ui.make_persistent_id("input");

            // On first frame, request focus for it
            if !self.first_frame {
                ui.memory_mut(|mem| mem.request_focus(id));
                self.first_frame = true;
            }

            // ~~~Space~~~
            ui.label("");

            // Draw the text input with the given ID
            ui.add(egui::TextEdit::singleline(&mut self.raw_text).id(id));
            // ui.text_edit_singleline(&mut self.text);

            // ~~~Space~~~
            ui.label("");
            
            // Always clear topic and content to avoid leftover data
            self.clear_topic();
            self.clear_content();

            // Search for the separator in text input
            self.search_separator();

            // If the separator is found, topic_index and contents_index will be greater than 0.
            if self.topic_index > 0 && self.contents_index > 0 {
                // These setters use the indices internally to separate content from topic.
                // The texts are trimmed internally.
                self.set_topic();
                self.set_content();
                ui.label(format!("Topic: {}", self.get_topic()));
                ui.label(format!("{}", self.get_content()));
            }

            // println!("Command mode: {}", self.cmd_mode);

            if self.cmd_mode {
                ui.label(egui::RichText::new("cmd mode").color(egui::Color32::RED));
                // The following abomination turns text cmd into enum cmd. Sets self.cmd internally.
                self.get_set_cmd();
                
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    match self.cmd {
                        CMD::Show => {
                            // This needs to be worked on...
                            // let results = self.conn.prepare("SELECT * FROM notes");
                            // println!("{:#?}", results);
                        },
                        CMD::Exit => {
                            ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
                        },
                        CMD::None => {
                            self.toasts.add(Toast {
                                text: format!("Not a cmd").into(),
                                kind: ToastKind::Error,
                                options: ToastOptions::default().duration_in_seconds(0.750),
                                ..Default::default()
                            });    
                            // Bring focus back to text input if bad command is entered (focus is lost when pressing enter)
                            self.first_frame = false;
                        },
                    }
                }
                
            }

            // Draw toasts each frame (must be last so they overlay UI nicely)
            self.toasts.show(ctx);
            
            // close_time is Some() only when Save is successful, so no need to have a save_ok bool.
            if let Some(close_time) = self.close_after {
                if std::time::Instant::now() >= close_time {
                    // ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close)
                    // ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Minimized(true));
                    self.close_after = None;
                }
            }
        });
    }
}