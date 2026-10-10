//! RIFF WAVE reader (PCM 8/16/24/32-bit, IEEE float, MS-ADPCM, extensible)
//! and a 16-bit PCM writer, plus Ogg Vorbis decoding for loose audio files.

use engine::SoundBuffer;

use super::adpcm;
use super::bin::Bin;

const TAG_PCM: u16 = 0x0001;
const TAG_ADPCM: u16 = 0x0002;
const TAG_FLOAT: u16 = 0x0003;
const TAG_EXTENSIBLE: u16 = 0xFFFE;

/// Decode a `.wav` file.
pub fn decode(bytes: &[u8]) -> Result<SoundBuffer, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF WAVE file".into());
    }
    let mut b = Bin::at(bytes, 12)?;
    let mut fmt: Option<&[u8]> = None;
    let mut data: Option<&[u8]> = None;
    let mut fact_frames: Option<u32> = None;
    let mut loop_region = None;
    while b.pos + 8 <= bytes.len() {
        let id = b.bytes(4)?;
        let size = b.u32()? as usize;
        // Tolerate a truncated final chunk (common for streamed writers).
        let body = &bytes[b.pos..(b.pos + size).min(bytes.len())];
        match id {
            b"fmt " => fmt = Some(body),
            b"data" => data = Some(body),
            b"fact" if body.len() >= 4 => fact_frames = Some(u32::from_le_bytes([body[0], body[1], body[2], body[3]])),
            b"smpl" if body.len() >= 36 + 24 => {
                let loops = u32::from_le_bytes([body[28], body[29], body[30], body[31]]);
                if loops > 0 {
                    let rd = |i: usize| u32::from_le_bytes([body[i], body[i + 1], body[i + 2], body[i + 3]]) as usize;
                    let (start, end) = (rd(36 + 8), rd(36 + 12));
                    if end > start {
                        loop_region = Some((start, end - start + 1));
                    }
                }
            }
            _ => {}
        }
        let next = b.pos.saturating_add(size + (size & 1));
        if next > bytes.len() {
            break;
        }
        b.pos = next;
    }
    let fmt = fmt.ok_or("WAVE file has no fmt chunk")?;
    let data = data.ok_or("WAVE file has no data chunk")?;
    let mut f = Bin::new(fmt);
    let mut tag = f.u16()?;
    let channels = f.u16()?;
    let rate = f.u32()?;
    f.skip(4)?; // byte rate
    let block_align = f.u16()? as usize;
    let bits = f.u16()?;
    let extra = if fmt.len() >= 18 { f.u16()? as usize } else { 0 };
    if tag == TAG_EXTENSIBLE && extra >= 22 {
        f.skip(2 + 4)?; // valid bits, channel mask
        tag = f.u16()?; // first two bytes of the sub-format GUID
    }
    if channels == 0 || rate == 0 {
        return Err("WAVE file has zero channels or sample rate".into());
    }
    let ch = channels as usize;
    let samples: Vec<i16> = match (tag, bits) {
        (TAG_PCM, 8) => data.iter().map(|&v| ((v as i16) - 128) << 8).collect(),
        (TAG_PCM, 16) => data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect(),
        (TAG_PCM, 24) => data.chunks_exact(3).map(|c| i16::from_le_bytes([c[1], c[2]])).collect(),
        (TAG_PCM, 32) => data.chunks_exact(4).map(|c| i16::from_le_bytes([c[2], c[3]])).collect(),
        (TAG_FLOAT, 32) => data.chunks_exact(4).map(|c| f32_to_i16(f32::from_le_bytes([c[0], c[1], c[2], c[3]]))).collect(),
        (TAG_FLOAT, 64) => data.chunks_exact(8).map(|c| f32_to_i16(f64::from_le_bytes(c.try_into().unwrap()) as f32)).collect(),
        (TAG_ADPCM, 4) => {
            let mut coefs = adpcm::STANDARD_COEFS.to_vec();
            if extra >= 4 {
                f.skip(2)?; // samples per block
                let n = f.u16()? as usize;
                if n > 0 && n <= 256 {
                    coefs.clear();
                    for _ in 0..n {
                        coefs.push((f.i16()? as i32, f.i16()? as i32));
                    }
                }
            }
            let mut s = adpcm::decode(data, ch, block_align, &coefs)?;
            if let Some(n) = fact_frames {
                s.truncate(n as usize * ch);
            }
            s
        }
        (t, b) => return Err(format!("unsupported WAVE encoding (format tag {t:#x}, {b} bits)")),
    };
    let mut buf = SoundBuffer::from_interleaved(rate, channels, samples);
    buf.loop_region = loop_region;
    Ok(buf)
}

fn f32_to_i16(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32767.0).round() as i16
}

