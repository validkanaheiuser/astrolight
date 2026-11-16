use std::sync::atomic::{AtomicU64, Ordering};
pub const RELEASE_GRACE_MS: u64 = 180;
// Transient held-key bits and a timestamp only; no text, history, or event queue.
pub struct InputGuard {
    held: [AtomicU64; 256],
    sequence: AtomicU64,
    last: AtomicU64,
    until: AtomicU64,
}
impl InputGuard {
    pub const fn new() -> Self {
        Self {
            held: [const { AtomicU64::new(0) }; 256],
            sequence: AtomicU64::new(0),
            last: AtomicU64::new(0),
            until: AtomicU64::new(0),
        }
    }
    pub fn event(&self, key: u32, down: bool, now: u64) {
        if !(8..256).contains(&key) {
            return;
        }
        let generation = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        self.held[key as usize].store((generation << 1) | u64::from(down), Ordering::Release);
        self.last.store(now, Ordering::Relaxed);
        self.until
            .store(now.saturating_add(RELEASE_GRACE_MS), Ordering::Release);
    }
    pub fn busy(&self, now: u64) -> bool {
        now < self.until.load(Ordering::Acquire)
            || self.held.iter().any(|x| x.load(Ordering::Acquire) & 1 != 0)
    }
    pub fn reconcile(&self, now: u64, mut is_down: impl FnMut(u32) -> bool) {
        // A low-level hook is invoked before Windows updates async key state.
        if now.saturating_sub(self.last.load(Ordering::Relaxed)) < 50 {
            return;
        }
        for (key, state) in self.held.iter().enumerate() {
            let observed = state.load(Ordering::Acquire);
            if observed & 1 != 0 && !is_down(key as u32) {
                let _ = state.compare_exchange(
                    observed,
                    observed & !1,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_and_release_grace() {
        let g = InputGuard::new();
        assert!(!g.busy(0));
        g.event(65, true, 10);
        assert!(g.busy(5000));
        g.event(65, false, 5000);
        assert!(g.busy(5179));
        assert!(!g.busy(5180));
    }
    #[test]
    fn overlap_and_repeat() {
        let g = InputGuard::new();
        g.event(65, true, 10);
        g.event(65, true, 20);
        g.event(66, true, 30);
        g.event(65, false, 40);
        assert!(g.busy(1000));
        g.event(66, false, 1000);
        assert!(!g.busy(1180));
    }
    #[test]
    fn missing_release_reconciles_without_racing_hook() {
        let g = InputGuard::new();
        g.event(65, true, 10);
        g.reconcile(11, |_| false);
        assert!(g.busy(1000));
        g.reconcile(1000, |_| false);
        assert!(!g.busy(1000));
    }
    #[test]
    fn reconciliation_cannot_clear_new_press() {
        let g = InputGuard::new();
        g.event(65, true, 10);
        g.reconcile(100, |key| {
            g.event(key, true, 100);
            false
        });
        assert!(g.busy(1000));
    }
    #[test]
    fn mouse_is_not_typing() {
        let g = InputGuard::new();
        g.event(1, true, 10);
        assert!(!g.busy(10));
    }
}
