use std::path::Path;

pub fn extract_supported_text(path: &str) -> Option<String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())?;

    match ext.as_str() {
        "txt" | "md" => std::fs::read(path)
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        "pdf" => pdf_extract::extract_text(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}
