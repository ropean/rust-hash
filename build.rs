#[cfg(all(windows, feature = "windows-icon"))]
fn main() {
    if let Ok(icon) = std::env::var("CARGO_CFG_WINDOWS_ICON_PATH") {
        let mut res = winres::WindowsResource::new();
        res.set_icon(&icon);
        let _ = res.compile();
    } else if let Ok(icon) = std::env::var("APP_ICON").or_else(|_| std::env::var("ICON")) {
        let mut res = winres::WindowsResource::new();
        res.set_icon(&icon);
        let _ = res.compile();
    }
}

#[cfg(not(all(windows, feature = "windows-icon")))]
fn main() {}


