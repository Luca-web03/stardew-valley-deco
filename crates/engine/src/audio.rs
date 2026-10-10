/// Fire-and-forget audio requests. Games push commands; the platform
/// backend drains the queue each frame and plays whatever it can resolve
/// (see [`crate::mixer`] for the mixer backends build on).
#[derive(Clone, Debug, PartialEq)]
pub enum SoundCmd {
    /// Play a one-shot cue by name (e.g. "hoeHit"). `pitch` is a multiplier.
    Play { cue: String, volume: f32, pitch: f32 },
    /// Start (or switch to) a looping music track. Empty name = silence.
    Music { track: String, volume: f32 },
    StopMusic,
}

/// Commands kept when nobody drains the queue (e.g. headless runs); older
/// one-shots are dropped first.
pub const MAX_QUEUED: usize = 256;

#[derive(Default, Debug)]
pub struct AudioQueue {
    pub cmds: Vec<SoundCmd>,
}

impl AudioQueue {
    pub fn push(&mut self, cmd: SoundCmd) {
        if self.cmds.len() >= MAX_QUEUED {
            let idx = self.cmds.iter().position(|c| matches!(c, SoundCmd::Play { .. })).unwrap_or(0);
            self.cmds.remove(idx);
        }
        self.cmds.push(cmd);
    }
    pub fn play(&mut self, cue: &str) {
        self.play_with(cue, 1.0, 1.0);
    }
    pub fn play_pitched(&mut self, cue: &str, pitch: f32) {
        self.play_with(cue, 1.0, pitch);
    }
    pub fn play_with(&mut self, cue: &str, volume: f32, pitch: f32) {
        self.push(SoundCmd::Play { cue: cue.to_string(), volume, pitch });
    }
    pub fn music(&mut self, track: &str) {
        self.push(SoundCmd::Music { track: track.to_string(), volume: 1.0 });
    }
    pub fn stop_music(&mut self) {
        self.push(SoundCmd::StopMusic);
    }
    pub fn drain(&mut self) -> Vec<SoundCmd> {
        std::mem::take(&mut self.cmds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_is_bounded_and_keeps_music() {
        let mut q = AudioQueue::default();
        q.music("theme");
        for i in 0..MAX_QUEUED + 10 {
            q.play(&format!("c{i}"));
        }
        assert_eq!(q.cmds.len(), MAX_QUEUED);
        assert!(matches!(&q.cmds[0], SoundCmd::Music { track, .. } if track == "theme"));
        assert!(matches!(q.cmds.last(), Some(SoundCmd::Play { cue, .. }) if cue == &format!("c{}", MAX_QUEUED + 9)));
        assert_eq!(q.drain().len(), MAX_QUEUED);
        assert!(q.cmds.is_empty());
    }
}
