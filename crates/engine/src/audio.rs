/// Fire-and-forget audio requests. Games push commands; the platform
/// backend drains the queue each frame and plays whatever it can resolve.
#[derive(Clone, Debug, PartialEq)]
pub enum SoundCmd {
    /// Play a one-shot cue by name (e.g. "hoeHit"). `pitch` is a multiplier.
    Play { cue: String, volume: f32, pitch: f32 },
    /// Start (or switch to) a looping music track. Empty name = silence.
    Music { track: String, volume: f32 },
    StopMusic,
}

#[derive(Default, Debug)]
pub struct AudioQueue {
    pub cmds: Vec<SoundCmd>,
}

impl AudioQueue {
    pub fn play(&mut self, cue: &str) {
        self.cmds.push(SoundCmd::Play { cue: cue.to_string(), volume: 1.0, pitch: 1.0 });
    }
    pub fn play_pitched(&mut self, cue: &str, pitch: f32) {
        self.cmds.push(SoundCmd::Play { cue: cue.to_string(), volume: 1.0, pitch });
    }
    pub fn music(&mut self, track: &str) {
        self.cmds.push(SoundCmd::Music { track: track.to_string(), volume: 1.0 });
    }
    pub fn drain(&mut self) -> Vec<SoundCmd> {
        std::mem::take(&mut self.cmds)
    }
}
