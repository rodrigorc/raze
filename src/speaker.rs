/*
 * This module implements a simple mono speaker with low-pass filter.
 * The basic algorithm is copied from the one in MAME but the code is much simpler. */

const FILTER_LENGTH : usize = 64;
const RATE_MULTIPLIER : i32 = 4;
const AUDIO_SAMPLE : i32 = 168 / RATE_MULTIPLIER;

pub struct Speaker {
    unfiltered: [i32; FILTER_LENGTH],
    index: usize,
    audio: Vec<f32>,
    filter: [f32; FILTER_LENGTH],
    audio_time: i32,
    audio_accum: i32,
}

impl Speaker {
    pub fn new() -> Speaker {
        const FILTER_STEP : f32 = std::f32::consts::PI / 2.0 / RATE_MULTIPLIER as f32;
        let mut filter = [0.0; FILTER_LENGTH];
        for (i, f) in filter.iter_mut().enumerate() {
            let x = ((1 - FILTER_LENGTH as i32 + 2 * i as i32) as f32) * (FILTER_STEP / 2.0);
            *f = if x == 0.0 { 1.0 } else { x.sin() / x };
        }
        //We add the normalization factor to the filter so that
        //we avoid an extra division at the end.
        //Also, sampled values are acumulated over AUDIO_SAMPLE time, so we add
        //this factor here, too.
        let multiplier = filter.iter().sum::<f32>() * 32768.0 * AUDIO_SAMPLE as f32;
        for f in filter.iter_mut() {
            *f /= multiplier;
        }

        Speaker {
            unfiltered: [0; FILTER_LENGTH],
            index: 0,
            audio: vec![],
            filter,
            audio_time: 0,
            audio_accum: 0,
        }
    }
    pub fn clear(&mut self) {
        self.audio.clear();
    }
    pub fn push_sample(&mut self, sample: i16, t: i32) {
        self.audio_time += t;
        self.audio_accum += t * sample as i32;
        while self.audio_time >= AUDIO_SAMPLE {
            //remove the excess samples
            self.audio_time -= AUDIO_SAMPLE;
            let audio_excess = self.audio_time * sample as i32;
            self.push_intermediate_audio_accum(self.audio_accum - audio_excess);
            self.audio_accum = audio_excess;
        }
    }
    pub fn complete_frame(&mut self, full_time: i32, mut sample_fn: impl FnMut() -> i16) -> &mut [f32] {
        while self.audio.len() < (full_time / AUDIO_SAMPLE / RATE_MULTIPLIER) as usize {
            let s = sample_fn();
            self.push_sample(s, AUDIO_SAMPLE - self.audio_time);
        }
        &mut self.audio
    }

    fn push_intermediate_audio_accum(&mut self, sample: i32) {
        self.unfiltered[self.index] = sample;
        self.index = (self.index + 1) % FILTER_LENGTH;

        if self.index as i32 % RATE_MULTIPLIER == 0 {
            let vol : f32 = self.filter.iter().zip(self.unfiltered.iter().cycle().skip(self.index))
                .map(|(&factor, &unfiltered)| factor * (unfiltered as f32))
                .sum();
            self.audio.push(vol);
        }
    }
}

