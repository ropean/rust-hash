#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::path::Path;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use anyhow::{Context, Result};
use base64::Engine as _;
use iced::alignment::{Horizontal, Vertical};
use iced::executor;
use iced::theme;
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{clipboard, event, window, Application, Command, Element, Length, Settings, Subscription, Theme, Size};
// time subscription for periodic UI updates
use rfd::FileDialog;
use sha2::{Digest, Sha256};

fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.window.size = Size::new(900.0, 560.0);
    settings.window.resizable = true;
    settings.window.min_size = Some(Size::new(900.0, 420.0));
    settings.window.position = window::Position::Centered;
    // Try to set window icon from env/paths, then embedded ICO fallback
    settings.window.icon = try_load_icon_from_env()
        .or_else(|| try_load_icon_from_paths())
        .or_else(|| load_embedded_icon());
    App::run(settings)
}

#[derive(Debug, Clone)]
enum Message {
    PathChanged(String),
    BrowsePressed,
    ClearPressed,
    CancelPressed,
    CopyHex,
    CopyBase64,
    UppercaseToggled(bool),
    AutoHashToggled(bool),
    DroppedFile(PathBuf),
    StartHash,
    Tick,
    Ignored,
}

#[derive(Debug, Clone)]
struct HashResult {
    hex: String,
    base64: String,
    elapsed: Duration,
    bytes: u64,
    path: Option<PathBuf>,
}

