//! XACT wave bank (`.xwb`) reader, written from the publicly documented
//! layout (the `WAVEBANK*` structures of the XACT SDK headers).
//!
//! Layout (little-endian, "WBND"):
//!
//! ```text
//! header:  signature[4] "WBND", version u32, [header_version u32 if version >= 42],
//!          5 regions {offset u32, length u32} (4 for version <= 3):
//!          bank data, entry metadata, seek tables, entry names, wave data
//! bank data: flags u32, entry_count u32, name[64], metadata_element_size u32,
//!          name_element_size u32, alignment u32, compact_format u32, build_time u64
//! entry (up to 24 bytes): flags:4|duration:28 u32, format u32,
//!          play_offset u32, play_length u32, loop_start u32, loop_length u32
//! compact entry (4 bytes): offset:21 (in alignment units) | length_deviation:11
//! mini format: tag:2 | channels:3 | sample_rate:18 | block_align:8 | bits:1
//! ```
//!
//! Only entry tables are read up front; wave data is read on demand, so
//! multi-hundred-megabyte banks are cheap to open. PCM (8/16-bit) and
//! MS-ADPCM entries decode; XMA and xWMA entries are reported as unsupported.

use std::io::{Read, Seek, SeekFrom};

use engine::SoundBuffer;

use super::adpcm;
use super::bin::{cstr, Bin};
use crate::source::ReadSeek;

pub const SIGNATURE: &[u8; 4] = b"WBND";
pub const FLAG_STREAMING: u32 = 0x0000_0001;
pub const FLAG_ENTRY_NAMES: u32 = 0x0001_0000;
pub const FLAG_COMPACT: u32 = 0x0002_0000;
/// For ADPCM the stored block align is `block_align / channels - 22`.
pub const ADPCM_BLOCK_ALIGN_OFFSET: u32 = 22;
const MAX_ENTRIES: u32 = 1 << 20;
const MAX_WAVE_BYTES: u32 = 512 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveCodec {
    Pcm,
    Xma,
    Adpcm,
    Wma,
}

impl WaveCodec {
    pub fn name(self) -> &'static str {
        match self {
            WaveCodec::Pcm => "PCM",
            WaveCodec::Xma => "XMA",
            WaveCodec::Adpcm => "MS-ADPCM",
            WaveCodec::Wma => "xWMA",
        }
    }
}

/// Unpacked `WAVEBANKMINIWAVEFORMAT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MiniFormat {
    pub codec: WaveCodec,
    pub channels: u16,
    pub sample_rate: u32,
    /// Raw 8-bit block-align field as stored (see [`MiniFormat::block_align`]).
    pub raw_block_align: u8,
    /// PCM: true = 16-bit, false = 8-bit.
    pub sixteen_bit: bool,
}

impl MiniFormat {
    pub fn unpack(v: u32) -> Self {
        let codec = match v & 0x3 {
            0 => WaveCodec::Pcm,
            1 => WaveCodec::Xma,
            2 => WaveCodec::Adpcm,
            _ => WaveCodec::Wma,
        };
        Self {
            codec,
            channels: ((v >> 2) & 0x7) as u16,
            sample_rate: (v >> 5) & 0x3_FFFF,
            raw_block_align: ((v >> 23) & 0xFF) as u8,
            sixteen_bit: (v >> 31) & 1 == 1,
        }
    }

    pub fn pack(&self) -> u32 {
        let tag = match self.codec {
            WaveCodec::Pcm => 0,
            WaveCodec::Xma => 1,
            WaveCodec::Adpcm => 2,
            WaveCodec::Wma => 3,
        };
        tag | ((self.channels as u32 & 0x7) << 2) | ((self.sample_rate & 0x3_FFFF) << 5) | ((self.raw_block_align as u32) << 23) | ((self.sixteen_bit as u32) << 31)
    }

    /// Actual block alignment in bytes.
    pub fn block_align(&self) -> u32 {
        match self.codec {
            WaveCodec::Adpcm => (self.raw_block_align as u32 + ADPCM_BLOCK_ALIGN_OFFSET) * self.channels as u32,
            _ => self.raw_block_align as u32,
        }
    }

