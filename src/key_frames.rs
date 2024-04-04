use crate::Lerp;
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub struct KeyFrames<T: Clone + Lerp> {
    /// for each point in time, the value T, that should be held at that time.
    /// Should contain values from 0.0 to 1.0
    /// ascending, e.g. 0.0 : -5.0, 0.1 : 7.0, 1.0
    frames: SmallVec<[(f32, T, Easing); 4]>,
}

impl<T: Clone + Lerp> KeyFrames<T> {
    pub fn new(frames: SmallVec<[(f32, T, Easing); 4]>) -> Self {
        assert!(!frames.is_empty());
        let is_sorted = frames.is_sorted_by(|a, b| a.0 < b.0);
        assert!(is_sorted);

        // assert that there is a positive difference between the times of all frames
        let mut prev = -f32::INFINITY;
        for (t, _, _) in frames.iter() {
            assert!(*t > prev);
            assert!(*t - prev > 0.0);
            prev = *t;
        }

        Self { frames }
    }

    pub fn normalize_time(mut self) -> Self {
        if self.frames.len() == 1 {
            self.frames.first_mut().unwrap().0 = 0.0;
            return self;
        }

        let min = self.frames.first().unwrap().0;
        let max = self.frames.last().unwrap().0;
        let span = max - min;
        for (t, _, _) in self.frames.iter_mut() {
            *t -= min;
            *t /= span;
        }
        self
    }

    pub fn get(&self, t_current: f32) -> T {
        // get two points to interpolate between:

        // find point with t greater than current_t:
        let Some((i_gr, t_gr, v_gr)) = self
            .frames
            .iter()
            .enumerate()
            .find_map(|(i, (t, v, _))| (*t > t_current).then_some((i, t, v)))
        else {
            // could not find any pt greater than current_t, so return last value:
            return self.frames.last().unwrap().1.clone();
        };

        // if the point found is the first point, just return the first value:
        if i_gr == 0 {
            return v_gr.clone();
        }

        // take the point before the greater point:
        let (t_sm, v_sm, easing) = &self.frames[i_gr - 1]; // always safe to do

        // calculate how much we are shifted to the gr pt on a scale from 0.0 to 1.0:
        let factor = (t_current - *t_sm) / (*t_gr - *t_sm);
        // modify the factor by an easing function (taken from the pt smaller than the current t):
        let factor_eased = easing.y(factor);

        v_sm.lerp(v_gr, factor_eased)
    }
}

#[macro_export]
macro_rules! key_frames {
    ($($t:expr => $v:expr),+) => {
      {
        use $crate::key_frames::{Easing, KeyFrames};
        let frames = smallvec::smallvec![$(($t, $v, Easing::Linear )),+];
        KeyFrames::new(frames)
      }
    };
}

#[derive(Debug, Clone, Default)]
pub enum Easing {
    #[default]
    Linear,
    Step,
    EaseInCubic,
    EaseOutCubic,
    EaseInOut,
}

impl Easing {
    #[inline(always)]
    fn y(&self, x: f32) -> f32 {
        match self {
            Easing::Linear => x,
            Easing::Step => x.round(),
            Easing::EaseInCubic => x * x * x,
            Easing::EaseOutCubic => {
                let x_minus_one = x - 1.0;
                1.0 + x_minus_one * x_minus_one * x_minus_one
            }
            Easing::EaseInOut => 0.5 * (1.0 - (x * core::f32::consts::PI).cos()),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_macro() {
        let frames = key_frames!(0.0 => -5.0, 3.0 => -10.0, 4.0 => 20.0);
        assert_eq!(frames.get(-22.0), -5.0);
        assert_eq!(frames.get(5.0), 20.0);
        assert_eq!(frames.get(3.5), 5.0); // between frame 2 and 3
    }
}
