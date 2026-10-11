use std::collections::HashMap;

use crate::math::Color;

/// Handle to a texture in a [`TextureStore`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TexId(pub u32);

/// CPU-side RGBA8 image (straight alpha).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, rgba: vec![0; (width * height * 4) as usize] }
    }

    pub fn filled(width: u32, height: u32, c: Color) -> Self {
        let mut img = Self::new(width, height);
        img.fill_rect(0, 0, width as i32, height as i32, c);
        img
    }

    pub fn get(&self, x: u32, y: u32) -> Color {
        if x >= self.width || y >= self.height {
            return Color::TRANSPARENT;
        }
        let i = ((y * self.width + x) * 4) as usize;
        Color::rgba(self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3])
    }

    pub fn set(&mut self, x: i32, y: i32, c: Color) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        self.rgba[i] = c.r;
        self.rgba[i + 1] = c.g;
        self.rgba[i + 2] = c.b;
        self.rgba[i + 3] = c.a;
    }

    /// Alpha-blend `c` over the pixel at (x, y).
    pub fn blend(&mut self, x: i32, y: i32, c: Color) {
        if c.a == 0 || x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return;
        }
        if c.a == 255 {
            self.set(x, y, c);
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        let sa = c.a as u32;
        let da = self.rgba[i + 3] as u32;
        let out_a = sa + da * (255 - sa) / 255;
        if out_a == 0 {
            return;
        }
        let ch = |s: u8, d: u8| -> u8 {
            ((s as u32 * sa + d as u32 * da * (255 - sa) / 255) / out_a).min(255) as u8
        };
        self.rgba[i] = ch(c.r, self.rgba[i]);
        self.rgba[i + 1] = ch(c.g, self.rgba[i + 1]);
        self.rgba[i + 2] = ch(c.b, self.rgba[i + 2]);
        self.rgba[i + 3] = out_a as u8;
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    /// Copy a region of `src` into this image at (dx, dy), alpha blended.
    pub fn blit(&mut self, src: &Image, sx: u32, sy: u32, w: u32, h: u32, dx: i32, dy: i32) {
        for y in 0..h {
            for x in 0..w {
                let c = src.get(sx + x, sy + y);
                self.blend(dx + x as i32, dy + y as i32, c);
            }
        }
    }

    pub fn decode_png(bytes: &[u8]) -> Result<Image, String> {
        let mut dec = png::Decoder::new(bytes);
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = dec.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
        let (w, h) = (info.width, info.height);
        let px = (w * h) as usize;
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf[..px * 4].to_vec(),
            png::ColorType::Rgb => buf[..px * 3]
                .chunks_exact(3)
                .flat_map(|c| [c[0], c[1], c[2], 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => buf[..px * 2]
                .chunks_exact(2)
                .flat_map(|c| [c[0], c[0], c[0], c[1]])
                .collect(),
            png::ColorType::Grayscale => buf[..px].iter().flat_map(|&g| [g, g, g, 255]).collect(),
            png::ColorType::Indexed => return Err("indexed PNG not expanded".into()),
        };
        Ok(Image { width: w, height: h, rgba })
    }

    pub fn encode_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, self.width, self.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().expect("png header");
            w.write_image_data(&self.rgba).expect("png data");
        }
        out
    }
}

/// Owns every texture a module (or several modules) uses. Backends upload
/// lazily and track [`TextureStore::generation`] to notice replaced images.
#[derive(Default)]
pub struct TextureStore {
    images: Vec<Image>,
    generations: Vec<u32>,
    by_name: HashMap<String, TexId>,
}

impl TextureStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, img: Image) -> TexId {
        let name = name.into();
        if let Some(&id) = self.by_name.get(&name) {
            self.images[id.0 as usize] = img;
            self.generations[id.0 as usize] += 1;
            return id;
        }
        let id = TexId(self.images.len() as u32);
        self.images.push(img);
        self.generations.push(0);
        self.by_name.insert(name, id);
        id
    }

    pub fn id(&self, name: &str) -> Option<TexId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: TexId) -> &Image {
        &self.images[id.0 as usize]
    }

    pub fn generation(&self, id: TexId) -> u32 {
        self.generations[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.images.len()
    }

    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    pub fn size(&self, id: TexId) -> (u32, u32) {
        let i = self.get(id);
        (i.width, i.height)
    }
}
