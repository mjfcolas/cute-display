//! A steady clock for the whole app image in a test: its threads sleep on it, and the
//! test moves time on. Once they are all asleep, only one of them runs at a time, woken
//! in the order of their deadlines, so a test sees the same interleaving every run; until
//! then, they start as the system schedules them.

use core::time::Duration;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use hal::steady::SteadyClock;

/// Real time the threads are given to be all asleep, before the test fails: long enough
/// for a slow machine, short enough to report a thread stuck elsewhere.
const STUCK_AFTER: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct FakeSteppedClock(Arc<Timeline>);

struct Timeline {
    state: Mutex<State>,
    /// Told when a thread falls asleep: the test, moving time on, waits on it.
    asleep: Condvar,
}

/// Handed out in the order the sleeps come, which breaks ties between equal deadlines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Ticket(u64);

struct State {
    now: Instant,
    threads: usize,
    stuck_after: Duration,
    sleepers: Vec<Sleeper>,
    next_ticket: Ticket,
    running: Option<Ticket>,
}

struct Sleeper {
    deadline: Instant,
    ticket: Ticket,
    /// Its own, so that waking it wakes no other.
    woken: Arc<Condvar>,
}

impl FakeSteppedClock {
    /// Exactly the threads that sleep on it: one fewer is reported stuck, one more is
    /// refused.
    pub fn for_threads(threads: usize) -> Self {
        let state = State { now: Instant::now(), threads, stuck_after: STUCK_AFTER, sleepers: Vec::new(), next_ticket: Ticket(0), running: None };
        Self(Arc::new(Timeline { state: Mutex::new(state), asleep: Condvar::new() }))
    }

    /// For a test of a thread meant to get stuck, so that it fails quickly.
    pub fn stuck_after(self, real_time: Duration) -> Self {
        self.lock().stuck_after = real_time;
        self
    }

    /// Wakes each thread whose deadline falls within `by`, earliest first, and waits until
    /// it sleeps again, then leaves the time at the end of `by`.
    pub fn advance(&self, by: Duration) {
        let mut state = self.lock();
        let target = later(state.now, by);
        loop {
            state = self.until_all_asleep(state);
            let due = state.sleepers.iter().enumerate().filter(|(_, sleeper)| sleeper.deadline <= target);
            let Some((at, _)) = due.min_by_key(|(_, sleeper)| (sleeper.deadline, sleeper.ticket)) else {
                break;
            };
            let sleeper = state.sleepers.swap_remove(at);
            state.now = state.now.max(sleeper.deadline);
            state.running = Some(sleeper.ticket);
            sleeper.woken.notify_one();
        }
        state.now = target;
    }

