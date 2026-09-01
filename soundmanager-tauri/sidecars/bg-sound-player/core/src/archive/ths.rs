//! .ths sound archive import/export. Port of SoundArchive.cs.
//! Format: zip containing Scheme.ini, Scheme.png, <Event>.wav (+ legacy "Windows XP ..." names).

use crate::domain::scheme_meta::{self, SchemeMeta};
use crate::domain::sound_event::ALL_EVENTS;
use crate::errors::{CoreError, CoreResult};
use std::io::Read;
use std::path::Path;

pub const FILE_EXTENSION: &str = "ths";

/// Extract one named entry from the zip reader to the output dir (overwrite).
fn try_extract<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    file_name: &str,
    output_dir: &Path,
    output_file_name: Option<&str>,
) -> CoreResult<bool> {
    let Ok(mut entry) = zip.by_name(file_name) else { return Ok(false) };
    let out_name = output_file_name.unwrap_or(file_name);
    let dest = output_dir.join(out_name);
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).map_err(|e| CoreError::Zip(format!("{file_name}: {e}")))?;
    std::fs::write(&dest, buf)?;
    Ok(true)
}

/// Import a .ths zip into the media dir, refresh SchemeMeta, return the new meta.
/// Caller is responsible for SoundScheme::setup() + apply() afterwards.
pub fn import(zipfile: &Path, media_dir: &Path) -> CoreResult<SchemeMeta> {
    let file = std::fs::File::open(zipfile)
        .map_err(|e| CoreError::Archive(format!("open {}: {e}", zipfile.display())))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| CoreError::Zip(e.to_string()))?;

    std::fs::create_dir_all(media_dir)?;

    for ev in ALL_EVENTS {
        let canonical = ev.file_name();
        let legacy = ev.legacy_name();
        if try_extract(&mut zip, &canonical, media_dir, None)?
            || (ev.legacy_file_name.is_some() && try_extract(&mut zip, &legacy, media_dir, Some(&canonical))?)
        {
            // present in archive
        } else {
            let _ = std::fs::remove_file(ev.file_path(media_dir));
        }
    }

    // Scheme meta: Scheme.png + Scheme.ini, with legacy fallbacks visuel.bmp / infos.ini
    let img = scheme_meta::SCHEME_IMAGE_FILE;
    let ini = scheme_meta::SCHEME_INFO_FILE;
    if !try_extract(&mut zip, img, media_dir, None)? {
        try_extract(&mut zip, "visuel.bmp", media_dir, Some(img))?;
    }
    if !try_extract(&mut zip, ini, media_dir, None)? {
        try_extract(&mut zip, "infos.ini", media_dir, Some(ini))?;
    }

    // Parse meta from the media dir we just populated (NOT the global one).
    let mut meta = SchemeMeta::default();
    if let Ok(text) = std::fs::read_to_string(media_dir.join(ini)) {
        meta = scheme_meta::parse_scheme_info(&text);
    }
    if let Ok(bytes) = std::fs::read(media_dir.join(img)) {
        use base64::Engine;
        meta.thumbnail_base64 =
            base64::engine::general_purpose::STANDARD.encode(bytes);
    }
    Ok(meta)
}

/// Export the current scheme (media dir sounds + meta) to a .ths zip file.
pub fn export(zipfile: &Path, media_dir: &Path) -> CoreResult<()> {
    let file = std::fs::File::create(zipfile)
        .map_err(|e| CoreError::Archive(format!("create {}: {e}", zipfile.display())))?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut added = |name: &str, bytes: &[u8]| -> CoreResult<()> {
        zip.start_file(name, options).map_err(|e| CoreError::Zip(format!("{name}: {e}")))?;
        std::io::Write::write_all(&mut zip, bytes)?;
        Ok(())
    };

    for ev in ALL_EVENTS {
        let path = ev.file_path(media_dir);
        if path.is_file() {
            let bytes = std::fs::read(&path)?;
            added(&ev.file_name(), &bytes)?;
        }
    }
    let info = scheme_meta::scheme_info_path();
    if info.is_file() {
        let bytes = std::fs::read(&info)?;
        added(scheme_meta::SCHEME_INFO_FILE, &bytes)?;
    }
    let img = scheme_meta::scheme_image_path();
    if img.is_file() {
        let bytes = std::fs::read(&img)?;
        added(scheme_meta::SCHEME_IMAGE_FILE, &bytes)?;
    }
    zip.finish().map_err(|e| CoreError::Zip(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_zip(dir: &Path, entries: &[(&str, &[u8])]) -> std::path::PathBuf {
        let path = dir.join("test.ths");
        let file = std::fs::File::create(&path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions =
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            zw.start_file(*name, opts).unwrap();
            std::io::Write::write_all(&mut zw, data).unwrap();
        }
        zw.finish().unwrap();
        path
    }

    #[test]
    fn import_export_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("smcore-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // 1-second silence wav (8000 Hz mono 16-bit) via hound
        let spec = hound::WavSpec { channels: 1, sample_rate: 8000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut wav_buf = Vec::new();
        {
            let mut w = hound::WavWriter::new(Cursor::new(&mut wav_buf), spec).unwrap();
            for _ in 0..8000 { w.write_sample(0i16).unwrap(); }
            w.finalize().unwrap();
        }

        let ini = "[SchemeInfo]\r\nname=Test Scheme\r\nauthor=T\r\nabout=X\r\n";
        let zip_path = make_test_zip(&tmp, &[
            ("Startup.wav", wav_buf.as_slice()),
            (scheme_meta::SCHEME_INFO_FILE, ini.as_bytes()),
        ]);

        let media = tmp.join("media");
        let meta = import(&zip_path, &media).unwrap();
        assert_eq!(meta.name, "Test Scheme");
        assert!(media.join("Startup.wav").is_file());
        assert!(!media.join("Shutdown.wav").is_file());

        // Export must produce a valid zip containing Startup.wav
        let out = tmp.join("out.ths");
        export(&out, &media).unwrap();
        let f = std::fs::File::open(&out).unwrap();
        let mut z = zip::ZipArchive::new(f).unwrap();
        assert!(z.by_name("Startup.wav").is_ok());

        std::fs::remove_dir_all(&tmp).ok();
    }
}

use std::io::Cursor;
