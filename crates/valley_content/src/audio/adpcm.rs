//! Microsoft ADPCM decoder (WAVE_FORMAT_ADPCM, as used by XACT wave banks
//! and ADPCM `.wav` files), written from the publicly documented algorithm.
//!
//! A block starts with a per-channel header (predictor index, initial step
//! `delta`, then the two most recent samples, stored newest first) followed
//! by 4-bit codes interleaved across channels, high nibble first.

/// The seven standard predictor coefficient pairs (scaled by 256).
pub const STANDARD_COEFS: [(i32, i32); 7] = [(256, 0), (512, -256), (0, 0), (192, 64), (240, 0), (460, -208), (392, -232)];

/// Step-size adaptation factors (scaled by 256), indexed by the 4-bit code.
const ADAPTATION: [i32; 16] = [230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230];

/// Header bytes per channel at the start of each block.
pub const HEADER_BYTES_PER_CHANNEL: usize = 7;

/// Sample frames held by a full block.
pub fn samples_per_block(block_align: usize, channels: usize) -> usize {
    let ch = channels.max(1);
    if block_align < HEADER_BYTES_PER_CHANNEL * ch {
        return 0;
    }
    (block_align - HEADER_BYTES_PER_CHANNEL * ch) * 2 / ch + 2
}

#[derive(Clone, Copy, Default)]
struct ChannelState {
    c1: i32,
    c2: i32,
    delta: i32,
    s1: i32,
    s2: i32,
}

impl ChannelState {
    #[inline]
    fn expand(&mut self, code: u8) -> i16 {
        let signed = if code & 8 != 0 { code as i32 - 16 } else { code as i32 };
        let predicted = (self.s1 * self.c1 + self.s2 * self.c2) >> 8;
        let sample = (predicted + signed * self.delta).clamp(-32768, 32767);
        self.s2 = self.s1;
        self.s1 = sample;
        self.delta = ((ADAPTATION[code as usize] * self.delta) >> 8).max(16);
        sample as i16
    }
}

/// Decode one block, appending interleaved samples to `out`. A short final
/// block is decoded as far as it goes.
pub fn decode_block(block: &[u8], channels: usize, coefs: &[(i32, i32)], out: &mut Vec<i16>) -> Result<(), String> {
    let ch = channels;
    if ch == 0 || ch > 8 {
        return Err(format!("unsupported ADPCM channel count {ch}"));
    }
    let header = HEADER_BYTES_PER_CHANNEL * ch;
    if block.len() < header {
        return Err(format!("ADPCM block of {} bytes is shorter than its {header}-byte header", block.len()));
    }
    let mut st = [ChannelState::default(); 8];
    let rd16 = |i: usize| i16::from_le_bytes([block[i], block[i + 1]]) as i32;
    for (c, s) in st.iter_mut().enumerate().take(ch) {
        let pred = block[c] as usize;
        let &(c1, c2) = coefs.get(pred).ok_or_else(|| format!("bad ADPCM predictor index {pred}"))?;
        s.c1 = c1;
        s.c2 = c2;
        s.delta = rd16(ch + 2 * c);
        s.s1 = rd16(3 * ch + 2 * c);
        s.s2 = rd16(5 * ch + 2 * c);
    }
    // The header samples come out first: the older one, then the newer.
    for s in st.iter().take(ch) {
        out.push(s.s2 as i16);
    }
    for s in st.iter().take(ch) {
        out.push(s.s1 as i16);
    }
    let mut c = 0;
    for &byte in &block[header..] {
        for code in [byte >> 4, byte & 0x0F] {
            out.push(st[c].expand(code));
            c += 1;
            if c == ch {
                c = 0;
            }
        }
    }
    // Drop a trailing partial frame (only possible with odd channel counts).
    let rem = out.len() % ch;
    out.truncate(out.len() - rem);
    Ok(())
}