    pub fn describe(&self) -> String {
        let bits = if self.codec == WaveCodec::Pcm { if self.sixteen_bit { " 16-bit" } else { " 8-bit" } } else { "" };
        format!("{}{bits} {}ch {} Hz", self.codec.name(), self.channels, self.sample_rate)
    }
}

/// One wave in the bank.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveEntry {
    pub name: Option<String>,
    pub format: MiniFormat,
    pub flags: u8,
    /// Length in sample frames (0 if the bank does not say).
    pub duration: u32,
    /// Absolute file offset of the wave data.
    pub offset: u64,
    /// Wave data length in bytes.
    pub length: u32,
    /// Loop region in sample frames (`loop_length == 0`: none).
    pub loop_start: u32,
    pub loop_length: u32,
}

/// Parsed bank header and entry table.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveBankInfo {
    pub name: String,
    pub version: u32,
    pub header_version: u32,
    pub flags: u32,
    pub entries: Vec<WaveEntry>,
}

impl WaveBankInfo {
    pub fn is_streaming(&self) -> bool {
        self.flags & FLAG_STREAMING != 0
    }
}

fn read_at<R: Read + Seek + ?Sized>(r: &mut R, offset: u64, len: usize) -> Result<Vec<u8>, String> {
    r.seek(SeekFrom::Start(offset)).map_err(|e| format!("seek to {offset:#x}: {e}"))?;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(|e| format!("read {len} bytes at {offset:#x}: {e}"))?;
    Ok(buf)
}

