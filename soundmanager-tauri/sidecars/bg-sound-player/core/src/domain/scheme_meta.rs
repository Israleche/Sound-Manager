//! Sound scheme metadata: name/author/about + 100x100 PNG thumbnail.
//! Port of SchemeMeta.cs. Stored in %APPDATA%\SoundManager\Media\Scheme.{png,ini}

use crate::domain::sound_event::{event_by_type, EventType, ALL_EVENTS};
use crate::domain::translation;
use crate::errors::CoreResult;
use image::imageops::FilterType;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::PathBuf;

pub const SCHEME_IMAGE_FILE: &str = "Scheme.png";
pub const SCHEME_INFO_FILE: &str = "Scheme.ini";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemeMeta {
    pub name: String,
    pub author: String,
    pub about: String,
    /// Base64-encoded 100x100 PNG thumbnail. Empty when no image.
    pub thumbnail_base64: String,
}

pub fn media_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata).join("SoundManager").join("Media")
}

pub fn scheme_image_path() -> PathBuf {
    media_dir().join(SCHEME_IMAGE_FILE)
}

pub fn scheme_info_path() -> PathBuf {
    media_dir().join(SCHEME_INFO_FILE)
}

/// Serialize the [SchemeInfo] block exactly like upstream SchemeMeta.SerializeSchemeInfo.
pub fn serialize_scheme_info(name: &str, author: &str, about: &str) -> Vec<u8> {
    format!("[SchemeInfo]\r\nname={name}\r\nauthor={author}\r\nabout={about}\r\n").into_bytes()
}

/// Parse Scheme.ini content. Tolerates legacy single-line "name=...;author=...;about=..."
/// and non-standard legacy infos.ini (same key names, FR aliases accepted).
pub fn parse_scheme_info(text: &str) -> SchemeMeta {
    let mut lines: Vec<String> = text.lines().map(|l| l.trim().to_string()).collect();
    if lines.len() == 1 && lines[0].contains(';') {
        lines = lines[0].split(';').map(|s| s.to_string()).collect();
    }
    let mut meta = SchemeMeta::default();
    for line_raw in lines {
        let line = line_raw.trim();
        let Some(eq) = line.find('=') else { continue };
        let field = line[..eq].trim().trim_matches('"').trim().to_lowercase();
        let value = line[eq + 1..].trim().trim_matches('"').trim().to_string();
        if line.len() <= eq + 1 {
            continue;
        }
        match field.as_str() {
            "name" | "nom" => meta.name = value,
            "author" | "auteur" => meta.author = value,
            "about" | "commentaire" => meta.about = value,
            _ => {}
        }
    }
    meta
}

/// Resize any image to 100x100 PNG and return PNG bytes.
pub fn make_thumbnail(png_or_img_bytes: &[u8]) -> CoreResult<Vec<u8>> {
    let img = image::load_from_memory(png_or_img_bytes)
        .map_err(|e| crate::errors::CoreError::Archive(format!("image decode: {e}")))?;
    let resized = img.resize_exact(100, 100, FilterType::Lanczos3);
    let mut out = Cursor::new(Vec::new());
    resized
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| crate::errors::CoreError::Archive(format!("png encode: {e}")))?;
    Ok(out.into_inner())
}

impl SchemeMeta {
    /// Reset all metadata to defaults (upstream SchemeMeta.ResetAll).
    pub fn reset_all() -> Self {
        let m = Self {
            name: translation::get("default_scheme_name"),
            author: translation::get("default_scheme_author"),
            about: translation::get("default_scheme_about"),
            thumbnail_base64: String::new(),
        };
        let _ = m.save();
        m
    }

    /// Load from disk. Missing files produce defaults.
    pub fn load() -> Self {
        let mut meta = SchemeMeta::default();
        if let Ok(text) = std::fs::read_to_string(scheme_info_path()) {
            meta = parse_scheme_info(&text);
        }
        if let Ok(bytes) = std::fs::read(scheme_image_path()) {
            meta.thumbnail_base64 = {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(bytes)
            };
        }
        meta
    }

    /// Persist info + (optionally replaced) thumbnail.
    pub fn save(&self) -> CoreResult<()> {
        let dir = media_dir();
        std::fs::create_dir_all(&dir)?;
        std::fs::write(scheme_info_path(), serialize_scheme_info(&self.name, &self.author, &self.about))?;
        if !self.thumbnail_base64.is_empty() {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD.decode(&self.thumbnail_base64)
                .map_err(|e| crate::errors::CoreError::Archive(format!("bad thumbnail base64: {e}")))?;
            let png = if bytes.len() > 4 && bytes[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A][..8.min(bytes.len())] {
                bytes
            } else {
                make_thumbnail(&bytes)?
            };
            std::fs::write(scheme_image_path(), png)?;
        }
        Ok(())
    }

    /// Pick a suitable sound event to play on scheme load: LoadScheme > Startup > Logon.
    /// Upstream also skips the stock Vista/7 Startup.wav (md5-identical) in favor of Logon.
    pub fn event_to_play_on_load() -> &'static crate::domain::sound_event::SoundEvent {
        use crate::domain::sound_event::{event_by_name, EventType};
        let load_scheme = event_by_type(EventType::LoadScheme);
        let startup = event_by_type(EventType::Startup);
        let logon = event_by_type(EventType::Logon);

        let media = media_dir();
        let exists = |e: &crate::domain::sound_event::SoundEvent| media.join(e.file_name()).is_file();

        if exists(load_scheme) {
            return load_scheme;
        }
        if !exists(startup) {
            return logon;
        }
        // Detect the stock Vista/7 startup sound by MD5 to avoid playing it too often.
        if let Ok(bytes) = std::fs::read(media.join(startup.file_name())) {
            use md5::{Digest, Md5};
            let mut hasher = Md5::new();
            hasher.update(&bytes);
            let hash = format!("{:x}", hasher.finalize());
            if hash == "155f2a0f886570157416ea85f4b4c613" {
                return logon;
            }
        }
        let _ = event_by_name("unused");
        startup
    }
}

/// Iterate all events (helper for services).
pub fn all_events() -> &'static [crate::domain::sound_event::SoundEvent] {
    ALL_EVENTS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard() {
        let m = parse_scheme_info("[SchemeInfo]\r\nname=My Scheme\r\nauthor=Me\r\nabout=Hello\r\n");
        assert_eq!(m.name, "My Scheme");
        assert_eq!(m.author, "Me");
        assert_eq!(m.about, "Hello");
    }

    #[test]
    fn parse_legacy_single_line() {
        let m = parse_scheme_info("name=\"Legacy\";author=\"Anon\";about=\"\"");
        assert_eq!(m.name, "Legacy");
        assert_eq!(m.author, "Anon");
    }

    #[test]
    fn serialize_roundtrip() {
        let bytes = serialize_scheme_info("A", "B", "C");
        let m = parse_scheme_info(&String::from_utf8(bytes).unwrap());
        assert_eq!((m.name.as_str(), m.author.as_str(), m.about.as_str()), ("A", "B", "C"));
    }
}
