//! Audio conversion: decode any supported input into PCM WAV (16-bit).
//! Replaces the NAudio MediaFoundationReader + WaveFileWriter path.
//! Max duration guard: 30 seconds (upstream SoundScheme.Update).

use crate::errors::{CoreError, CoreResult};
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

pub const MAX_SOUND_DURATION: Duration = Duration::from_secs(30);

/// Check whether a file is a plain WAV that Windows can play natively.
pub fn is_wav(path: &Path) -> bool {
    std::fs::read(path)
        .ok()
        .and_then(|b| probe_wav(&b))
        .unwrap_or(false)
}

/// Very cheap RIFF/WAVE sniff (upstream used SoundPlayer.Play+Stop as the check).
fn probe_wav(bytes: &[u8]) -> Option<bool> {
    if bytes.len() < 12 {
        return None;
    }
    Some(&bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE")
}

/// Decode any supported audio file to 16-bit PCM WAV bytes using Symphonia.
/// Mono inputs are copied as mono; sample rate preserved (Windows plays any PCM wav).
pub fn to_wav(input: &Path) -> CoreResult<Vec<u8>> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(input)
        .map_err(|e| CoreError::Audio(format!("open {}: {e}", input.display())))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = input.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| CoreError::Audio(format!("probe: {e}")))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| CoreError::Audio("no audio track".into()))?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| CoreError::Audio(format!("decoder: {e}")))?;

    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2) as u16;

    let mut pcm: Vec<i16> = Vec::new();
    let mut total_samples: u64 = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(CoreError::Audio(format!("packet: {e}"))),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| CoreError::Audio(format!("decode: {e}")))?;
        let mut buf = SampleBuffer::<i16>::new(decoded.capacity() as u64, *decoded.spec());
        buf.copy_interleaved_ref(decoded);
        pcm.extend_from_slice(buf.samples());
        total_samples += buf.samples().len() as u64 / channels.max(1) as u64;
    }

    let duration = Duration::from_secs_f64(total_samples as f64 / sample_rate as f64);
    if duration > MAX_SOUND_DURATION {
        return Err(CoreError::SoundTooLong);
    }

    write_wav_16(&pcm, sample_rate, channels)
}

/// Serialize interleaved i16 PCM into a canonical 44-byte-header WAV.
/// Hand-rolled header (44 bytes, PCM16) — matches what Windows expects.
pub fn write_wav_16(samples: &[i16], sample_rate: u32, channels: u16) -> CoreResult<Vec<u8>> {
    let bits = 16u16;
    let byte_rate = sample_rate * channels as u32 * (bits / 8) as u32;
    let block_align = channels * (bits / 8);
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header() {
        let samples = vec![0i16; 44100];
        let wav = write_wav_16(&samples, 44100, 2).unwrap();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }
}