/// Decode a whole stream of `block_align`-sized blocks.
pub fn decode(data: &[u8], channels: usize, block_align: usize, coefs: &[(i32, i32)]) -> Result<Vec<i16>, String> {
    if block_align < HEADER_BYTES_PER_CHANNEL * channels.max(1) {
        return Err(format!("ADPCM block align {block_align} too small for {channels} channel(s)"));
    }
    let mut out = Vec::with_capacity(data.len() / block_align.max(1) * samples_per_block(block_align, channels) * channels + 16);
    for block in data.chunks(block_align) {
        if block.len() < HEADER_BYTES_PER_CHANNEL * channels {
            break; // trailing padding
        }
        decode_block(block, channels, coefs, &mut out)?;
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Minimal greedy MS-ADPCM encoder (fixed predictor), used only to build
    /// synthetic test data. Returns whole blocks.
    pub(crate) fn encode_mono(samples: &[i16], block_align: usize, predictor: u8) -> Vec<u8> {
        let per_block = samples_per_block(block_align, 1);
        let (c1, c2) = STANDARD_COEFS[predictor as usize];
        let mut out = Vec::new();
        for chunk in samples.chunks(per_block) {
            let mut block = vec![0u8; block_align];
            let s2 = chunk[0] as i32;
            let s1 = *chunk.get(1).unwrap_or(&chunk[0]) as i32;
            let mut st = ChannelState { c1, c2, delta: 16, s1, s2 };
            block[0] = predictor;
            block[1..3].copy_from_slice(&16i16.to_le_bytes());
            block[3..5].copy_from_slice(&(s1 as i16).to_le_bytes());
            block[5..7].copy_from_slice(&(s2 as i16).to_le_bytes());
            for (k, &target) in chunk.iter().skip(2).enumerate() {
                let predicted = (st.s1 * st.c1 + st.s2 * st.c2) >> 8;
                let q = ((target as i32 - predicted) as f32 / st.delta as f32).round().clamp(-8.0, 7.0) as i32;
                let code = (q & 0x0F) as u8;
                st.expand(code);
                let byte = 7 + k / 2;
                if k % 2 == 0 {
                    block[byte] |= code << 4;
                } else {
                    block[byte] |= code;
                }
            }
            out.extend_from_slice(&block);
        }
        out
    }

    #[test]
    fn decodes_hand_built_mono_block() {
        // predictor 1 (512, -256), delta 16, newest sample 100, oldest 50.
        let mut block = vec![1u8];
        block.extend_from_slice(&16i16.to_le_bytes());
        block.extend_from_slice(&100i16.to_le_bytes());
        block.extend_from_slice(&50i16.to_le_bytes());
        // Codes: 0x1, 0x7, 0xF (-1), 0x8 (-8)
        block.extend_from_slice(&[0x17, 0xF8]);
        let mut out = Vec::new();
        decode_block(&block, 1, &STANDARD_COEFS, &mut out).unwrap();
        // pred = (2*s1 - s2):
        //  code 1:  pred 150, +1*16 = 166; delta = 230*16>>8 = 14 -> 16
        //  code 7:  pred 232, +7*16 = 344; delta = 614*16>>8 = 38
        //  code -1: pred 522, -1*38 = 484; delta = 230*38>>8 = 34
        //  code -8: pred 624, -8*34 = 352; delta = 768*34>>8 = 102
        assert_eq!(out, vec![50, 100, 166, 344, 484, 352]);
        assert_eq!(samples_per_block(block.len(), 1), 6);
    }

    #[test]
    fn decodes_stereo_interleaving_and_clamps() {
        // Left: predictor 0 (256,0), delta 20000; right: predictor 2 (0,0), delta 16.
        let mut block = vec![0u8, 2];
        for v in [20000i16, 16, 30000, -5, 1, 2] {
            block.extend_from_slice(&v.to_le_bytes());
        }
        // High nibble -> left, low nibble -> right.
        block.extend_from_slice(&[0x73]);
        let mut out = Vec::new();
        decode_block(&block, 2, &STANDARD_COEFS, &mut out).unwrap();
        // Frames: (s2L, s2R), (s1L, s1R), then decoded: left clamps at 32767,
        // right = 0 + 3*16 = 48.
        assert_eq!(out, vec![1, 2, 30000, -5, 32767, 48]);
    }

    #[test]
    fn roundtrips_a_sine_through_the_test_encoder() {
        let pcm: Vec<i16> = (0..2000).map(|i| ((i as f32 * 0.05).sin() * 12000.0) as i16).collect();
        let block_align = 64;
        let enc = encode_mono(&pcm, block_align, 1);
        let dec = decode(&enc, 1, block_align, &STANDARD_COEFS).unwrap();
        assert!(dec.len() >= pcm.len());
        let err: f64 = pcm.iter().zip(&dec).map(|(a, b)| (*a as f64 - *b as f64).abs()).sum::<f64>() / pcm.len() as f64;
        assert!(err < 400.0, "mean abs error {err}");
    }

    #[test]
    fn rejects_bad_predictor() {
        let block = [9u8, 0, 0, 0, 0, 0, 0, 0];
        assert!(decode_block(&block, 1, &STANDARD_COEFS, &mut Vec::new()).is_err());
    }
}
