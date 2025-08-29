use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use base64::Engine as _;
use iced::alignment::{Horizontal, Vertical};
use iced::executor;
use iced::theme;
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{clipboard, event, window, Application, Command, Element, Length, Settings, Subscription, Theme, Size};
use rfd::FileDialog;
use sha2::{Digest, Sha256};

fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.window.size = Size::new(800.0, 560.0);
    settings.window.resizable = true;
    settings.window.min_size = Some(Size::new(640.0, 420.0));
    App::run(settings)
}

#[derive(Debug, Clone)]
enum Message {
    PathChanged(String),
    BrowsePressed,
    ClearPressed,
    CopyHex,
    CopyBase64,
    UppercaseToggled(bool),
    AutoHashToggled(bool),
    DroppedFile(PathBuf),
    StartHash,
    HashFinished { token: u64, result: std::result::Result<HashResult, String> },
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
        "Rust Hash256".to_string()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        event::listen().map(|e| match e {
            event::Event::Window(_, window::Event::FileDropped(path)) => Message::DroppedFile(path),
            _ => Message::Ignored,
        })
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::PathChanged(value) => {
                self.path_input = value;
                self.error = None;
                if self.auto_hash && !self.path_input.trim().is_empty() && !self.is_hashing {
                    return Command::perform(start_hash(self.next_token(), self.path_input.clone()), |r| r);
                }
                Command::none()
            }
            Message::BrowsePressed => {
                if let Some(path) = FileDialog::new().pick_file() {
                    self.path_input = path.to_string_lossy().to_string();
                    self.error = None;
                    if self.auto_hash {
                        return Command::perform(start_hash(self.next_token(), self.path_input.clone()), |r| r);
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
                self.path_input = path.to_string_lossy().to_string();
                self.error = None;
                if self.auto_hash {
                    return Command::perform(start_hash(self.next_token(), self.path_input.clone()), |r| r);
                }
                Command::none()
            }
            Message::StartHash => {
                if !self.path_input.trim().is_empty() && !self.is_hashing {
                    return Command::perform(start_hash(self.next_token(), self.path_input.clone()), |r| r);
                }
                Command::none()
            }
            Message::HashFinished { token, result } => {
                if token != self.token {
                    // Stale result; ignore
                    return Command::none();
                }
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
                        self.error = Some(e);
                        self.hex_output.clear();
                        self.base64_output.clear();
                        self.last_elapsed = None;
                        self.last_bytes = None;
                        self.last_path = None;
                    }
                }
                Command::none()
            }
            Message::Ignored => Command::none(),
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let title = text("Rust Hash256 🔒").size(28);

        let path_input = text_input("Drag a file here or paste path...", &self.path_input)
            .on_input(Message::PathChanged)
            .padding(12)
            .size(16)
            .width(Length::Fill);

        let browse_btn = button(text("📁 Browse").size(16)).on_press(Message::BrowsePressed);

        let clear_btn = button(text("🧹 Clear").size(16)).on_press(Message::ClearPressed);

        let start_btn = button(if self.is_hashing { text("⏳ Hashing...") } else { text("⚙️ Hash") })
            .on_press(Message::StartHash)
            .style(theme::Button::Primary);

        let toggles = row![
            checkbox("Uppercase HEX", self.uppercase).on_toggle(Message::UppercaseToggled),
            checkbox("Auto hash on select", self.auto_hash).on_toggle(Message::AutoHashToggled),
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center);

        let header = row![path_input, browse_btn, clear_btn, start_btn]
            .spacing(10)
            .align_items(iced::Alignment::Center);

        let drag_hint = container(text("Drop a file anywhere in this window to hash ⤵").size(14))
            .width(Length::Fill)
            .padding(6);

        let outputs = column![
            labeled_value(
                "SHA-256 (HEX)",
                &self.hex_output,
                Message::CopyHex,
                "📋 Copy HEX",
            ),
            labeled_value(
                "SHA-256 (Base64)",
                &self.base64_output,
                Message::CopyBase64,
                "📋 Copy Base64",
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

fn labeled_value<'a>(label: &str, value: &str, copy_msg: Message, copy_label: &str) -> Element<'a, Message> {
    let label_widget = text(label).size(16);
    let value_widget = text(if value.is_empty() { "—" } else { value })
        .size(15)
        .width(Length::Fill);

    let copy_btn = if value.is_empty() {
        button(text("📋 Copy")).style(theme::Button::Secondary)
    } else {
        button(text(copy_label)).on_press(copy_msg).style(theme::Button::Secondary)
    };

    row![
        container(label_widget)
            .width(Length::Fixed(180.0))
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
        let s = format!("📄 {}", p.display());
        parts.push(text(s).size(14).into());
    }
    if let Some(e) = error {
        parts.push(text(format!("❗ {}", e)).style(theme::Text::Color([1.0, 0.5, 0.5].into())).into());
    } else {
        if let (Some(el), Some(b)) = (elapsed, bytes) {
            let secs = el.as_secs_f64();
            let speed = if secs > 0.0 { (*b as f64) / secs } else { 0.0 };
            let speed_human = human_bytes(speed);
            let b_human = human_bytes(*b as f64);
            parts.push(text(format!("⏱️ {} • {} • {}/s", human_duration(el), b_human, speed_human)).size(14).into());
        } else if is_hashing {
            parts.push(text("⏳ Hashing...").size(14).into());
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

fn start_hash(token: u64, path_str: String) -> impl std::future::Future<Output = Message> {
    async move {
        let started = Instant::now();
        let res = compute_sha256_file(&path_str)
            .map(|(hex, b64, bytes, path)| HashResult { hex, base64: b64, elapsed: started.elapsed(), bytes, path })
            .map_err(|e| format!("{}", e));
        Message::HashFinished { token, result: res }
    }
}

fn compute_sha256_file(path_str: &str) -> Result<(String, String, u64, Option<PathBuf>)> {
    let path = PathBuf::from(path_str);
    let file = File::open(&path).with_context(|| format!("Failed to open file: {}", path_str))?;
    let metadata = file.metadata().ok();
    let mut reader = BufReader::with_capacity(1024 * 1024, file); // 1 MiB buffer
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        total += n as u64;
    }
    let digest = hasher.finalize();
    let bytes = digest.as_slice();
    let hex = hex::encode(bytes);
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok((hex, b64, metadata.map(|m| m.len()).unwrap_or(total), Some(path)))
}

impl App {
    fn next_token(&mut self) -> u64 {
        self.is_hashing = true;
        self.error = None;
        self.started_at = Some(Instant::now());
        self.token = self.token.wrapping_add(1);
        self.token
    }
}