/// Encode a buffer as a 16-bit PCM `.wav` file.
pub fn encode_pcm16(buf: &SoundBuffer) -> Vec<u8> {
    let ch = buf.channels.max(1);
    let data_len = (buf.samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&TAG_PCM.to_le_bytes());
    out.extend_from_slice(&ch.to_le_bytes());
    out.extend_from_slice(&buf.sample_rate.to_le_bytes());
    out.extend_from_slice(&(buf.sample_rate * ch as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(ch * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in &buf.samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Decode an Ogg Vorbis file.
pub fn decode_ogg(bytes: &[u8]) -> Result<SoundBuffer, String> {
    let mut r = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes)).map_err(|e| format!("ogg: {e}"))?;
    let channels = r.ident_hdr.audio_channels as u16;
    let rate = r.ident_hdr.audio_sample_rate;
    let mut samples = Vec::new();
    while let Some(packet) = r.read_dec_packet_itl().map_err(|e| format!("ogg: {e}"))? {
        samples.extend_from_slice(&packet);
    }
    Ok(SoundBuffer::from_interleaved(rate, channels, samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn riff(fmt: &[u8], extra_chunks: &[(&[u8; 4], Vec<u8>)], data: &[u8]) -> Vec<u8> {
        let mut body = b"WAVE".to_vec();
        let mut chunk = |id: &[u8], b: &[u8]| {
            body.extend_from_slice(id);
            body.extend_from_slice(&(b.len() as u32).to_le_bytes());
            body.extend_from_slice(b);
            if b.len() % 2 == 1 {
                body.push(0);
            }
        };
        chunk(b"fmt ", fmt);
        for (id, b) in extra_chunks {
            chunk(&id[..], b);
        }
        chunk(b"data", data);
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn fmt(tag: u16, ch: u16, rate: u32, align: u16, bits: u16, extra: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&tag.to_le_bytes());
        f.extend_from_slice(&ch.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * align as u32).to_le_bytes());
        f.extend_from_slice(&align.to_le_bytes());
        f.extend_from_slice(&bits.to_le_bytes());
        if !extra.is_empty() {
            f.extend_from_slice(&(extra.len() as u16).to_le_bytes());
            f.extend_from_slice(extra);
        }
        f
    }

    #[test]
    fn pcm16_roundtrip() {
        let b = SoundBuffer::from_interleaved(32000, 2, vec![1, -2, 300, -400, 32767, -32768]);
        let back = decode(&encode_pcm16(&b)).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn decodes_float_8bit_and_24bit() {
        let data: Vec<u8> = [0.5f32, -1.0].iter().flat_map(|v| v.to_le_bytes()).collect();
        let b = decode(&riff(&fmt(TAG_FLOAT, 1, 48000, 4, 32, &[]), &[], &data)).unwrap();
        assert_eq!(b.samples, vec![16384, -32767]);

        let b = decode(&riff(&fmt(TAG_PCM, 1, 8000, 1, 8, &[]), &[], &[128, 0, 255])).unwrap();
        assert_eq!(b.samples, vec![0, -32768, 32512]);

        let b = decode(&riff(&fmt(TAG_PCM, 1, 8000, 3, 24, &[]), &[], &[0xAA, 0x34, 0x12])).unwrap();
        assert_eq!(b.samples, vec![0x1234]);
    }

    #[test]
    fn decodes_ms_adpcm_with_fact_and_loop() {
        let pcm: Vec<i16> = (0..300).map(|i| ((i as f32 * 0.1).sin() * 8000.0) as i16).collect();
        let enc = adpcm::tests::encode_mono(&pcm, 64, 0);
        let mut extra = Vec::new();
        extra.extend_from_slice(&(adpcm::samples_per_block(64, 1) as u16).to_le_bytes());
        extra.extend_from_slice(&7u16.to_le_bytes());
        for (a, b) in adpcm::STANDARD_COEFS {
            extra.extend_from_slice(&(a as i16).to_le_bytes());
            extra.extend_from_slice(&(b as i16).to_le_bytes());
        }
        let mut smpl = vec![0u8; 36];
        smpl[28] = 1;
        for v in [0u32, 0, 10, 109, 0, 0] {
            smpl.extend_from_slice(&v.to_le_bytes());
        }
        let wav = riff(&fmt(TAG_ADPCM, 1, 22050, 64, 4, &extra), &[(b"fact", 300u32.to_le_bytes().to_vec()), (b"smpl", smpl)], &enc);
        let b = decode(&wav).unwrap();
        assert_eq!(b.frames(), 300);
        assert_eq!(b.loop_region, Some((10, 100)));
        let err: f64 = pcm.iter().zip(&b.samples).map(|(a, b)| (*a as f64 - *b as f64).abs()).sum::<f64>() / 300.0;
        assert!(err < 400.0, "{err}");
    }

    #[test]
    fn rejects_bad_files() {
        assert!(decode(b"RIFF\0\0\0\0WAVE").is_err());
        assert!(decode(b"OggS").is_err());
        assert!(decode(&riff(&fmt(0x55, 1, 8000, 1, 0, &[]), &[], &[1, 2])).unwrap_err().contains("unsupported"));
        assert!(decode_ogg(b"OggS not really").is_err());
    }
}
