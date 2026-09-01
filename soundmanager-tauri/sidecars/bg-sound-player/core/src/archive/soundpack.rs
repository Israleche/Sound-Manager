//! Proprietary .soundpack import (decrypt-only).
//! Port of SoundArchiveProprietary.cs: AES layer over a zip + 3DES-encrypted
//! XML metadata inside. Fair-use interop: unpack-only, no generation.

use crate::errors::{CoreError, CoreResult};
use cipher::{BlockDecryptMut, KeyIvInit};
use std::io::Read;
use std::path::Path;

pub const FILE_EXTENSION: &str = "soundpack";
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type DesEde3CbcDec = cbc::Decryptor<des::TdesEde3>;

const ZIP_AES_KEY: [u8; 16] = [
    0x43, 0x6f, 0x70, 0x79, 0x72, 0x69, 0x67, 0x68, 0x74, 0x3f, 0x53, 0x74, 0x61, 0x72, 0x64, 0x6f,
];
const ZIP_AES_IV: [u8; 16] = [
    0x7d, 0x2a, 0x7e, 0x61, 0x70, 0x3f, 0x3f, 0x3f, 0x53, 0x74, 0x61, 0x72, 0x64, 0x6f, 0x63, 0x6b,
];
const XML_3DES_KEY: [u8; 24] = [
    0x3f, 0x43, 0x6f, 0x70, 0x79, 0x72, 0x69, 0x67, 0x68, 0x74, 0x53, 0x74, 0x61, 0x72, 0x64, 0x6f,
    0x63, 0x6b, 0x32, 0x30, 0x30, 0x38, 0x3f, 0x3f,
];
const XML_3DES_IV: [u8; 8] = [0x7d, 0x6c, 0x60, 0x3f, 0x2a, 0x7e, 0x61, 0x70];

pub fn is_proprietary(file: &Path) -> bool {
    file.extension()
        .map(|e| e.eq_ignore_ascii_case(FILE_EXTENSION))
        .unwrap_or(false)
}

/// Decrypt the AES layer: returns the plain zip bytes.
pub fn decrypt_zip_layer(input: &Path) -> CoreResult<Vec<u8>> {
    let encrypted = std::fs::read(input)
        .map_err(|e| CoreError::Archive(format!("read {}: {e}", input.display())))?;
    // Skip if already a plain zip
    if encrypted.len() >= 4 && &encrypted[..2] == b"PK" {
        return Ok(encrypted);
    }
    let dec = Aes128CbcDec::new((&ZIP_AES_KEY).into(), (&ZIP_AES_IV).into());
    Ok(dec.decrypt_padded_vec_mut::<cipher::block_padding::Pkcs7>(&encrypted)
        .map_err(|_| CoreError::Archive("invalid soundpack AES layer".into()))?)
}

/// Decrypt the inner metadata XML (base64 + 3DES).
pub fn decrypt_metadata_xml(base64_data: &str) -> CoreResult<String> {
    use base64::Engine;
    let cipher_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_data.trim())
        .map_err(|e| CoreError::Archive(format!("metadata base64: {e}")))?;
    let mut dec = DesEde3CbcDec::new((&XML_3DES_KEY).into(), (&XML_3DES_IV).into());
    let plain = dec.decrypt_padded_vec_mut::<cipher::block_padding::Pkcs7>(&cipher_bytes)
        .map_err(|_| CoreError::Archive("invalid soundpack metadata 3DES".into()))?;
    Ok(String::from_utf8_lossy(&plain).to_string())
}

/// Convert a .soundpack file into a .ths archive at `outfile`.
/// Returns metadata extracted from the pack (name/author/about) when present.
pub fn convert(infile: &Path, outfile: &Path) -> CoreResult<Option<String>> {
    let zip_bytes = decrypt_zip_layer(infile)?;
    let tmp = std::env::temp_dir().join("soundmanager-soundpack.zip");
    std::fs::write(&tmp, &zip_bytes)?;

    let file = std::fs::File::open(&tmp)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| CoreError::Zip(e.to_string()))?;

    let mut meta_xml: Option<String> = None;
    // Locate soundpackage.data (base64 of 3DES XML), decrypt, and remap to Scheme.ini format.
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| CoreError::Zip(e.to_string()))?;
        if entry.name().ends_with("soundpackage.data") {
            let mut s = String::new();
            entry.read_to_string(&mut s).ok();
            meta_xml = decrypt_metadata_xml(&s).ok();
        }
    }
    drop(zip);

    // Rezip as .ths with the decrypted contents.
    let file = std::fs::File::open(&tmp)?;
    let mut src = zip::ZipArchive::new(file).map_err(|e| CoreError::Zip(e.to_string()))?;
    let out_file = std::fs::File::create(outfile)?;
    let mut out = zip::ZipWriter::new(out_file);
    let opts: zip::write::SimpleFileOptions =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for i in 0..src.len() {
        let mut entry = src.by_index(i).map_err(|e| CoreError::Zip(e.to_string()))?;
        let name = entry.name().to_string();
        if name.ends_with("soundpackage.data") {
            continue;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        out.start_file(name, opts).map_err(|e| CoreError::Zip(e.to_string()))?;
        std::io::Write::write_all(&mut out, &buf)?;
    }
    // Write Scheme.ini from decrypted metadata (extract name/author/about fields loosely).
    if let Some(xml) = meta_xml.as_ref() {
        let pick = |tag: &str| -> String {
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");
            xml.find(&open)
                .and_then(|s| xml[s + open.len()..].find(&close).map(|e| xml[s + open.len()..s + open.len() + e].to_string()))
                .unwrap_or_default()
        };
        let ini = format!(
            "[SchemeInfo]\r\nname={}\r\nauthor={}\r\nabout={}\r\n",
            pick("SchemeName"),
            pick("Author"),
            pick("Description")
        );
        out.start_file("Scheme.ini", opts).map_err(|e| CoreError::Zip(e.to_string()))?;
        std::io::Write::write_all(&mut out, ini.as_bytes())?;
    }
    out.finish().map_err(|e| CoreError::Zip(e.to_string()))?;
    let _ = std::fs::remove_file(&tmp);
    Ok(meta_xml.clone())
}
