pub fn format_bytes(bytes: Option<u64>) -> String {
    match bytes {
        None => "-".to_string(),
        Some(0) => "0 B".to_string(),
        Some(bytes) => {
            let units = ["B", "KB", "MB", "GB", "TB", "PB"];
            let exponent = ((bytes as f64).log(1024.0).floor() as usize).min(units.len() - 1);
            let size = bytes as f64 / 1024f64.powi(exponent as i32);
            if size >= 10.0 {
                format!("{:.0} {}", size, units[exponent])
            } else {
                format!("{:.1} {}", size, units[exponent])
            }
        }
    }
}

pub fn format_date(mtime: Option<std::time::SystemTime>) -> String {
    match mtime {
        None => "-".to_string(),
        Some(t) => {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y/%m/%d %H:%M:%S").to_string()
        }
    }
}

pub fn escape_html(s: &str) -> String {
    html_escape::encode_text(s).to_string()
}

pub fn escape_attr(s: &str) -> String {
    html_escape::encode_double_quoted_attribute(s).to_string()
}

pub fn format_url_host(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{}]", host)
    } else {
        host.to_string()
    }
}