/// Read the header and entry table of a wave bank.
pub fn parse_info<R: Read + Seek + ?Sized>(r: &mut R) -> Result<WaveBankInfo, String> {
    let sig = read_at(r, 0, 4).map_err(|_| "file too short to be a wave bank".to_string())?;
    if sig == b"DNBW" {
        return Err("big-endian (console) wave banks are not supported".into());
    }
    if sig != SIGNATURE {
        return Err("not an XACT wave bank (missing WBND signature)".into());
    }
    let head = read_at(r, 4, 8)?;
    let version = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    if version <= 1 {
        return Err(format!("wave bank version {version} is too old to be supported"));
    }
    let (header_version, regions_at) = if version >= 42 { (u32::from_le_bytes([head[4], head[5], head[6], head[7]]), 12) } else { (0, 8) };
    let region_count = if version <= 3 { 4 } else { 5 };
    let raw = read_at(r, regions_at, region_count * 8)?;
    let mut b = Bin::new(&raw);
    let mut regions = Vec::with_capacity(5);
    for _ in 0..region_count {
        regions.push((b.u32()? as u64, b.u32()? as u64));
    }
    // Index the regions as bank data, metadata, seek tables, names, wave data.
    if region_count == 4 {
        regions.insert(2, (0, 0));
    }
    let (bank_off, bank_len) = regions[0];
    let (meta_off, meta_len) = regions[1];
    let (names_off, names_len) = regions[3];
    let (data_off, data_len) = regions[4];

    if bank_len < 84 {
        return Err(format!("bank data region too small ({bank_len} bytes)"));
    }
    let bank = read_at(r, bank_off, bank_len.min(96) as usize)?;
    let mut b = Bin::new(&bank);
    let flags = b.u32()?;
    let count = b.u32()?;
    let name = b.fixed_str(64)?;
    let meta_size = b.u32()?;
    let name_size = b.u32()?;
    let alignment = b.u32()?.max(1);
    let compact_format = MiniFormat::unpack(b.u32().unwrap_or(0));
    if count > MAX_ENTRIES {
        return Err(format!("implausible entry count {count}"));
    }
    let compact = flags & FLAG_COMPACT != 0;
    let meta_size = if compact { 4 } else { meta_size };
    if !(4..=64).contains(&meta_size) {
        return Err(format!("unsupported entry metadata size {meta_size}"));
    }
    let table_len = count as u64 * meta_size as u64;
    if meta_len != 0 && meta_len < table_len {
        return Err(format!("entry table truncated ({meta_len} < {table_len} bytes)"));
    }
    let meta = read_at(r, meta_off, table_len as usize)?;

    let mut names: Vec<Option<String>> = vec![None; count as usize];
    if flags & FLAG_ENTRY_NAMES != 0 && names_len > 0 && name_size > 0 && name_size <= 256 {
        let want = (count as u64 * name_size as u64).min(names_len) as usize;
        if let Ok(raw) = read_at(r, names_off, want) {
            for (i, chunk) in raw.chunks_exact(name_size as usize).enumerate() {
                let s = cstr(chunk);
                if !s.is_empty() {
                    names[i] = Some(s);
                }
            }
        }
    }

    let mut entries = Vec::with_capacity(count as usize);
    if compact {
        // Offsets come from the packed field; lengths from the next entry.
        let mut raw_offsets = Vec::with_capacity(count as usize);
        for c in meta.chunks_exact(4) {
            let v = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            raw_offsets.push(((v & 0x1F_FFFF) as u64 * alignment as u64, (v >> 21) as u64));
        }
        for i in 0..raw_offsets.len() {
            let (off, deviation) = raw_offsets[i];
            let next = raw_offsets.get(i + 1).map(|e| e.0).unwrap_or(data_len);
            let length = next.saturating_sub(off).saturating_sub(deviation);
            entries.push(WaveEntry {
                name: names[i].take(),
                format: compact_format,
                flags: 0,
                duration: 0,
                offset: data_off + off,
                length: length.min(u32::MAX as u64) as u32,
                loop_start: 0,
                loop_length: 0,
            });
        }
    } else {
        let mut b = Bin::new(&meta);
        let mut tmp = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let rec = b.bytes(meta_size as usize)?;
            let field = |k: usize| -> Option<u32> { rec.get(4 * k..4 * k + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])) };
            let fd = field(0).unwrap_or(0);
            tmp.push((fd, field(1).map(MiniFormat::unpack).unwrap_or(compact_format), field(2).unwrap_or(0) as u64, field(3), field(4).unwrap_or(0), field(5).unwrap_or(0)));
        }
        for i in 0..tmp.len() {
            let (fd, format, off, len, loop_start, loop_length) = tmp[i];
            let length = match len {
                Some(l) if l > 0 => l as u64,
                _ => tmp.get(i + 1).map(|e| e.2).unwrap_or(data_len).saturating_sub(off),
            };
            entries.push(WaveEntry {
                name: names[i].take(),
                format,
                flags: (fd & 0xF) as u8,
                duration: fd >> 4,
                offset: data_off + off,
                length: length.min(u32::MAX as u64) as u32,
                loop_start,
                loop_length,
            });
        }
    }
    Ok(WaveBankInfo { name, version, header_version, flags, entries })
}

/// Decode raw wave data of `entry` to PCM.
pub fn decode_entry(entry: &WaveEntry, data: &[u8]) -> Result<SoundBuffer, String> {
    let f = entry.format;
    let ch = f.channels as usize;
    if ch == 0 {
        return Err("entry has zero channels".into());
    }
    if f.sample_rate == 0 {
        return Err("entry has a zero sample rate".into());
    }
    let mut samples: Vec<i16> = match f.codec {
        WaveCodec::Pcm if f.sixteen_bit => data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect(),
        WaveCodec::Pcm => data.iter().map(|&b| ((b as i16) - 128) << 8).collect(),
        WaveCodec::Adpcm => adpcm::decode(data, ch, f.block_align() as usize, &adpcm::STANDARD_COEFS)?,
        WaveCodec::Wma | WaveCodec::Xma => return Err(format!("{} audio cannot be decoded (unsupported codec)", f.codec.name())),
    };
    let frames = samples.len() / ch;
    if entry.duration > 0 && (entry.duration as usize) < frames {
        samples.truncate(entry.duration as usize * ch);
    }
    let mut buf = SoundBuffer::from_interleaved(f.sample_rate, f.channels, samples);
    if entry.loop_length > 0 {
        buf.loop_region = Some((entry.loop_start as usize, entry.loop_length as usize));
    }
    Ok(buf)
}

