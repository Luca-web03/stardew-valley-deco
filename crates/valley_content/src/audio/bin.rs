//! Bounds-checked little-endian reader over a byte slice.

pub(crate) struct Bin<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> Bin<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn at(data: &'a [u8], pos: usize) -> Result<Self, String> {
        if pos > data.len() {
            return Err(format!("offset {pos:#x} is past the end ({:#x})", data.len()));
        }
        Ok(Self { data, pos })
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.data.len()).ok_or_else(|| format!("truncated data reading {n} bytes at {:#x}", self.pos))?;
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    pub fn skip(&mut self, n: usize) -> Result<(), String> {
        self.bytes(n).map(|_| ())
    }

    pub fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, String> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn i16(&mut self) -> Result<i16, String> {
        Ok(self.u16()? as i16)
    }

    pub fn u32(&mut self) -> Result<u32, String> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Fixed-size, NUL-padded string field.
    pub fn fixed_str(&mut self, n: usize) -> Result<String, String> {
        Ok(cstr(self.bytes(n)?))
    }
}

/// Text up to the first NUL byte.
pub(crate) fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}
