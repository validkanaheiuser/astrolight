use std::time::{Duration, Instant};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::Threading::{
        CreateWaitableTimerExW, SetWaitableTimer, WaitForSingleObject,
        CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, INFINITE, TIMER_ALL_ACCESS,
    },
};
pub struct Clock(HANDLE);
/// Never catch up with back-to-back transactions after a missed deadline.
pub fn next_frame(previous: Instant, finished: Instant, period: Duration) -> Instant {
    (previous + period).max(finished + Duration::from_millis(8))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_burst_after_stall() {
        let t = Instant::now();
        let done = t + Duration::from_millis(150);
        assert_eq!(
            next_frame(t, done, Duration::from_millis(33)),
            done + Duration::from_millis(8)
        );
    }
    #[test]
    fn normal_cadence_retained() {
        let t = Instant::now();
        assert_eq!(
            next_frame(t, t + Duration::from_millis(10), Duration::from_millis(33)),
            t + Duration::from_millis(33)
        );
    }
}
impl Clock {
    pub fn new() -> Self {
        Self(unsafe {
            CreateWaitableTimerExW(
                std::ptr::null(),
                std::ptr::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS,
            )
        })
    }
    pub fn until(&self, deadline: Instant) {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            return;
        };
        self.sleep(left);
    }
    pub fn sleep(&self, d: Duration) {
        if d.is_zero() {
            return;
        }
        if self.0.is_null() {
            std::thread::sleep(d);
            return;
        }
        let due = -((d.as_nanos() / 100).max(1).min(i64::MAX as u128) as i64);
        unsafe {
            if SetWaitableTimer(self.0, &due, 0, None, std::ptr::null(), 0) != 0 {
                WaitForSingleObject(self.0, INFINITE);
            } else {
                std::thread::sleep(d);
            }
        }
    }
}
impl Drop for Clock {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}
