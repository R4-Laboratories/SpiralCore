//! Retry timing after a verified empty mailbox response.
use std::time::{Duration, Instant};

pub(crate) struct EmptyPollDelay(Duration);

impl Default for EmptyPollDelay {
    fn default() -> Self {
        Self(Duration::from_millis(5))
    }
}

impl EmptyPollDelay {
    fn next(&mut self, remaining: Duration) -> Duration {
        let delay = self.0.min(remaining);
        self.0 = (self.0 * 2).min(Duration::from_millis(80));
        delay
    }

    pub(crate) fn wait(&mut self, deadline: Instant) {
        std::thread::sleep(self.next(deadline.saturating_duration_since(Instant::now())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_empty_replies_back_off_but_a_new_delivery_starts_promptly() {
        let mut delay = EmptyPollDelay::default();
        let budget = Duration::from_secs(2);
        for expected_ms in [5, 10, 20, 40, 80, 80, 80] {
            assert_eq!(delay.next(budget), Duration::from_millis(expected_ms));
        }
        assert_eq!(
            EmptyPollDelay::default().next(budget),
            Duration::from_millis(5)
        );
    }

    #[test]
    fn sleep_is_clipped_to_the_original_deadline_even_below_one_millisecond() {
        let mut delay = EmptyPollDelay::default();
        let left = Duration::from_micros(250);
        assert_eq!(delay.next(left), left);
        assert_eq!(delay.next(Duration::ZERO), Duration::ZERO);
    }
}
