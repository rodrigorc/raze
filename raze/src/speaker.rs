/*
 * This module implements a simple mono speaker.
 */

pub struct Speaker {
    audio: Vec<f32>,
    audio_time: u32,
    audio_accum: u32,
    t_per_sample: u32,
}

impl Speaker {
    pub fn new(t_per_sample: u32) -> Speaker {
        /* This is the freq. of the CPU divided by the freq. of the audio sample frequency: 22050 Hz
         * 48k model runs at 3.5 MHz
         * 128k runs at 3.5469 MHz
         */
        Speaker {
            audio: vec![],
            audio_time: 0,
            audio_accum: 0,
            t_per_sample,
        }
    }
    pub fn clear(&mut self) {
        self.audio.clear();
    }
    pub fn push_sample(&mut self, sample: u32, t: u32) {
        self.audio_time += t;
        self.audio_accum += t * sample;
        while self.audio_time >= self.t_per_sample {
            //remove the excess samples
            self.audio_time -= self.t_per_sample;
            let audio_excess = self.audio_time * sample;
            self.push_audio_accum(self.audio_accum - audio_excess);
            self.audio_accum = audio_excess;
        }
    }
    pub fn complete_frame(
        &mut self,
        full_time: u32,
        mut sample_fn: impl FnMut() -> u32,
    ) -> &mut [f32] {
        while self.audio.len() < (full_time / self.t_per_sample) as usize {
            let s = sample_fn();
            self.push_sample(s, self.t_per_sample - self.audio_time);
        }
        &mut self.audio
    }

    fn push_audio_accum(&mut self, sample: u32) {
        let v = sample as f32 / (65536 * self.t_per_sample) as f32 - 0.1;
        self.audio.push(v);
    }
}
