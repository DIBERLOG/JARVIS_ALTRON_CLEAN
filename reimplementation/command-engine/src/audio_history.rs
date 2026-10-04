use std::collections::VecDeque;

/// Bounded pre-roll storage: oldest frames are replaced first.
pub struct AudioHistory {
    frames: VecDeque<Vec<i16>>,
    capacity: usize,
}

impl AudioHistory {
    pub fn new(seconds: f32, frame_size: usize, sample_rate: usize) -> Self {
        let capacity = if seconds.is_finite() && seconds > 0. && frame_size > 0 {
            (seconds as f64 * sample_rate as f64 / frame_size as f64)
                .ceil()
                .min(100_000.) as usize
        } else {
            0
        };
        Self {
            frames: VecDeque::new(),
            capacity,
        }
    }
    pub fn push(&mut self, samples: &[i16]) {
        if self.capacity == 0 {
            return;
        }
        if self.frames.len() == self.capacity {
            self.frames.pop_front();
        }
        self.frames.push_back(samples.to_vec());
    }
    pub fn drain_all(&mut self) -> Vec<Vec<i16>> {
        self.frames.drain(..).collect()
    }
    pub fn len(&self) -> usize {
        self.frames.len()
    }
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_newest_frames_in_original_order() {
        let mut history = AudioHistory::new(1., 4, 8);
        history.push(&[1]);
        history.push(&[2]);
        history.push(&[3]);
        assert_eq!(history.drain_all(), vec![vec![2], vec![3]]);
        assert!(history.is_empty());
    }
    #[test]
    fn invalid_capacity_never_panics_or_grows() {
        let mut history = AudioHistory::new(1., 0, 16000);
        history.push(&[1, 2]);
        assert_eq!(history.len(), 0);
    }
}