/// An open wave bank: parsed entry table plus a reader for wave data.
pub struct WaveBank {
    pub info: WaveBankInfo,
    reader: Box<dyn ReadSeek>,
}

impl std::fmt::Debug for WaveBank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaveBank").field("name", &self.info.name).field("entries", &self.info.entries.len()).finish()
    }
}

impl WaveBank {
    pub fn open(mut reader: Box<dyn ReadSeek>) -> Result<Self, String> {
        let info = parse_info(&mut *reader)?;
        Ok(Self { info, reader })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        Self::open(Box::new(std::io::Cursor::new(bytes)))
    }

    pub fn name(&self) -> &str {
        &self.info.name
    }

    pub fn len(&self) -> usize {
        self.info.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.info.entries.is_empty()
    }

    /// Index of the entry with this name (banks built with entry names only).
    pub fn find(&self, name: &str) -> Option<usize> {
        self.info.entries.iter().position(|e| e.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
    }

    pub fn read_raw(&mut self, index: usize) -> Result<Vec<u8>, String> {
        let e = self.info.entries.get(index).ok_or_else(|| format!("wave index {index} out of range (bank has {})", self.info.entries.len()))?;
        if e.length > MAX_WAVE_BYTES {
            return Err(format!("wave {index} is implausibly large ({} bytes)", e.length));
        }
        let (offset, length) = (e.offset, e.length as usize);
        read_at(&mut *self.reader, offset, length)
    }

    pub fn decode(&mut self, index: usize) -> Result<SoundBuffer, String> {
        let data = self.read_raw(index)?;
        decode_entry(&self.info.entries[index], &data)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::audio::adpcm;

    pub(crate) struct TestWave {
        pub name: &'static str,
        pub format: MiniFormat,
        pub data: Vec<u8>,
        pub duration: u32,
        pub loop_region: (u32, u32),
    }

    fn put32(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_le_bytes());
    }

    /// Build a version-46 (XACT3) wave bank in memory.
    pub(crate) fn build_xwb(bank_name: &str, waves: &[TestWave], with_names: bool) -> Vec<u8> {
        let alignment = 4usize;
        let mut data = Vec::new();
        let mut meta = Vec::new();
        for w in waves {
            while data.len() % alignment != 0 {
                data.push(0);
            }
            put32(&mut meta, (w.duration << 4) | 1);
            put32(&mut meta, w.format.pack());
            put32(&mut meta, data.len() as u32);
            put32(&mut meta, w.data.len() as u32);
            put32(&mut meta, w.loop_region.0);
            put32(&mut meta, w.loop_region.1);
            data.extend_from_slice(&w.data);
        }
        let mut names = Vec::new();
        if with_names {
            for w in waves {
                let mut n = w.name.as_bytes().to_vec();
                n.resize(64, 0);
                names.extend_from_slice(&n);
            }
        }
        let mut bank = Vec::new();
        put32(&mut bank, if with_names { FLAG_ENTRY_NAMES } else { 0 });
        put32(&mut bank, waves.len() as u32);
        let mut n = bank_name.as_bytes().to_vec();
        n.resize(64, 0);
        bank.extend_from_slice(&n);
        put32(&mut bank, 24);
        put32(&mut bank, 64);
        put32(&mut bank, alignment as u32);
        put32(&mut bank, 0);
        bank.extend_from_slice(&[0; 8]);
        assert_eq!(bank.len(), 96);

        let header_len = 12 + 5 * 8;
        let bank_off = header_len;
        let meta_off = bank_off + bank.len();
        let names_off = meta_off + meta.len();
        let data_off = (names_off + names.len()).div_ceil(alignment) * alignment;
        let mut out = Vec::new();
        out.extend_from_slice(SIGNATURE);
        put32(&mut out, 46);
        put32(&mut out, 44);
        for (o, l) in [(bank_off, bank.len()), (meta_off, meta.len()), (0, 0), (names_off, names.len()), (data_off, data.len())] {
            put32(&mut out, o as u32);
            put32(&mut out, l as u32);
        }
        out.extend_from_slice(&bank);
        out.extend_from_slice(&meta);
        out.extend_from_slice(&names);
        out.resize(data_off, 0);
        out.extend_from_slice(&data);
        out
    }

    pub(crate) fn pcm16(rate: u32, ch: u16, samples: &[i16]) -> TestWave {
        let data = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let format = MiniFormat { codec: WaveCodec::Pcm, channels: ch, sample_rate: rate, raw_block_align: (2 * ch) as u8, sixteen_bit: true };
        TestWave { name: "", format, data, duration: (samples.len() / ch as usize) as u32, loop_region: (0, 0) }
    }

    pub(crate) fn sample_bank() -> (Vec<u8>, Vec<i16>) {
        let ramp: Vec<i16> = (0..100).map(|i| (i * 300 - 15000) as i16).collect();
        let sine: Vec<i16> = (0..700).map(|i| ((i as f32 * 0.07).sin() * 9000.0) as i16).collect();
        let block_align = 36; // stored as 36 - 22 = 14 for mono
        let adpcm_data = adpcm::tests::encode_mono(&sine, block_align, 1);
        let waves = vec![
            TestWave { name: "ramp16", ..pcm16(22050, 1, &ramp) },
            TestWave {
                name: "stereo8",
                format: MiniFormat { codec: WaveCodec::Pcm, channels: 2, sample_rate: 11025, raw_block_align: 2, sixteen_bit: false },
                data: vec![128, 255, 0, 128, 192, 64],
                duration: 3,
                loop_region: (0, 0),
            },
            TestWave {
                name: "sine_adpcm",
                format: MiniFormat { codec: WaveCodec::Adpcm, channels: 1, sample_rate: 44100, raw_block_align: (block_align - 22) as u8, sixteen_bit: false },
                data: adpcm_data,
                duration: sine.len() as u32,
                loop_region: (100, 500),
            },
            TestWave {
                name: "music_wma",
                format: MiniFormat { codec: WaveCodec::Wma, channels: 2, sample_rate: 44100, raw_block_align: 3, sixteen_bit: false },
                data: vec![1, 2, 3, 4, 5, 6, 7, 8],
                duration: 1000,
                loop_region: (0, 0),
            },
        ];
        (build_xwb("Test Bank", &waves, true), sine)
    }

    #[test]
    fn mini_format_bitfields_roundtrip() {
        let f = MiniFormat { codec: WaveCodec::Adpcm, channels: 2, sample_rate: 44100, raw_block_align: 48, sixteen_bit: false };
        let v = f.pack();
        assert_eq!(v & 3, 2);
        assert_eq!((v >> 2) & 7, 2);
        assert_eq!((v >> 5) & 0x3FFFF, 44100);
        assert_eq!((v >> 23) & 0xFF, 48);
        assert_eq!(MiniFormat::unpack(v), f);
        assert_eq!(f.block_align(), (48 + 22) * 2);
        let p = MiniFormat { codec: WaveCodec::Pcm, channels: 1, sample_rate: 48000, raw_block_align: 2, sixteen_bit: true };
        assert_eq!(MiniFormat::unpack(p.pack()), p);
        assert_eq!(p.pack() >> 31, 1);
    }

    #[test]
    fn parses_header_and_entries() {
        let (bytes, _) = sample_bank();
        let info = parse_info(&mut std::io::Cursor::new(&bytes)).unwrap();
        assert_eq!(info.name, "Test Bank");
        assert_eq!(info.version, 46);
        assert_eq!(info.header_version, 44);
        assert!(!info.is_streaming());
        assert_eq!(info.entries.len(), 4);
        let e = &info.entries[0];
        assert_eq!(e.name.as_deref(), Some("ramp16"));
        assert_eq!(e.format.codec, WaveCodec::Pcm);
        assert_eq!(e.format.sample_rate, 22050);
        assert_eq!(e.duration, 100);
        assert_eq!(e.length, 200);
        assert_eq!(e.flags, 1);
        assert_eq!(info.entries[1].format.channels, 2);
        assert!(!info.entries[1].format.sixteen_bit);
        assert_eq!(info.entries[2].format.codec, WaveCodec::Adpcm);
        assert_eq!(info.entries[2].format.block_align(), 36);
        assert_eq!((info.entries[2].loop_start, info.entries[2].loop_length), (100, 500));
        assert_eq!(info.entries[3].format.codec, WaveCodec::Wma);
        // Data offsets respect the bank's alignment.
        assert!(info.entries.iter().all(|e| e.offset % 4 == 0));
    }

    #[test]
    fn extracts_pcm_and_adpcm_and_reports_wma() {
        let (bytes, sine) = sample_bank();
        let mut bank = WaveBank::from_bytes(bytes).unwrap();
        assert_eq!(bank.find("SINE_ADPCM"), Some(2));
        assert_eq!(bank.find("nope"), None);

        let ramp = bank.decode(0).unwrap();
        assert_eq!((ramp.sample_rate, ramp.channels, ramp.frames()), (22050, 1, 100));
        assert_eq!(ramp.samples[0], -15000);
        assert_eq!(ramp.samples[99], 99 * 300 - 15000);

        let st = bank.decode(1).unwrap();
        assert_eq!((st.channels, st.frames()), (2, 3));
        assert_eq!(st.samples, vec![0, 127 << 8, -128 << 8, 0, 64 << 8, -64 << 8]);

        let ad = bank.decode(2).unwrap();
        assert_eq!(ad.frames(), sine.len(), "trimmed to the entry duration");
        assert_eq!(ad.loop_region, Some((100, 500)));
        let err: f64 = sine.iter().zip(&ad.samples).map(|(a, b)| (*a as f64 - *b as f64).abs()).sum::<f64>() / sine.len() as f64;
        assert!(err < 400.0, "ADPCM error {err}");

        let wma = bank.decode(3).unwrap_err();
        assert!(wma.contains("xWMA"), "{wma}");
        assert!(bank.decode(4).is_err());
    }

    #[test]
    fn compact_banks_derive_lengths_from_offsets() {
        // Two 16-bit mono waves of 8 and 6 bytes, alignment 4, compact table.
        let fmt = MiniFormat { codec: WaveCodec::Pcm, channels: 1, sample_rate: 8000, raw_block_align: 2, sixteen_bit: true };
        let mut bank = Vec::new();
        bank.extend_from_slice(&FLAG_COMPACT.to_le_bytes());
        bank.extend_from_slice(&2u32.to_le_bytes());
        bank.extend_from_slice(&[0u8; 64]);
        for v in [4u32, 0, 4, fmt.pack()] {
            bank.extend_from_slice(&v.to_le_bytes());
        }
        bank.extend_from_slice(&[0; 8]);
        let meta: Vec<u8> = [0u32, 2 | (2 << 21)].iter().flat_map(|v| v.to_le_bytes()).collect();
        let data: Vec<u8> = [1i16, 2, 3, 4, 5, 6, 7, 0].iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut out = b"WBND".to_vec();
        out.extend_from_slice(&46u32.to_le_bytes());
        out.extend_from_slice(&44u32.to_le_bytes());
        let (b_off, m_off) = (52u32, 52 + bank.len() as u32);
        let d_off = m_off + meta.len() as u32;
        for (o, l) in [(b_off, bank.len() as u32), (m_off, meta.len() as u32), (0, 0), (0, 0), (d_off, data.len() as u32)] {
            out.extend_from_slice(&o.to_le_bytes());
            out.extend_from_slice(&l.to_le_bytes());
        }
        out.extend_from_slice(&bank);
        out.extend_from_slice(&meta);
        out.extend_from_slice(&data);
        let mut wb = WaveBank::from_bytes(out).unwrap();
        assert_eq!(wb.info.entries[0].length, 8);
        assert_eq!(wb.info.entries[1].length, 6);
        assert_eq!(wb.decode(1).unwrap().samples, vec![5, 6, 7]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(WaveBank::from_bytes(b"RIFF....".to_vec()).is_err());
        assert!(WaveBank::from_bytes(b"DNBW\0\0\0\x2e".to_vec()).unwrap_err().contains("big-endian"));
        let (mut bytes, _) = sample_bank();
        bytes.truncate(100);
        assert!(WaveBank::from_bytes(bytes).is_err());
    }
}
