//! Frame timing.
//!
//! Split out from the event loop so it can be tested without a window: every
//! method takes the current [`Instant`] as a parameter rather than reading the
//! clock itself, which lets a test drive it with fixed values.

use std::time::Instant;

/// How long this frame took, and how long the loop has been running.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FrameTime {
    /// Seconds since the previous frame.
    pub delta_seconds: f32,
    /// Seconds since the clock started.
    pub elapsed_seconds: f32,
}

/// Tracks the two timestamps the render loop needs.
pub(super) struct FrameClock {
    started_at: Instant,
    last_frame_at: Instant,
}

impl FrameClock {
    /// Starts the clock. `now` becomes both the origin for `elapsed_seconds`
    /// and the baseline for the first frame's delta.
    pub(super) fn new(now: Instant) -> Self {
        FrameClock {
            started_at: now,
            last_frame_at: now,
        }
    }

    /// Closes off one frame and returns its timing.
    pub(super) fn tick(&mut self, now: Instant) -> FrameTime {
        let time = FrameTime {
            delta_seconds: now.duration_since(self.last_frame_at).as_secs_f32(),
            elapsed_seconds: now.duration_since(self.started_at).as_secs_f32(),
        };
        self.last_frame_at = now;
        time
    }

    /// Discards the time since the last tick without touching `elapsed_seconds`.
    ///
    /// Used after a long pause that was not a frame — window and device
    /// creation, mainly — so the first real frame does not open with a delta
    /// covering all of startup.
    pub(super) fn discard_gap(&mut self, now: Instant) {
        self.last_frame_at = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// An arbitrary but fixed origin. Nothing here reads the real clock, so
    /// every assertion below is deterministic.
    fn origin() -> Instant {
        Instant::now()
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn the_first_tick_measures_from_the_start() {
        let t0 = origin();
        let mut clock = FrameClock::new(t0);

        let time = clock.tick(t0 + ms(16));
        assert!(approx(time.delta_seconds, 0.016));
        assert!(approx(time.elapsed_seconds, 0.016));
    }

    #[test]
    fn delta_is_per_frame_while_elapsed_accumulates() {
        let t0 = origin();
        let mut clock = FrameClock::new(t0);

        let first = clock.tick(t0 + ms(16));
        let second = clock.tick(t0 + ms(32));
        let third = clock.tick(t0 + ms(64));

        assert!(approx(first.delta_seconds, 0.016));
        assert!(approx(second.delta_seconds, 0.016));
        assert!(approx(third.delta_seconds, 0.032));

        assert!(approx(second.elapsed_seconds, 0.032));
        assert!(approx(third.elapsed_seconds, 0.064));
    }

    #[test]
    fn elapsed_is_monotonic_across_frames() {
        let t0 = origin();
        let mut clock = FrameClock::new(t0);

        let mut previous = 0.0;
        for frame in 1..=10u64 {
            let time = clock.tick(t0 + ms(frame * 17));
            assert!(time.elapsed_seconds > previous);
            previous = time.elapsed_seconds;
        }
    }

    #[test]
    fn two_ticks_at_the_same_instant_report_a_zero_delta() {
        let t0 = origin();
        let mut clock = FrameClock::new(t0);

        clock.tick(t0 + ms(16));
        let time = clock.tick(t0 + ms(16));
        assert_eq!(time.delta_seconds, 0.0);
        assert!(approx(time.elapsed_seconds, 0.016));
    }

    #[test]
    fn a_long_stall_is_reported_rather_than_hidden() {
        // The clock does not clamp: a game that wants a maximum step has to
        // decide that for itself, because the right cap is game-specific.
        let t0 = origin();
        let mut clock = FrameClock::new(t0);

        let time = clock.tick(t0 + Duration::from_secs(3));
        assert!(approx(time.delta_seconds, 3.0));
    }

    #[test]
    fn discard_gap_drops_the_pause_but_keeps_elapsed_running() {
        let t0 = origin();
        let mut clock = FrameClock::new(t0);

        // Window and device creation take 500ms and are not a frame.
        clock.discard_gap(t0 + ms(500));

        let time = clock.tick(t0 + ms(516));
        assert!(
            approx(time.delta_seconds, 0.016),
            "the first frame must not inherit startup time, got {}",
            time.delta_seconds
        );
        assert!(
            approx(time.elapsed_seconds, 0.516),
            "elapsed still counts from when the clock started, got {}",
            time.elapsed_seconds
        );
    }

    #[test]
    fn discard_gap_does_not_rewind_elapsed_on_the_next_tick() {
        let t0 = origin();
        let mut clock = FrameClock::new(t0);

        let before = clock.tick(t0 + ms(100));
        clock.discard_gap(t0 + ms(900));
        let after = clock.tick(t0 + ms(916));

        assert!(after.elapsed_seconds > before.elapsed_seconds);
        assert!(approx(after.delta_seconds, 0.016));
    }
}
