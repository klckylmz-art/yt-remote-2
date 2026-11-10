//! Bounded, synthetic-testable policy, not a capture/codec engine.
use std::{sync::Mutex, time::Duration};

#[path = "../../../third_party/rustdesk/rtt.rs"]
mod rtt;

/// A single pending value shared by worker/UI threads. Replacing or clearing
/// destroys the old value after unlocking, so a slow destructor holds no lock.
pub struct Latest<T>(Mutex<Option<T>>);
impl<T> Default for Latest<T> {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}
impl<T> Latest<T> {
    pub fn put(&self, value: T) {
        let replaced = self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .replace(value);
        drop(replaced);
    }
    pub fn take(&self) -> Option<T> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
    pub fn clear(&self) {
        drop(self.take());
    }
}

/// Targets only: actual delivery depends on capture, encoding and transport.
/// Eight consecutive pressured observations reduce one step; 120 observations
/// with headroom for the *next* step recover one step. One observation should be
/// made per captured frame, using total capture/encode cost and latest ACK RTT.
pub struct AdaptiveVideo {
    level: usize,
    bad: u32,
    good: u32,
    rtt: rtt::RttCalculator,
    floor: Option<u32>,
    stable_min: u32,
    stable_max: u32,
    stable_count: u32,
}
impl Default for AdaptiveVideo {
    fn default() -> Self {
        Self {
            level: 0,
            bad: 0,
            good: 0,
            rtt: rtt::RttCalculator::default(),
            floor: None,
            stable_min: u32::MAX,
            stable_max: 0,
            stable_count: 0,
        }
    }
}
impl AdaptiveVideo {
    const RATES: [u32; 4] = [60, 30, 15, 5];

    pub fn observe(&mut self, processing: Duration, rtt: Duration) {
        let delay = rtt.as_millis().min(u32::MAX as u128) as u32;
        self.rtt.update(delay);
        self.floor = Some(self.floor.unwrap_or(delay).min(delay));

        // The upstream estimator permits small rising floors. Keep an anchor
        // until the whole path settles: slow-growing queues must not become a
        // new baseline. A stable route change can recover after 120 samples.
        self.stable_min = self.stable_min.min(delay);
        self.stable_max = self.stable_max.max(delay);
        if self.stable_max.saturating_sub(self.stable_min) > 5 {
            self.stable_min = delay;
            self.stable_max = delay;
            self.stable_count = 0;
        }
        self.stable_count += 1;
        if self.stable_count >= 120 {
            self.floor = self.rtt.get_rtt();
            self.stable_count = 0;
            self.stable_min = delay;
            self.stable_max = delay;
        }
        let baseline = self
            .rtt
            .get_rtt()
            .unwrap_or(delay)
            .min(self.floor.unwrap_or(delay));
        let queue_ms = delay.saturating_sub(baseline) as f64;
        let budget_ms = 1000.0 / self.fps() as f64;
        let pressured =
            processing.as_secs_f64() * 1000.0 > budget_ms * 0.90 || queue_ms > budget_ms * 2.0;
        if pressured {
            self.bad += 1;
            self.good = 0;
            if self.bad >= 8 {
                self.level = (self.level + 1).min(Self::RATES.len() - 1);
                self.bad = 0;
            }
        } else {
            self.bad = 0;
            let next_budget = 1000.0 / Self::RATES[self.level.saturating_sub(1)] as f64;
            if processing.as_secs_f64() * 1000.0 < next_budget * 0.70
                && queue_ms < next_budget * 0.5
            {
                self.good += 1;
                if self.good >= 120 {
                    self.level = self.level.saturating_sub(1);
                    self.good = 0;
                }
            } else {
                self.good = 0;
            }
        }
    }
    pub fn fps(&self) -> u32 {
        Self::RATES[self.level]
    }
    pub fn interval(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.fps() as f64)
    }
    /// Prefer lower frame rate to aggressive JPEG degradation of formula text.
    pub fn quality(&self) -> u8 {
        90
    }
}

pub struct Viewport {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}
impl Viewport {
    /// Integer aspect-preserving fit, centered with at most one pixel rounding.
    /// Bound dimensions to keep allocation and coordinate arithmetic reasonable.
    /// Decoders/renderers must independently enforce their allocation limits.
    pub fn fit(remote_w: u32, remote_h: u32, view_w: usize, view_h: usize) -> Option<Self> {
        if remote_w == 0
            || remote_h == 0
            || view_w == 0
            || view_h == 0
            || remote_w > 16384
            || remote_h > 16384
            || view_w > 16384
            || view_h > 16384
        {
            return None;
        }
        let rw = u64::from(remote_w);
        let rh = u64::from(remote_h);
        let vw = view_w as u64;
        let vh = view_h as u64;
        let (width, height) = if vw * rh <= vh * rw {
            (view_w, ((vw * rh / rw) as usize).max(1))
        } else {
            (((vh * rw / rh) as usize).max(1), view_h)
        };
        Some(Self {
            x: (view_w - width) / 2,
            y: (view_h - height) / 2,
            width,
            height,
        })
    }