#[derive(Default)]
struct App {
    // Input
    path_input: String,
    // Output
    hex_output: String,
    base64_output: String,
    // State
    is_hashing: bool,
    error: Option<String>,
    uppercase: bool,
    auto_hash: bool,
    started_at: Option<Instant>,
    last_elapsed: Option<Duration>,
    last_bytes: Option<u64>,
    last_path: Option<PathBuf>,
    prev_path_before_hash: Option<String>,
    // Progress
    progress_total: Option<u64>,
    progress_processed: u64,
    progress_counter: Option<Arc<AtomicU64>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    worker_rx: Option<Receiver<(u64, std::result::Result<HashResult, String>)>>,
    worker_token: Option<u64>,
    // Concurrency token to ignore late results
    token: u64,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut app = App::default();
        app.auto_hash = true;
        (app, Command::none())
    }

    fn title(&self) -> String {
        if self.is_hashing {
            if let Some(total) = self.progress_total {
                if total > 0 {
                    let pct = ((self.progress_processed as f64 / total as f64) * 100.0).clamp(0.0, 100.0);
                    return format!("Rust Hash256 v{} - {:.0}% ", app_version(), pct);
                }
            }
            return format!("Rust Hash256 v{} - hashing... ", app_version());
        }
        format!("Rust Hash256 v{} ", app_version())
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let file_drop = event::listen().map(|e| match e {
            event::Event::Window(_, window::Event::FileDropped(path)) => Message::DroppedFile(path),
            _ => Message::Ignored,
        });
        let tick = iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick);
        Subscription::batch(vec![file_drop, tick])
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::PathChanged(value) => {
                let old_path = self.path_input.clone();
                self.path_input = value;
                self.error = None;
                if self.auto_hash && !self.path_input.trim().is_empty() && !self.is_hashing {
                    self.start_hashing(self.path_input.clone(), Some(old_path));
                    return Command::none();
                }
                Command::none()
            }
            Message::BrowsePressed => {
                let mut dialog = FileDialog::new();
                // Try to start from previous/current path when available
                if !self.path_input.trim().is_empty() {
                    let p = PathBuf::from(&self.path_input);
                    if p.is_dir() {
                        dialog = dialog.set_directory(&p);
                    } else if let Some(parent) = p.parent() {
                        if parent.is_dir() {
                            dialog = dialog.set_directory(parent);
                        }
                    }
                } else if let Some(p) = &self.last_path {
                    if p.is_dir() {
                        dialog = dialog.set_directory(p);
                    } else if let Some(parent) = p.parent() {
                        if parent.is_dir() {
                            dialog = dialog.set_directory(parent);
                        }
                    }
                }
                if let Some(path) = dialog.pick_file() {
                    let old_path = self.path_input.clone();
                    self.path_input = path.to_string_lossy().to_string();
                    self.error = None;
                    if self.auto_hash {
                        self.start_hashing(self.path_input.clone(), Some(old_path));
                        return Command::none();
                    }
                }
                Command::none()
            }
            Message::ClearPressed => {
                self.path_input.clear();
                self.hex_output.clear();
                self.base64_output.clear();
                self.error = None;
                self.last_elapsed = None;
                self.last_bytes = None;
                self.last_path = None;
                self.progress_total = None;
                self.progress_processed = 0;
                Command::none()
            }
            Message::CancelPressed => {
                if let Some(flag) = &self.cancel_flag {
                    flag.store(true, Ordering::Relaxed);
                }
                // Try to restore previous path when possible
                if let Some(prev) = self.prev_path_before_hash.take() {
                    self.path_input = prev;
                } else if let Some(p) = &self.last_path {
                    self.path_input = p.to_string_lossy().to_string();
                }
                self.is_hashing = false;
                self.progress_total = None;
                self.progress_processed = 0;
                self.worker_rx = None;
                Command::none()
            }
            Message::CopyHex => clipboard::write(self.hex_output.clone()),
            Message::CopyBase64 => clipboard::write(self.base64_output.clone()),
            Message::UppercaseToggled(v) => {
                self.uppercase = v;
                if !self.hex_output.is_empty() {
                    if self.uppercase {
                        self.hex_output = self.hex_output.to_uppercase();
                    } else {
                        self.hex_output = self.hex_output.to_lowercase();
                    }
                }
                Command::none()
            }
            Message::AutoHashToggled(v) => {
                self.auto_hash = v;
                Command::none()
            }
            Message::DroppedFile(path) => {
                let old_path = self.path_input.clone();
                self.path_input = path.to_string_lossy().to_string();
                self.error = None;
                if self.auto_hash {
                    self.start_hashing(self.path_input.clone(), Some(old_path));
                    return Command::none();
                }
                Command::none()
            }
            Message::StartHash => {
                if !self.path_input.trim().is_empty() && !self.is_hashing {
                    self.start_hashing(self.path_input.clone(), None);
                    return Command::none();
                }
                Command::none()
            }
            Message::Tick => {
                if self.is_hashing {
                    if let Some(counter) = &self.progress_counter {
                        self.progress_processed = counter.load(Ordering::Relaxed);
                    }
                    if let Some(rx) = &self.worker_rx {
                        if let Ok((token, result)) = rx.try_recv() {
                            if token == self.token {
                                self.is_hashing = false;
                                match result {
                                    Ok(hr) => {
                                        self.error = None;
                                        self.hex_output = if self.uppercase { hr.hex.to_uppercase() } else { hr.hex };
                                        self.base64_output = hr.base64;
                                        self.last_elapsed = Some(hr.elapsed);
                                        self.last_bytes = Some(hr.bytes);
                                        self.last_path = hr.path;
                                    }
                                    Err(e) => {
                                        if e == "CANCELLED" {
                                            // Already restored path in CancelPressed
                                            self.error = None;
                                        } else {
                                            self.error = Some(e);
                                            self.hex_output.clear();
                                            self.base64_output.clear();
                                            self.last_elapsed = None;
                                            self.last_bytes = None;
                                            self.last_path = None;
                                        }
                                    }
                                }
                                self.progress_total = None;
                                self.progress_processed = 0;
                                self.progress_counter = None;
                                self.cancel_flag = None;
                                self.worker_rx = None;
                                self.worker_token = None;
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::Ignored => Command::none(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let title = text("Rust Hash256").size(28);

        let path_input = text_input("Drag a file here or paste path...", &self.path_input)
            .on_input(Message::PathChanged)
            .on_submit(Message::StartHash)
            .padding(12)
            .size(16)
            .width(Length::Fill);

        let browse_btn = if self.is_hashing {
            button(text("Browse").size(16)).style(theme::Button::Secondary)
        } else {
            button(text("Browse").size(16)).on_press(Message::BrowsePressed)
        };

        let clear_btn = if self.is_hashing {
            button(text("Clear").size(16)).style(theme::Button::Secondary)
        } else {
            button(text("Clear").size(16)).on_press(Message::ClearPressed)
        };

        let cancel_btn: Option<Element<'_, Message>> = if self.is_hashing {
            Some(button(text("Cancel").size(16)).on_press(Message::CancelPressed).style(theme::Button::Primary).into())
        } else {
            None
        };

        let toggles = row![
            checkbox("Uppercase HEX", self.uppercase).on_toggle(Message::UppercaseToggled),
            checkbox("Auto hash on select", self.auto_hash).on_toggle(Message::AutoHashToggled),
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center);

        let header = if let Some(c) = cancel_btn {
            row![path_input, browse_btn, clear_btn, c]
                .spacing(10)
                .align_items(iced::Alignment::Center)
        } else {
            row![path_input, browse_btn, clear_btn]
                .spacing(10)
                .align_items(iced::Alignment::Center)
        };

        let drag_hint = container(text("Drop a file anywhere in this window to hash").size(14))
            .width(Length::Fill)
            .padding(6);

        let outputs = column![
            labeled_value(
                "SHA-256 (HEX)",
                &self.hex_output,
                Message::CopyHex,
                "Copy HEX",
                self.is_hashing,
            ),
            labeled_value(
                "SHA-256 (Base64)",
                &self.base64_output,
                Message::CopyBase64,
                "Copy Base64",
                self.is_hashing,
            ),
        ]
        .spacing(12);

        let meta = meta_info(self.is_hashing, self.last_elapsed, self.last_bytes.as_ref(), self.last_path.as_ref(), self.error.as_ref());

        let content = column![title, header, toggles, drag_hint, outputs, meta]
            .spacing(16)
            .padding(16)
            .max_width(900)
            .align_items(iced::Alignment::Start);

        scrollable(container(content).width(Length::Fill))
            .height(Length::Fill)
            .into()
    }
}

fn labeled_value<'a>(label: &str, value: &str, copy_msg: Message, copy_label: &str, disabled: bool) -> Element<'a, Message> {
    let label_widget = text(label).size(16);
    let value_widget = text(if value.is_empty() { "-" } else { value })
        .size(15)
        .width(Length::Fill);

    let copy_btn = if value.is_empty() || disabled {
        button(text("Copy")).style(theme::Button::Secondary)
    } else {
        button(text(copy_label)).on_press(copy_msg).style(theme::Button::Secondary).width(Length::Fixed(110.0))
    };

    row![
        container(label_widget)
            .width(Length::Fixed(200.0))
            .align_x(Horizontal::Left)
            .align_y(Vertical::Center),
        container(value_widget).padding(10).width(Length::Fill),
        copy_btn,
    ]
    .spacing(10)
    .align_items(iced::Alignment::Center)
    .into()
}

fn meta_info(
    is_hashing: bool,
    elapsed: Option<Duration>,
    bytes: Option<&u64>,
    path: Option<&PathBuf>,
    error: Option<&String>,
) -> Element<'static, Message> {
    let mut parts: Vec<Element<'static, Message>> = Vec::new();
    if let Some(p) = path {
        let s = format!("{}", p.display());
        parts.push(text(s).size(14).into());
    }
    if let Some(e) = error {
        parts.push(text(format!("{}", e)).style(theme::Text::Color([1.0, 0.5, 0.5].into())).into());
    } else {
        if let (Some(el), Some(b)) = (elapsed, bytes) {
            let secs = el.as_secs_f64();
            let speed = if secs > 0.0 { (*b as f64) / secs } else { 0.0 };
            let speed_human = human_bytes(speed);
            let b_human = human_bytes(*b as f64);
            parts.push(text(format!("{} • {} • {}/s", human_duration(el), b_human, speed_human)).size(14).into());
        } else if is_hashing {
            parts.push(text("Hashing...").size(14).into());
        }
    }

    column(parts)
        .spacing(6)
        .padding(6)
        .into()
}

fn human_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{} ms", ms)
    } else {
        format!("{:.2} s", (ms as f64) / 1000.0)
    }
}

