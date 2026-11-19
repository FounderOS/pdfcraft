//! Saves handed to the host and not yet acknowledged: each `save` carries an `id`, and the host
//! answers `saved` with it once the file is stored. Autosave ([`crate::autosave`]) waits while one
//! is out, so a second save never overtakes the first. A host that never answers holds nothing up
//! for longer than [`TIMEOUT`].

use web_time::{Duration, Instant};

/// How long an unanswered save counts as running.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// The saves out, shared by the bridge (which posts them and hears `saved`) and the shell.
#[cfg(target_arch = "wasm32")]
pub type Shared = std::rc::Rc<std::cell::RefCell<InFlight>>;

#[derive(Default)]
pub struct InFlight {
    next: u64,
    /// Each save's id and when it was posted.
    out: Vec<(String, Instant)>,
}

impl InFlight {
    /// A save is about to be posted → its id. Call [`Self::done`] with it if the post fails.
    pub fn begin(&mut self, now: Instant) -> String {
        self.next += 1;
        let id = format!("save-{}", self.next);
        self.out.push((id.clone(), now));
        id
    }

    /// The host answered `saved` for `id`, or the save never left.
    pub fn done(&mut self, id: &str) {
        self.out.retain(|(out, _)| out != id);
    }

    /// A save is still out: posted under [`TIMEOUT`] ago and not answered.
    pub fn running(&mut self, now: Instant) -> bool {
        self.out.retain(|(_, at)| now.saturating_duration_since(*at) < TIMEOUT);
        !self.out.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autosave::Autosave;

    fn at(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    #[test]
    fn a_save_runs_until_the_host_answers() {
        let t = Instant::now();
        let mut f = InFlight::default();
        assert!(!f.running(t));
        let id = f.begin(t);
        assert!(f.running(at(t, 5_000)));
        f.done("save-other");
        assert!(f.running(at(t, 5_000)), "another id answers nothing");
        f.done(&id);
        assert!(!f.running(at(t, 5_000)));
    }

    #[test]
    fn an_unanswered_save_stops_counting_after_the_timeout() {
        let t = Instant::now();
        let mut f = InFlight::default();
        f.begin(t);
        assert!(f.running(at(t, 29_999)));
        assert!(!f.running(at(t, 30_000)));
    }

    #[test]
    fn ids_are_not_reused() {
        let t = Instant::now();
        let mut f = InFlight::default();
        let a = f.begin(t);
        let b = f.begin(t);
        assert_ne!(a, b);
        f.done(&a);
        assert!(f.running(t), "the second is still out");
    }

    /// The first autosave is out and unanswered; the user edits again and rests: no second
    /// autosave until the host answers (or the save times out).
    #[test]
    fn no_second_autosave_while_the_first_is_out() {
        let t = Instant::now();
        let mut a = Autosave::new(t);
        let mut f = InFlight::default();
        a.step(Some((1, 0)), false, false, f.running(t), t);
        a.step(Some((1, 1)), true, false, f.running(t), t);
        assert!(a.step(Some((1, 1)), true, false, f.running(at(t, 3_000)), at(t, 3_000)), "first autosave");
        let id = f.begin(at(t, 3_000));
        // Saved: clean, then edited again.
        a.step(Some((1, 1)), false, false, f.running(at(t, 3_100)), at(t, 3_100));
        a.step(Some((1, 2)), true, false, f.running(at(t, 3_200)), at(t, 3_200));
        assert!(!a.step(Some((1, 2)), true, false, f.running(at(t, 7_000)), at(t, 7_000)), "the first is still out");
        f.done(&id);
        assert!(a.step(Some((1, 2)), true, false, f.running(at(t, 7_250)), at(t, 7_250)), "answered: the second goes");
    }

    #[test]
    fn a_host_that_never_answers_holds_autosave_up_to_the_timeout() {
        let t = Instant::now();
        let mut a = Autosave::new(t);
        let mut f = InFlight::default();
        a.step(Some((1, 0)), false, false, false, t);
        f.begin(t);
        a.step(Some((1, 1)), true, false, f.running(at(t, 100)), at(t, 100));
        assert!(!a.step(Some((1, 1)), true, false, f.running(at(t, 29_000)), at(t, 29_000)));
        assert!(a.step(Some((1, 1)), true, false, f.running(at(t, 30_000)), at(t, 30_000)));
    }
}
