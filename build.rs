#[cfg(all(windows, feature = "windows-icon"))]
fn main() {
    let version = std::env::var("APP_VERSION")
        .or_else(|_| std::env::var("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| "0.0.0".to_string());
    let version4 = to_winver4(&version);

    let mut res = winres::WindowsResource::new();

    if let Ok(icon) = std::env::var("CARGO_CFG_WINDOWS_ICON_PATH") {
        res.set_icon(&icon);
    } else if let Ok(icon) = std::env::var("APP_ICON").or_else(|_| std::env::var("ICON")) {
        res.set_icon(&icon);
    }

    // Package metadata
    let _pkg_name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "rust-hash".to_string());
    let pkg_desc = std::env::var("CARGO_PKG_DESCRIPTION").unwrap_or_else(|_| "SHA-256 file hasher".to_string());
    let pkg_authors = std::env::var("CARGO_PKG_AUTHORS").unwrap_or_else(|_| "".to_string());
    let pkg_license = std::env::var("CARGO_PKG_LICENSE").unwrap_or_else(|_| "".to_string());

    let product_name = "Rust Hash";
    let original_filename = "rust-hash.exe";
    let internal_name = "rust-hash";
    let comments = format!("Built from version {}", &version);

    // Version/info metadata
    let _ = res.set("FileVersion", &version4);
    let _ = res.set("ProductVersion", &version4);
    let _ = res.set("ProductName", product_name);
    let _ = res.set("FileDescription", &pkg_desc);
    let _ = res.set("OriginalFilename", original_filename);
    let _ = res.set("InternalName", internal_name);
    if !pkg_authors.is_empty() { let _ = res.set("CompanyName", &pkg_authors); }
    if !pkg_license.is_empty() { let _ = res.set("LegalCopyright", &pkg_license); }
    let _ = res.set("Comments", &comments);

    let _ = res.compile();
}

#[cfg(not(all(windows, feature = "windows-icon")))]
fn main() {}

#[cfg(all(windows, feature = "windows-icon"))]
fn to_winver4(tag: &str) -> String {
    // Strip leading 'v' or 'V' if present and keep only digits and dots
    let s = tag.trim().trim_start_matches(['v', 'V']);
    let parts: Vec<u16> = s
        .split('.')
        .take(3)
        .map(|p| p.parse::<u16>().unwrap_or(0))
        .collect();
    let major = *parts.get(0).unwrap_or(&0);
    let minor = *parts.get(1).unwrap_or(&0);
    let patch = *parts.get(2).unwrap_or(&0);
    format!("{}.{}.{}.0", major, minor, patch)
}