fn human_bytes(b: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut val = b;
    let mut idx = 0;
    while val >= 1024.0 && idx < UNITS.len() - 1 {
        val /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{:.0} {}", val, UNITS[idx])
    } else {
        format!("{:.2} {}", val, UNITS[idx])
    }
}

// old async hash and non-progress variant removed (no longer used)

impl App {
    fn next_token(&mut self) -> u64 {
        self.is_hashing = true;
        self.error = None;
        self.started_at = Some(Instant::now());
        self.token = self.token.wrapping_add(1);
        self.token
    }

    fn start_hashing(&mut self, path: String, prev: Option<String>) {
        let token = self.next_token();
        self.prev_path_before_hash = prev.or_else(|| Some(self.path_input.clone()));
        let (tx, rx): (Sender<(u64, std::result::Result<HashResult, String>)>, Receiver<_>) = mpsc::channel();
        let progress = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));

        // Determine total size if possible (for progress)
        let total = std::fs::metadata(&path).ok().map(|m| m.len());
        self.progress_total = total;
        self.progress_processed = 0;
        self.progress_counter = Some(progress.clone());
        self.cancel_flag = Some(cancel.clone());
        self.worker_rx = Some(rx);
        self.worker_token = Some(token);

        thread::spawn(move || {
            let started = Instant::now();
            let result: std::result::Result<HashResult, String> = compute_sha256_file_progress(&path, progress, cancel)
                .map(|(hex, b64, bytes, path)| HashResult { hex, base64: b64, elapsed: started.elapsed(), bytes, path })
                .map_err(|e| format!("{}", e));
            let _ = tx.send((token, result));
        });
    }
}

