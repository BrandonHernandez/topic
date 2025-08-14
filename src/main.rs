use eframe::{egui, NativeOptions, egui::ViewportBuilder};
use directories::ProjectDirs;
use rusqlite::{params, Connection};
use std::fs;

use std::time::Duration;
use std::thread;

// Toasts
use egui_toast::{Toast, Toasts, ToastKind, ToastOptions};

use crossbeam_channel::{unbounded, Receiver, Sender};



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

// --------------------------- Hotkey wiring ---------------------------
mod ghk_win {
    use super::*;
    use windows_hotkeys::{HotkeyManager, HotkeyManagerImpl};
    use windows_hotkeys::keys::{ModKey, VKey};

    /// Runs in a background thread: registers key combination and dispatches events
    pub fn spawn_hotkey_listener(tx: Sender<()>) {
        thread::spawn(move || {
            let mut mgr = HotkeyManager::new();

            let res = mgr.register(VKey::N, &[ModKey::Ctrl, ModKey::Shift], {
                let tx = tx.clone();
                move || {
                    let _ = tx.send(());
                }
            }).expect("failed to register key combination");

            println!("{:?}", res);

            let res2 = mgr.register(VKey::A, &[ModKey::Ctrl, ModKey::Shift], {
                let tx = tx.clone();
                move || {
                    let _ = tx.send(());
                }
            }).expect("failed to register key combination");

            println!("{:?}", res2);

            // Blocks here; processes hotkey callbacks on this thread:
            mgr.event_loop();
        });
    }
}

// --------------------------- App ---------------------------
fn main() -> Result<(), eframe::Error> {    

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([300.0, 300.0])    
            .with_resizable(true)
            .with_visible(false),                 // this does nothing
        ..Default::default()
    };
    
    // Channel for hotkey events
    let (tx, rx) = unbounded::<()>();

    ghk_win::spawn_hotkey_listener(tx);

    // run_native() runs update() under the hood.
    eframe::run_native(
        "topic v1.0",
        options,
        Box::new(|_cc| Ok(Box::new(TopicApp::new(rx).expect("db open")))),
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
    // delimiter: usize,
    topic_index: usize,
    contents_index: usize,
    toasts: Toasts,
    close_after: Option<std::time::Instant>,
    hotkey_rx: Receiver<()>,
}

impl TopicApp {
    fn new(hotkey_rx: Receiver<()>) -> rusqlite::Result<Self> {

        let toasts = Toasts::new()
            .anchor(egui::Align2::RIGHT_BOTTOM, (-12.0, -12.0))
            .direction(egui::Direction::BottomUp);

        Ok(Self {
            raw_text: String::new(),
            first_frame: false,
            conn: open_db()?,
            topic: String::new(),
            content: String::new(),
            // delimiter: 0,
            topic_index: 0,
            contents_index: 0,
            toasts,
            close_after: None,
            hotkey_rx,
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
                    text: "Saving...".into(),
                    kind: ToastKind::Success,
                    options: ToastOptions::default().duration_in_seconds(0.75),
                    ..Default::default()
                });
                self.close_after = Some(std::time::Instant::now() + std::time::Duration::from_millis(750));

                // If saved, clear the raw text in self
                self.clear_raw_text();

                // These are to avoid saving an empty note after having saved a good note.
                self.clear_topic();
                self.clear_content();

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

    fn bring_up_input(&mut self, ctx: &egui::Context) {
        // Unminimize, bring to front, and focus input
        use egui::viewport::ViewportCommand as VC;
        ctx.send_viewport_cmd(VC::Minimized(false));
        ctx.send_viewport_cmd(VC::Focus);
        // Request focus to the text field on next frame:
        self.first_frame = false;
    }

    fn delimiter(&mut self) {
        let text_bytes = self.raw_text.as_bytes();

        let mut delimiter: [u8; 3] = [0, 0, 0];
        
        for i in 0..text_bytes.len() {
            
            // not the most elegant solution...
            if i + 2 < text_bytes.len()  {
                delimiter[0] = text_bytes[i + 0];
                delimiter[1] = text_bytes[i + 1];
                delimiter[2] = text_bytes[i + 2];
            }

            // println!("{:?}", delimiter);

            if delimiter[0] == b',' && delimiter[1] == b',' && delimiter[2] == b' ' {
                // self.delimiter = i;
                self.topic_index = i + 3;
                self.contents_index = i;
                return;
            }
        }

        // self.delimiter = 0;
        self.topic_index = 0;
        self.contents_index = 0;
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




}

impl eframe::App for TopicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        // React to global hotkey
        while self.hotkey_rx.try_recv().is_ok() {
            self.bring_up_input(ctx);
        }
        
        // Check for key presses
        // esc --> exit
        // enter --> save
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || ctx.input(|i| i.viewport().close_requested()) {
            // ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
            ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Minimized(true));
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.save_current(); 
            return;
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("content,, topic");

            // Create a persistent ID for the text field
            let id = ui.make_persistent_id("input");

            // On first frame, request focus for it
            if !self.first_frame {
                ui.memory_mut(|mem| mem.request_focus(id));
                self.first_frame = true;
            }

            // Draw the text input with the given ID
            ui.add(egui::TextEdit::singleline(&mut self.raw_text).id(id));

            // ui.text_edit_singleline(&mut self.text);
            
            // Search for the delimiter character in input
            self.delimiter();

            if !self.raw_text.is_empty() && self.topic_index > 0 && self.contents_index > 0 {
                self.set_topic();
                self.set_content();
                ui.label(format!("Topic: {}", self.get_topic()));
                ui.label(format!("Content: {}", self.get_content()));
            }

            // if !self.text.is_empty() {
            //     ui.label(format!("{}", self.text));
            // }

            // Draw toasts each frame (must be last so they overlay UI nicely)
            self.toasts.show(ctx);
            
            // close_time is Some() only when Save is successful, so no need to have a save_ok bool.
            if let Some(close_time) = self.close_after {
                if std::time::Instant::now() >= close_time {
                    // ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close)
                    ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Minimized(true));
                    self.close_after = None;
                }
            }
            
        });
    }
}