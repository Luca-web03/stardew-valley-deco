//! Co-op networking by render streaming. The host runs every player's game
//! screen (each remote player's UI state and farmhand are swapped in for
//! their turn) and sends them the drawn frame, any textures they haven't
//! seen and their sound requests. Clients only send input and show frames,
//! so they need no game content.
//!
//! Wire format: frames of `u32 length (LE) + u8 kind + payload`.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use engine::{Color, DrawCmd, DrawList, FRect, IRect, Image, InputState, Key, MouseButton, SoundCmd, Sprite, TexId, Vec2};
use serde::{Deserialize, Serialize};

pub const KIND_HELLO: u8 = 1;
pub const KIND_INPUT: u8 = 2;
pub const KIND_WELCOME: u8 = 10;
pub const KIND_TEXTURE: u8 = 11;
pub const KIND_FRAME: u8 = 12;
pub const KIND_SOUND: u8 = 13;

/// One raw input event from a client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    KeyDown(Key),
    KeyUp(Key),
    MouseMove(f32, f32),
    /// 0 left, 1 right, 2 middle.
    MouseDown(u8),
    MouseUp(u8),
    Wheel(f32),
    Text(String),
    Screen(u32, u32),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NetSound {
    Play { cue: String, volume: f32, pitch: f32 },
    Music { track: String, volume: f32 },
    StopMusic,
}

pub fn mouse_button(b: u8) -> MouseButton {
    match b {
        1 => MouseButton::Right,
        2 => MouseButton::Middle,
        _ => MouseButton::Left,
    }
}

pub fn mouse_code(b: MouseButton) -> u8 {
    match b {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
    }
}

/// Apply an input event to an input state (and track screen size).
pub fn apply_input(i: &mut InputState, screen: &mut (u32, u32), e: &InputEvent) {
    match e {
        InputEvent::KeyDown(k) => i.key_down(*k),
        InputEvent::KeyUp(k) => i.key_up(*k),
        InputEvent::MouseMove(x, y) => i.mouse = Vec2::new(*x, *y),
        InputEvent::MouseDown(b) => i.mouse_down(mouse_button(*b)),
        InputEvent::MouseUp(b) => i.mouse_up(mouse_button(*b)),
        InputEvent::Wheel(w) => i.wheel = *w,
        InputEvent::Text(t) => i.text.push_str(t),
        InputEvent::Screen(w, h) => *screen = ((*w).max(320), (*h).max(240)),
    }
}

// ------------------------------------------------------------- framing

/// Buffered, non-blocking framed connection.
pub struct Conn {
    stream: TcpStream,
    inbuf: Vec<u8>,
    pub closed: bool,
}

impl Conn {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;
        Ok(Self { stream, inbuf: Vec::new(), closed: false })
    }

    pub fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        Self::new(TcpStream::connect(addr)?)
    }

    pub fn send(&mut self, kind: u8, payload: &[u8]) {
        let mut msg = Vec::with_capacity(payload.len() + 5);
        msg.extend_from_slice(&((payload.len() + 1) as u32).to_le_bytes());
        msg.push(kind);
        msg.extend_from_slice(payload);
        // The socket is non-blocking: write it all, retrying briefly.
        let mut sent = 0;
        let mut spins = 0;
        while sent < msg.len() {
            match self.stream.write(&msg[sent..]) {
                Ok(0) => {
                    self.closed = true;
                    return;
                }
                Ok(n) => sent += n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    spins += 1;
                    if spins > 20_000 {
                        self.closed = true;
                        return;
                    }
                    std::thread::yield_now();
                }
                Err(_) => {
                    self.closed = true;
                    return;
                }
            }
        }
    }

    pub fn send_json<T: Serialize>(&mut self, kind: u8, v: &T) {
        let bytes = serde_json::to_vec(v).unwrap_or_default();
        self.send(kind, &bytes);
    }

    /// All complete messages received so far.
    pub fn recv(&mut self) -> Vec<(u8, Vec<u8>)> {
        let mut buf = [0u8; 65536];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => {
                    self.closed = true;
                    break;
                }
                Ok(n) => self.inbuf.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    self.closed = true;
                    break;
                }
            }
        }
        let mut out = Vec::new();
        loop {
            if self.inbuf.len() < 4 {
                break;
            }
            let len = u32::from_le_bytes([self.inbuf[0], self.inbuf[1], self.inbuf[2], self.inbuf[3]]) as usize;
            if len == 0 || len > 64 << 20 {
                self.closed = true;
                break;
            }
            if self.inbuf.len() < 4 + len {
                break;
            }
            let kind = self.inbuf[4];
            let payload = self.inbuf[5..4 + len].to_vec();
            self.inbuf.drain(..4 + len);
            out.push((kind, payload));
        }
        out
    }
}

