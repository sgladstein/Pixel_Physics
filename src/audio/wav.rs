//! **A 16-bit PCM WAV writer**, for the offline render
//! (`examples/soundtrack.rs`). Thirty lines rather than the `hound` crate: the
//! format is a fixed 44-byte header and interleaved samples, and the project
//! keeps its dependency count deliberately small (`Cargo.toml`).

use std::io::Write;

/// Interleaved `f32` samples in -1 ..= 1 to a little-endian 16-bit WAV.
pub fn write(path: &std::path::Path, samples: &[f32], channels: u16, sample_rate: u32) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    f.write_all(&encode(samples, channels, sample_rate))?;
    f.flush()
}

/// The whole file as bytes -- split out so a test can hash it without touching
/// the disk.
pub fn encode(samples: &[f32], channels: u16, sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_describes_the_data() {
        let bytes = encode(&[0.0, 1.0, -1.0, 0.5], 2, 48_000);
        assert_eq!(bytes.len(), 44 + 8);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8, "data chunk length");
        assert_eq!(i16::from_le_bytes(bytes[46..48].try_into().unwrap()), 32767, "full scale");
        assert_eq!(i16::from_le_bytes(bytes[48..50].try_into().unwrap()), -32767, "clamped, symmetric");
    }
}