fn compute_sha256_file_progress(path_str: &str, progress: Arc<AtomicU64>, cancel: Arc<AtomicBool>) -> Result<(String, String, u64, Option<PathBuf>)> {
    let path = PathBuf::from(path_str);
    let file = File::open(&path).with_context(|| format!("Failed to open file: {}", path_str))?;
    let metadata = file.metadata().ok();
    let mut reader = BufReader::with_capacity(1024 * 1024, file); // 1 MiB buffer
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut total: u64 = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("CANCELLED"));
        }
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        total += n as u64;
        progress.store(total, Ordering::Relaxed);
    }
    let digest = hasher.finalize();
    let bytes = digest.as_slice();
    let hex = hex::encode(bytes);
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok((hex, b64, metadata.map(|m| m.len()).unwrap_or(total), Some(path)))
}

fn try_load_icon_from_env() -> Option<window::Icon> {
    if let Ok(icon_path) = std::env::var("APP_ICON").or_else(|_| std::env::var("ICON")) {
        if let Ok(icon) = window::icon::from_file(Path::new(&icon_path)) {
            return Some(icon);
        }
    }
    None
}

fn try_load_icon_from_paths() -> Option<window::Icon> {
    let candidates = [
        Path::new("assets/app.ico").to_path_buf(),
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("assets/app.ico"))).unwrap_or_else(|| PathBuf::from("assets/app.ico")),
    ];
    for p in candidates {
        if let Ok(icon) = window::icon::from_file(&p) {
            return Some(icon);
        }
    }
    None
}

fn load_embedded_icon() -> Option<window::Icon> {
    // Fallback: embed ICO at compile-time and load it via a temp file
    const EMBEDDED_ICO: &[u8] = include_bytes!("../assets/app.ico");
    if EMBEDDED_ICO.is_empty() {
        return None;
    }
    let temp_path = std::env::temp_dir().join("rust-hash-app.ico");
    if std::fs::write(&temp_path, EMBEDDED_ICO).is_ok() {
        if let Ok(icon) = window::icon::from_file(&temp_path) {
            return Some(icon);
        }
    }
    None
}

fn app_version() -> &'static str {
    // Prefer runtime env APP_VERSION injected by CI; fallback to Cargo package version
    static VERSION: once_cell::sync::Lazy<String> = once_cell::sync::Lazy::new(|| {
        std::env::var("APP_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
    });
    &VERSION
}