// ------------------------------------------------------------- encoding

struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn color(&mut self, c: Color) {
        self.0.extend_from_slice(&[c.r, c.g, c.b, c.a]);
    }
}

struct R<'a>(&'a [u8], usize);

impl R<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.0.get(self.1..self.1 + n)?;
        self.1 += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i32(&mut self) -> Option<i32> {
        self.take(4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Option<f32> {
        self.take(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn color(&mut self) -> Option<Color> {
        self.take(4).map(|b| Color::rgba(b[0], b[1], b[2], b[3]))
    }
}

/// Serialize a draw list (lz4-compressed).
pub fn encode_frame(list: &DrawList) -> Vec<u8> {
    let mut w = W(Vec::with_capacity(list.cmds.len() * 48 + 16));
    w.u32(list.width);
    w.u32(list.height);
    w.color(list.clear);
    w.u32(list.cmds.len() as u32);
    for c in &list.cmds {
        match c {
            DrawCmd::Fill { rect, color, depth } => {
                w.u8(0);
                for v in [rect.x, rect.y, rect.w, rect.h, *depth] {
                    w.f32(v);
                }
                w.color(*color);
            }
            DrawCmd::Sprite(s) => {
                w.u8(1);
                w.u32(s.tex.0);
                for v in [s.src.x, s.src.y, s.src.w, s.src.h] {
                    w.i32(v);
                }
                for v in [s.dst.x, s.dst.y, s.dst.w, s.dst.h, s.rotation, s.origin.x, s.origin.y, s.depth] {
                    w.f32(v);
                }
                w.color(s.tint);
                w.u8(s.flip_x as u8 | (s.flip_y as u8) << 1);
            }
        }
    }
    lz4_flex::compress_prepend_size(&w.0)
}

/// Inverse of [`encode_frame`]; `map` turns host texture ids into local ones.
pub fn decode_frame(data: &[u8], map: &dyn Fn(u32) -> Option<TexId>) -> Option<DrawList> {
    let raw = lz4_flex::decompress_size_prepended(data).ok()?;
    let mut r = R(&raw, 0);
    let mut list = DrawList::new(r.u32()?, r.u32()?);
    list.clear = r.color()?;
    let n = r.u32()? as usize;
    for _ in 0..n {
        match r.u8()? {
            0 => {
                let (x, y, w, h, depth) = (r.f32()?, r.f32()?, r.f32()?, r.f32()?, r.f32()?);
                list.cmds.push(DrawCmd::Fill { rect: FRect::new(x, y, w, h), color: r.color()?, depth });
            }
            _ => {
                let tex = r.u32()?;
                let src = IRect::new(r.i32()?, r.i32()?, r.i32()?, r.i32()?);
                let (dx, dy, dw, dh, rot, ox, oy, depth) = (r.f32()?, r.f32()?, r.f32()?, r.f32()?, r.f32()?, r.f32()?, r.f32()?, r.f32()?);
                let tint = r.color()?;
                let flips = r.u8()?;
                let Some(local) = map(tex) else { continue };
                let mut s = Sprite::new(local, src, FRect::new(dx, dy, dw, dh), depth);
                s.rotation = rot;
                s.origin = Vec2::new(ox, oy);
                s.tint = tint;
                s.flip_x = flips & 1 != 0;
                s.flip_y = flips & 2 != 0;
                list.cmds.push(DrawCmd::Sprite(s));
            }
        }
    }
    Some(list)
}

/// Texture message: id, size and lz4 RGBA.
pub fn encode_texture(id: TexId, img: &Image) -> Vec<u8> {
    let mut w = W(Vec::new());
    w.u32(id.0);
    w.u32(img.width);
    w.u32(img.height);
    w.0.extend_from_slice(&lz4_flex::compress_prepend_size(&img.rgba));
    w.0
}

pub fn decode_texture(data: &[u8]) -> Option<(u32, Image)> {
    let mut r = R(data, 0);
    let (id, w, h) = (r.u32()?, r.u32()?, r.u32()?);
    let rgba = lz4_flex::decompress_size_prepended(&data[12..]).ok()?;
    if rgba.len() != (w * h * 4) as usize {
        return None;
    }
    let mut img = Image::new(w, h);
    img.rgba = rgba;
    Some((id, img))
}

pub fn net_sound(c: &SoundCmd) -> NetSound {
    match c {
        SoundCmd::Play { cue, volume, pitch } => NetSound::Play { cue: cue.clone(), volume: *volume, pitch: *pitch },
        SoundCmd::Music { track, volume } => NetSound::Music { track: track.clone(), volume: *volume },
        SoundCmd::StopMusic => NetSound::StopMusic,
    }
}

pub fn sound_cmd(s: &NetSound) -> SoundCmd {
    match s {
        NetSound::Play { cue, volume, pitch } => SoundCmd::Play { cue: cue.clone(), volume: *volume, pitch: *pitch },
        NetSound::Music { track, volume } => SoundCmd::Music { track: track.clone(), volume: *volume },
        NetSound::StopMusic => SoundCmd::StopMusic,
    }
}

// ------------------------------------------------------------- host side

/// A connected remote player as seen by the host.
pub struct Remote {
    pub conn: Conn,
    pub name: Option<String>,
    /// Farmhand index once the hello arrived.
    pub farmhand: Option<usize>,
    pub input: InputState,
    pub screen: (u32, u32),
    /// Texture generation last sent, by host texture id.
    pub sent: HashMap<u32, u32>,
    pub ui: crate::PlayerUi,
}

pub struct NetHost {
    listener: TcpListener,
    pub remotes: Vec<Remote>,
}

impl NetHost {
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener, remotes: Vec::new() })
    }

    pub fn local_port(&self) -> u16 {
        self.listener.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    /// Accept new connections.
    pub fn accept(&mut self) {
        while let Ok((stream, _)) = self.listener.accept() {
            if let Ok(conn) = Conn::new(stream) {
                self.remotes.push(Remote { conn, name: None, farmhand: None, input: InputState::default(), screen: (1280, 720), sent: HashMap::new(), ui: crate::PlayerUi::default() });
            }
        }
    }
}

// ------------------------------------------------------------- client side

/// A joining player's connection: sends input, receives frames.
pub struct NetClient {
    pub conn: Conn,
    /// Host texture id -> local texture id.
    pub textures: HashMap<u32, TexId>,
    pub frame: Option<DrawList>,
    pub sounds: Vec<SoundCmd>,
    pub index: Option<usize>,
    pending: Vec<InputEvent>,
}

impl NetClient {
    pub fn join(addr: impl ToSocketAddrs, name: &str) -> io::Result<Self> {
        let mut conn = Conn::connect(addr)?;
        conn.send_json(KIND_HELLO, &Hello { name: name.to_string() });
        Ok(Self { conn, textures: HashMap::new(), frame: None, sounds: Vec::new(), index: None, pending: Vec::new() })
    }

    pub fn push_input(&mut self, e: InputEvent) {
        self.pending.push(e);
    }

    /// Send queued input and process everything received.
    pub fn pump(&mut self, store: &mut engine::TextureStore) {
        if !self.pending.is_empty() {
            let events = std::mem::take(&mut self.pending);
            self.conn.send_json(KIND_INPUT, &events);
        }
        for (kind, payload) in self.conn.recv() {
            match kind {
                KIND_WELCOME => self.index = serde_json::from_slice(&payload).ok(),
                KIND_TEXTURE => {
                    if let Some((id, img)) = decode_texture(&payload) {
                        // Same name replaces the image (and bumps its generation).
                        let local = store.insert(format!("__net/{id}"), img);
                        self.textures.insert(id, local);
                    }
                }
                KIND_FRAME => {
                    let map = |id: u32| self.textures.get(&id).copied();
                    if let Some(f) = decode_frame(&payload, &map) {
                        self.frame = Some(f);
                    }
                }
                KIND_SOUND => {
                    if let Ok(list) = serde_json::from_slice::<Vec<NetSound>>(&payload) {
                        self.sounds.extend(list.iter().map(sound_cmd));
                    }
                }
                _ => {}
            }
        }
    }
}