    /// Return normalized remote coordinates. Right/bottom edges and letterbox
    /// clicks are excluded; callers intercept local overlay controls first.
    pub fn remote_point(&self, x: f32, y: f32) -> Option<(f64, f64)> {
        if !x.is_finite() || !y.is_finite() || self.width == 0 || self.height == 0 {
            return None;
        }
        let rx = x as f64 - self.x as f64;
        let ry = y as f64 - self.y as f64;
        if rx < 0.0 || ry < 0.0 || rx >= self.width as f64 || ry >= self.height as f64 {
            return None;
        }
        Some((rx / self.width as f64, ry / self.height as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observe(policy: &mut AdaptiveVideo, count: usize, processing: u64, rtt: u64) {
        for _ in 0..count {
            policy.observe(
                Duration::from_millis(processing),
                Duration::from_millis(rtt),
            );
        }
    }

    #[test]
    fn mailbox_keeps_only_latest_and_clear_discards_pending_frame() {
        let mailbox = Latest::<u32>::default();
        for n in 0..10000 {
            mailbox.put(n);
        }
        assert_eq!(mailbox.take(), Some(9999));
        assert_eq!(mailbox.take(), None);
        mailbox.put(42);
        mailbox.clear();
        assert_eq!(mailbox.take(), None);
    }

    #[test]
    fn sustained_processing_cost_drops_to_thirty() {
        let mut policy = AdaptiveVideo::default();
        assert_eq!(policy.fps(), 60);
        observe(&mut policy, 12, 24, 20);
        assert_eq!(policy.fps(), 30);
        assert!(policy.interval() >= Duration::from_millis(33));
        assert!(policy.quality() >= 85);
    }

    #[test]
    fn isolated_spikes_and_threshold_noise_do_not_oscillate() {
        let mut policy = AdaptiveVideo::default();
        for _ in 0..200 {
            observe(&mut policy, 1, 30, 20);
            observe(&mut policy, 3, 8, 20);
        }
        assert_eq!(policy.fps(), 60);
        observe(&mut policy, 12, 24, 20);
        assert_eq!(policy.fps(), 30);
        for _ in 0..200 {
            observe(&mut policy, 1, 17, 20);
            observe(&mut policy, 1, 14, 20);
            assert_eq!(policy.fps(), 30);
        }
    }

    #[test]
    fn recovery_requires_sustained_headroom_and_has_a_bound() {
        let mut policy = AdaptiveVideo::default();
        observe(&mut policy, 12, 24, 20);
        observe(&mut policy, 20, 5, 20);
        assert_eq!(policy.fps(), 30);
        observe(&mut policy, 600, 5, 20);
        assert_eq!(policy.fps(), 60);
    }

    #[test]
    fn severe_pressure_falls_back_below_thirty_without_collapsing_quality() {
        let mut policy = AdaptiveVideo::default();
        observe(&mut policy, 200, 250, 20);
        assert!(policy.fps() < 30);
        assert!(policy.fps() >= 5);
        assert!(policy.quality() >= 85);
        observe(&mut policy, 1500, 5, 20);
        assert_eq!(policy.fps(), 60);
    }

    #[test]
    fn constant_high_rtt_is_baseline_not_permanent_congestion() {
        let mut policy = AdaptiveVideo::default();
        observe(&mut policy, 2000, 5, 800);
        assert_eq!(policy.fps(), 60);
        observe(&mut policy, 40, 5, 100);
        observe(&mut policy, 40, 5, 800);
        observe(&mut policy, 2000, 5, 800);
        assert_eq!(policy.fps(), 60);
    }

    #[test]
    fn increasing_network_queue_is_not_absorbed_by_baseline() {
        let mut policy = AdaptiveVideo::default();
        observe(&mut policy, 30, 5, 20);
        for n in 1..200 {
            observe(&mut policy, 1, 5, 20 + n * 5);
        }
        assert!(policy.fps() <= 30);
    }

    #[test]
    fn letterbox_points_use_normalized_coordinates_and_half_open_bounds() {
        let viewport = Viewport::fit(1920, 1080, 1000, 1000).unwrap();
        assert_eq!(
            (viewport.x, viewport.y, viewport.width, viewport.height),
            (0, 219, 1000, 562)
        );
        assert_eq!(viewport.remote_point(0.0, 219.0), Some((0.0, 0.0)));
        assert_eq!(viewport.remote_point(500.0, 500.0), Some((0.5, 0.5)));
        assert!(viewport.remote_point(999.0, 780.0).is_some());
        for (x, y) in [
            (-1.0, 500.0),
            (1000.0, 500.0),
            (500.0, 218.0),
            (500.0, 781.0),
            (f32::NAN, 500.0),
            (500.0, f32::INFINITY),
        ] {
            assert_eq!(viewport.remote_point(x, y), None);
        }
    }

    #[test]
    fn portrait_is_centered_and_maps_edges() {
        let viewport = Viewport::fit(1080, 1920, 1920, 1080).unwrap();
        assert_eq!(
            (viewport.x, viewport.y, viewport.width, viewport.height),
            (656, 0, 607, 1080)
        );
        assert_eq!(viewport.remote_point(656.0, 0.0), Some((0.0, 0.0)));
        assert_eq!(viewport.remote_point(1263.0, 1079.0), None);
        assert!(viewport.remote_point(1262.0, 1079.0).is_some());
    }

    #[test]
    fn rejects_zero_and_unreasonable_dimensions_without_overflow() {
        for dims in [
            (0, 1080, 100, 100),
            (1920, 0, 100, 100),
            (1920, 1080, 0, 100),
            (1920, 1080, 100, 0),
            (u32::MAX, 1080, 100, 100),
            (1920, 1080, usize::MAX, 100),
        ] {
            assert!(Viewport::fit(dims.0, dims.1, dims.2, dims.3).is_none());
        }
        let tiny = Viewport::fit(1920, 1080, 1, 1).unwrap();
        assert_eq!((tiny.width, tiny.height), (1, 1));
    }
}
