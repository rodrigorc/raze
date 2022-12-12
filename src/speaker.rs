/*
 * This module implements a simple mono speaker.
 */

pub struct Speaker {
    audio: Vec<f32>,
    audio_time: u32,
    audio_accum: u32,
    sample_rate: u32,
}

impl Speaker {
    pub fn new(is128k: bool) -> Speaker {
        /* This is the freq. of the CPU divided by the freq. of the audio sample frequency: 22050 Hz
        * 48k model runs at 3.5 MHz
        * 128k runs at 3.5469 MHz
        */
        let sampler = 22050;
        let sample_rate = (if is128k { 3_546_900 } else { 3_500_000 } + sampler / 2) / sampler;
        Speaker {
            audio: vec![],
            audio_time: 0,
            audio_accum: 0,
            sample_rate,
        }
    }
    pub fn clear(&mut self) {
        self.audio.clear();
    }
    pub fn push_sample(&mut self, sample: u32, t: u32) {
        self.audio_time += t;
        self.audio_accum += t * sample;
        while self.audio_time >= self.sample_rate {
            //remove the excess samples
            self.audio_time -= self.sample_rate;
            let audio_excess = self.audio_time * sample;
            self.push_audio_accum(self.audio_accum - audio_excess);
            self.audio_accum = audio_excess;
        }
    }
    pub fn complete_frame(&mut self, full_time: u32, mut sample_fn: impl FnMut() -> u32) -> &mut [f32] {
        while self.audio.len() < (full_time / self.sample_rate) as usize {
            let s = sample_fn();
            self.push_sample(s, self.sample_rate - self.audio_time);
        }
        &mut self.audio
    }

    fn push_audio_accum(&mut self, sample: u32) {
        let v = sample as f32 / (65536 * self.sample_rate) as f32 - 0.1;
        self.audio.push(v);
    }
}