    fn until_all_asleep<'a>(&'a self, state: MutexGuard<'a, State>) -> MutexGuard<'a, State> {
        let stuck_after = state.stuck_after;
        let (state, waited) = self
            .0
            .asleep
            .wait_timeout_while(state, stuck_after, |state| state.running.is_some() || state.sleepers.len() < state.threads)
            .unwrap_or_else(PoisonError::into_inner);
        assert!(
            !waited.timed_out(),
            "{} of {} threads asleep after {stuck_after:?}: one is stuck elsewhere, or has ended",
            state.sleepers.len(),
            state.threads
        );
        state
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn later(now: Instant, by: Duration) -> Instant {
    now.checked_add(by).expect("a time beyond what an Instant holds")
}

impl SteadyClock for FakeSteppedClock {
    fn now(&self) -> Instant {
        self.lock().now
    }

    fn sleep(&self, duration: Duration) {
        let mut state = self.lock();
        assert!(state.sleepers.len() < state.threads, "more threads sleep on the clock than for_threads({}) said", state.threads);
        let ticket = state.next_ticket;
        state.next_ticket = Ticket(ticket.0 + 1);
        let deadline = later(state.now, duration);
        let woken = Arc::new(Condvar::new());
        state.sleepers.push(Sleeper { deadline, ticket, woken: woken.clone() });
        // The others are asleep, the count being exact: this sleep ends the running one's turn.
        state.running = None;
        self.0.asleep.notify_one();
        drop(woken.wait_while(state, |state| state.running != Some(ticket)).unwrap_or_else(PoisonError::into_inner));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;

    use super::*;
    use crate::steady_clock;

    type Heard = Arc<Mutex<Vec<(u128, &'static str)>>>;

    /// How many of `every`'s threads ran at once, at most.
    #[derive(Clone, Default)]
    struct Running {
        now: Arc<AtomicUsize>,
        most: Arc<AtomicUsize>,
    }

    fn every(clock: &FakeSteppedClock, period: Duration, name: &'static str, heard: &Heard, running: &Running) {
        let (clock, heard, running, started) = (clock.clone(), heard.clone(), running.clone(), clock.now());
        thread::spawn(move || loop {
            clock.sleep(period);
            let at_once = running.now.fetch_add(1, Ordering::SeqCst) + 1;
            running.most.fetch_max(at_once, Ordering::SeqCst);
            heard.lock().unwrap().push((clock.now().duration_since(started).as_millis(), name));
            thread::yield_now();
            running.now.fetch_sub(1, Ordering::SeqCst);
        });
    }

    fn count(heard: &[(u128, &'static str)], name: &str) -> usize {
        heard.iter().filter(|(_, heard)| *heard == name).count()
    }

    /// Three threads paced as the app image's are.
    fn app_like(clock: &FakeSteppedClock) -> (Heard, Running) {
        let (heard, running) = (Heard::default(), Running::default());
        every(clock, Duration::from_millis(20), "ui", &heard, &running);
        every(clock, Duration::from_millis(100), "main", &heard, &running);
        every(clock, Duration::from_secs(1), "network", &heard, &running);
        (heard, running)
    }

    #[test]
    fn threads_wake_one_at_a_time_at_their_deadlines_in_order() {
        let clock = FakeSteppedClock::for_threads(3);
        let (heard, running) = app_like(&clock);
        clock.advance(Duration::from_secs(1));
        let heard = heard.lock().unwrap().clone();
        assert_eq!((count(&heard, "ui"), count(&heard, "main"), count(&heard, "network")), (50, 10, 1));
        assert!(heard.windows(2).all(|pair| pair[0].0 <= pair[1].0), "in the order of the deadlines");
        assert_eq!(running.most.load(Ordering::SeqCst), 1, "one at a time");
    }

    #[test]
    fn at_the_same_deadline_the_one_asleep_first_wakes_first() {
        let clock = FakeSteppedClock::for_threads(3);
        let (heard, _) = app_like(&clock);
        clock.advance(Duration::from_secs(1));
        let at_one_second: Vec<&str> = heard.lock().unwrap().iter().filter(|(at, _)| *at == 1000).map(|(_, name)| *name).collect();
        assert_eq!(at_one_second, ["network", "main", "ui"], "asleep at 0 s, 0.9 s and 0.98 s");
    }

    #[test]
    fn the_same_steps_come_in_the_same_order_every_time() {
        let run = || {
            let clock = FakeSteppedClock::for_threads(2);
            let (heard, running) = (Heard::default(), Running::default());
            every(&clock, Duration::from_millis(30), "a", &heard, &running);
            every(&clock, Duration::from_millis(45), "b", &heard, &running);
            clock.advance(Duration::from_millis(500));
            let heard = heard.lock().unwrap().clone();
            heard
        };
        let first = run();
        assert!((0..20).all(|_| run() == first));
    }

    #[test]
    fn time_stands_between_advances_and_ends_where_asked() {
        let clock = FakeSteppedClock::for_threads(1);
        let start = clock.now();
        every(&clock, Duration::from_secs(60), "rare", &Heard::default(), &Running::default());
        clock.advance(Duration::from_millis(1500));
        assert_eq!(clock.now() - start, Duration::from_millis(1500));
        thread::sleep(Duration::from_millis(20));
        assert_eq!(clock.now() - start, Duration::from_millis(1500), "no time of its own");
    }

    #[test]
    fn a_stepped_clock_keeps_the_contract() {
        let clock = FakeSteppedClock::for_threads(1);
        let (checked, heard) = mpsc::channel();
        let sleeper = clock.clone();
        thread::spawn(move || {
            steady_clock::check_contract(&sleeper);
            checked.send(()).unwrap();
            loop {
                sleeper.sleep(Duration::from_secs(3600));
            }
        });
        clock.advance(Duration::from_millis(5));
        heard.recv().unwrap();
    }

    #[test]
    #[should_panic(expected = "stuck elsewhere")]
    fn a_thread_that_never_sleeps_at_all_fails_the_test() {
        FakeSteppedClock::for_threads(1).stuck_after(Duration::from_millis(50)).advance(Duration::from_millis(1));
    }

    #[test]
    #[should_panic(expected = "stuck elsewhere")]
    fn a_woken_thread_that_never_sleeps_again_fails_the_test() {
        let clock = FakeSteppedClock::for_threads(1).stuck_after(Duration::from_millis(50));
        let once = clock.clone();
        thread::spawn(move || once.sleep(Duration::from_millis(10)));
        clock.advance(Duration::from_millis(20));
    }

    #[test]
    fn a_thread_more_than_said_is_refused() {
        let clock = FakeSteppedClock::for_threads(1);
        every(&clock, Duration::from_secs(1), "counted", &Heard::default(), &Running::default());
        clock.advance(Duration::ZERO);
        let extra = clock.clone();
        let refused = thread::spawn(move || extra.sleep(Duration::from_secs(1))).join();
        assert!(refused.is_err());
    }
}
