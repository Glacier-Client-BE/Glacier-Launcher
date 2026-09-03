//! Progress reporting, throttled so callers aren't spammed on every chunk.
//!
//! Ports the intent of `ThrottledProgress` in the C# `HttpFactory.cs`: always let
//! through the first (`<= 0`), last (`>= complete_value`) update, and otherwise only
//! forward an update if enough wall-clock time *and* enough delta has accumulated
//! since the last one that was let through.

use std::time::{Duration, Instant};

/// A progress value in `[0.0, complete_value]`, typically `[0.0, 1.0]`.
pub type ProgressValue = f64;

/// Wraps a progress callback and only invokes it when the throttling condition is met.
pub struct ThrottledProgress<F: FnMut(ProgressValue)> {
    inner: F,
    min_interval: Duration,
    complete_value: ProgressValue,
    min_delta: ProgressValue,
    last_instant: Option<Instant>,
    last_value: ProgressValue,
}

impl<F: FnMut(ProgressValue)> ThrottledProgress<F> {
    pub fn new(inner: F) -> Self {
        Self::with_options(inner, Duration::from_millis(100), 1.0, 0.01)
    }

    pub fn with_options(
        inner: F,
        min_interval: Duration,
        complete_value: ProgressValue,
        min_delta: ProgressValue,
    ) -> Self {
        Self {
            inner,
            min_interval,
            complete_value,
            min_delta,
            last_instant: None,
            last_value: 0.0,
        }
    }

    /// Reports a progress value, forwarding it to the inner callback only if the
    /// throttling condition allows it. Returns whether the value was forwarded.
    pub fn report(&mut self, value: ProgressValue) -> bool {
        self.report_at(value, Instant::now())
    }

    /// Same as [`Self::report`] but with an injectable clock, for deterministic tests.
    fn report_at(&mut self, value: ProgressValue, now: Instant) -> bool {
        let should_report = value <= 0.0
            || value >= self.complete_value
            || match self.last_instant {
                None => true,
                Some(last) => {
                    now.duration_since(last) >= self.min_interval
                        && (value - self.last_value).abs() >= self.min_delta
                }
            };

        if should_report {
            self.last_instant = Some(now);
            self.last_value = value;
            (self.inner)(value);
        }
        should_report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn collector() -> (Rc<RefCell<Vec<ProgressValue>>>, impl FnMut(ProgressValue)) {
        let values = Rc::new(RefCell::new(Vec::new()));
        let sink = values.clone();
        let cb = move |v: ProgressValue| sink.borrow_mut().push(v);
        (values, cb)
    }

    #[test]
    fn first_update_always_reported() {
        let (values, cb) = collector();
        let mut throttled =
            ThrottledProgress::with_options(cb, Duration::from_millis(100), 1.0, 0.01);
        let now = Instant::now();
        assert!(throttled.report_at(0.5, now));
        assert_eq!(*values.borrow(), vec![0.5]);
    }

    #[test]
    fn zero_and_complete_always_reported() {
        let (values, cb) = collector();
        let mut throttled =
            ThrottledProgress::with_options(cb, Duration::from_millis(100), 1.0, 0.5);
        let now = Instant::now();
        assert!(throttled.report_at(0.0, now));
        assert!(throttled.report_at(1.0, now));
        assert_eq!(*values.borrow(), vec![0.0, 1.0]);
    }

    #[test]
    fn suppresses_updates_within_interval_and_delta() {
        let (values, cb) = collector();
        let mut throttled =
            ThrottledProgress::with_options(cb, Duration::from_millis(100), 1.0, 0.1);
        let now = Instant::now();
        assert!(throttled.report_at(0.3, now));
        // Too soon and too small a delta.
        assert!(!throttled.report_at(0.32, now + Duration::from_millis(10)));
        assert_eq!(*values.borrow(), vec![0.3]);
    }

    #[test]
    fn reports_once_interval_and_delta_both_satisfied() {
        let (values, cb) = collector();
        let mut throttled =
            ThrottledProgress::with_options(cb, Duration::from_millis(100), 1.0, 0.1);
        let now = Instant::now();
        assert!(throttled.report_at(0.3, now));
        assert!(throttled.report_at(0.45, now + Duration::from_millis(150)));
        assert_eq!(*values.borrow(), vec![0.3, 0.45]);
    }

    #[test]
    fn enough_time_but_not_enough_delta_is_suppressed() {
        let (values, cb) = collector();
        let mut throttled =
            ThrottledProgress::with_options(cb, Duration::from_millis(100), 1.0, 0.5);
        let now = Instant::now();
        assert!(throttled.report_at(0.3, now));
        assert!(!throttled.report_at(0.35, now + Duration::from_millis(200)));
        assert_eq!(*values.borrow(), vec![0.3]);
    }
}
