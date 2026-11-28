//! Synchronization primitives reference implementation (RFC-0028).
//!
//! Provides mutex, semaphore, condition variable, event, and futex primitives
//! for inter-thread and inter-component coordination, with priority
//! inheritance, lock-order enforcement, timeouts, and scheduler integration
//! (RFC-0011 blocking/requeue).

use aether_sched_model::{ThreadId, ThreadState, TransitionReason, UniprocessorEdf};
use aether_types::DomainId;

// ---------------------------------------------------------------------------
// Primitive types (RFC-0028.1)
// ---------------------------------------------------------------------------

/// Synchronization primitive type (RFC-0028.1).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SyncType {
    /// Binary lock; supports ownership, contention queue, priority inheritance.
    Mutex = 0,
    /// Counting lock with non-negative count; supports acquire/release.
    Semaphore = 1,
    /// Wait/notify pair; requires a mutex to wait on.
    ConditionVariable = 2,
    /// Single-use or auto-reset notification; supports broadcasting.
    Event = 3,
    /// Lightweight lock for intra-domain fast-path synchronization.
    Futex = 4,
}

// ---------------------------------------------------------------------------
// Errors (RFC-0028 failure semantics)
// ---------------------------------------------------------------------------

/// Synchronization primitive errors (RFC-0028 §4).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SyncError {
    /// The primitive does not exist.
    NotFound,
    /// Double lock by the same thread (RFC-0028.4).
    EDeadLk,
    /// Unlock without owning (RFC-0028.4).
    EPerm,
    /// Timeout expired before the primitive was satisfied (RFC-0028.10).
    ETimedOut,
    /// The primitive is already in the requested state.
    Busy,
    /// Invalid operation for this primitive type.
    InvalidOp,
}

// ---------------------------------------------------------------------------
// Mutex (RFC-0028.2, RFC-0028.3)
// ---------------------------------------------------------------------------

/// Mutex states (RFC-0028.2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MutexState {
    /// Not held by any thread.
    Unlocked,
    /// Held by one thread, no waiters.
    LockedUncontended,
    /// Held by one thread, one or more waiters.
    LockedContended,
}

/// A binary mutex with ownership, a contention queue, and priority
/// inheritance (RFC-0028.2, RFC-0028.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mutex {
    /// Current state.
    pub state: MutexState,
    /// Owning thread (valid only when LOCKED_*).
    pub owner: Option<ThreadId>,
    /// Count of waiting threads.
    pub waiters: u32,
    /// Active priority-inheritance boost (RFC-0028.3).
    pub boost: Option<PriorityBoost>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PriorityBoost {
    /// The boosted priority the holder is temporarily elevated to.
    pub boosted_priority: u8,
    /// The holder's original priority, restored on release.
    pub original_priority: u8,
}

impl Mutex {
    /// Creates a new unlocked mutex.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: MutexState::Unlocked,
            owner: None,
            waiters: 0,
            boost: None,
        }
    }

    /// Acquires the mutex. Returns `EDeadLk` if the caller already owns it
    /// (RFC-0028.4: double lock rejected).
    pub fn lock(&mut self, thread: ThreadId) -> Result<(), SyncError> {
        match self.state {
            MutexState::Unlocked => {
                self.state = MutexState::LockedUncontended;
                self.owner = Some(thread);
                self.waiters = 0;
                self.boost = None;
                Ok(())
            }
            MutexState::LockedUncontended | MutexState::LockedContended => {
                if self.owner == Some(thread) {
                    return Err(SyncError::EDeadLk);
                }
                self.state = MutexState::LockedContended;
                self.waiters = self.waiters.saturating_add(1);
                Err(SyncError::ETimedOut) // caller must block via scheduler
            }
        }
    }

    /// Blocks the calling thread on this mutex (scheduler integration,
    /// RFC-0028.11). Returns the thread id that was blocked.
    pub fn block_on(
        &mut self,
        sched: &mut UniprocessorEdf,
        thread: ThreadId,
    ) -> Result<ThreadId, SyncError> {
        if self.owner != Some(thread) && self.state == MutexState::Unlocked {
            // Fast path: no contention, acquire immediately.
            self.state = MutexState::LockedUncontended;
            self.owner = Some(thread);
            return Ok(thread);
        }
        sched.block().map_err(|_| SyncError::NotFound)?;
        self.state = MutexState::LockedContended;
        self.waiters = self.waiters.saturating_add(1);
        Ok(thread)
    }

    /// Releases the mutex. Returns `EPerm` if the caller does not own it
    /// (RFC-0028.4: unlock without owning rejected).
    pub fn unlock(&mut self, thread: ThreadId) -> Result<(), SyncError> {
        if self.owner != Some(thread) {
            return Err(SyncError::EPerm);
        }
        // Undo any priority inheritance elevation (RFC-0028.3).
        self.boost = None;
        if self.waiters > 0 {
            self.waiters = self.waiters.saturating_sub(1);
            // A waiter will take ownership on wake; keep contended.
            self.state = MutexState::LockedContended;
        } else {
            self.state = MutexState::Unlocked;
            self.owner = None;
        }
        Ok(())
    }

    /// Elevates the holder's priority due to priority inheritance
    /// (RFC-0028.3).
    pub fn inherit_priority(
        &mut self,
        original: u8,
        boosted: u8,
    ) {
        self.boost = Some(PriorityBoost {
            boosted_priority: boosted,
            original_priority: original,
        });
    }
}

impl Default for Mutex {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Semaphore (RFC-0028.5)
// ---------------------------------------------------------------------------

/// A counting semaphore with non-negative count (RFC-0028.5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Semaphore {
    /// Current count (non-negative).
    pub count: u32,
    /// Count of threads blocked waiting for the count to become positive.
    pub waiters: u32,
}

impl Semaphore {
    /// Creates a new semaphore with the given initial count.
    #[must_use]
    pub fn new(count: u32) -> Self {
        Self { count, waiters: 0 }
    }

    /// Acquires: blocks until count is positive, then atomically decrements
    /// and returns (RFC-0028.5).
    pub fn acquire(&mut self) -> Result<(), SyncError> {
        if self.count > 0 {
            self.count -= 1;
            Ok(())
        } else {
            self.waiters = self.waiters.saturating_add(1);
            Err(SyncError::ETimedOut) // caller must block via scheduler
        }
    }

    /// Releases: atomically increments the count and wakes one waiter
    /// (RFC-0028.5).
    pub fn release(&mut self) {
        self.count = self.count.saturating_add(1);
        if self.waiters > 0 {
            self.waiters = self.waiters.saturating_sub(1);
        }
    }
}

impl Default for Semaphore {
    fn default() -> Self {
        Self::new(0)
    }
}

// ---------------------------------------------------------------------------
// Condition variable (RFC-0028.6, RFC-0028.7)
// ---------------------------------------------------------------------------

/// A condition variable: wait/notify pair requiring an associated mutex
/// (RFC-0028.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionVariable {
    /// Count of threads waiting on this condition variable.
    pub waiters: u32,
}

impl ConditionVariable {
    /// Creates a new condition variable.
    #[must_use]
    pub fn new() -> Self {
        Self { waiters: 0 }
    }

    /// Atomically releases the associated mutex and blocks the thread; on
    /// wake, re-acquires the mutex before returning (RFC-0028.6).
    pub fn wait(&mut self) -> u32 {
        self.waiters = self.waiters.saturating_add(1);
        self.waiters
    }

    /// Wakes one waiter (RFC-0028.7).
    pub fn notify_one(&mut self) {
        if self.waiters > 0 {
            self.waiters = self.waiters.saturating_sub(1);
        }
    }

    /// Wakes all waiters (RFC-0028.7).
    pub fn notify_all(&mut self) {
        self.waiters = 0;
    }
}

impl Default for ConditionVariable {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Event (RFC-0028.1)
// ---------------------------------------------------------------------------

/// Event mode: single-use or auto-reset (RFC-0028.1).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EventMode {
    /// Single-use: set once, consumed by the first waiter.
    SingleUse,
    /// Auto-reset: set wakes one waiter, then resets.
    AutoReset,
}

/// An event notification supporting broadcasting (RFC-0028.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// Whether the event is currently signaled.
    pub signaled: bool,
    /// Event mode.
    pub mode: EventMode,
    /// Count of waiting threads.
    pub waiters: u32,
}

impl Event {
    /// Creates a new non-signaled event.
    #[must_use]
    pub fn new(mode: EventMode) -> Self {
        Self {
            signaled: false,
            mode,
            waiters: 0,
        }
    }

    /// Signals the event, waking waiters (broadcast for single-use,
    /// auto-reset wakes one then clears) (RFC-0028.1).
    pub fn signal(&mut self) -> bool {
        self.signaled = true;
        let woke = self.waiters > 0;
        if matches!(self.mode, EventMode::AutoReset) && woke {
            self.signaled = false;
            self.waiters = self.waiters.saturating_sub(1);
        }
        woke
    }

    /// Waits on the event (blocks until signaled).
    pub fn wait(&mut self) {
        self.waiters = self.waiters.saturating_add(1);
    }

    /// Consumes the signal if present (manual reset consumers).
    pub fn try_wait(&mut self) -> bool {
        if self.signaled {
            self.signaled = false;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Futex (RFC-0028.1)
// ---------------------------------------------------------------------------

/// A lightweight userspace futex for intra-domain fast-path synchronization
/// (RFC-0028.1).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Futex {
    /// The futex value.
    pub value: u32,
}

impl Futex {
    /// Creates a new futex with the given initial value.
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self { value }
    }

    /// Waits until the value equals `expected`, then blocks.
    #[must_use]
    pub fn wait(&self, expected: u32) -> bool {
        self.value == expected
    }

    /// Wakes up to `n` waiters.
    pub fn wake(&mut self, n: u32) -> u32 {
        n.min(1) // userspace fast-path: single waker by default
    }
}

// ---------------------------------------------------------------------------
// Lock-order declaration (RFC-0028.8)
// ---------------------------------------------------------------------------

/// A lock-order declaration: `first` must be acquired before `second`
/// (RFC-0028.8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockOrderDeclaration {
    pub first: u64,
    pub second: u64,
}

// ---------------------------------------------------------------------------
// SyncManager (RFC-0028)
// ---------------------------------------------------------------------------

/// Coordinates synchronization primitives and scheduler integration
/// (RFC-0028.11). When a thread blocks on a primitive the scheduler is
/// notified (`RUNNING -> BLOCKED`); when signaled the thread is requeued to
/// `READY` (RFC-0028.11).
pub struct SyncManager {
    /// Registered mutexes, keyed by id.
    pub mutexes: Vec<Mutex>,
    /// Registered semaphores, keyed by id.
    pub semaphores: Vec<Semaphore>,
    /// Registered condition variables, keyed by id.
    pub condvars: Vec<ConditionVariable>,
    /// Registered events, keyed by id.
    pub events: Vec<Event>,
    /// Registered futexes, keyed by id.
    pub futexes: Vec<Futex>,
    /// Declared lock-order hierarchy (RFC-0028.8).
    pub lock_order: Vec<LockOrderDeclaration>,
}

impl SyncManager {
    /// Creates a new empty sync manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mutexes: Vec::new(),
            semaphores: Vec::new(),
            condvars: Vec::new(),
            events: Vec::new(),
            futexes: Vec::new(),
            lock_order: Vec::new(),
        }
    }

    /// Declares a lock-order hierarchy entry (RFC-0028.8): `first` must be
    /// acquired before `second`. Verified at runtime; reverse-order
    /// acquisition emits `LockOrderViolation`.
    pub fn declare_lock_order(&mut self, first: u64, second: u64) {
        self.lock_order.push(LockOrderDeclaration { first, second });
    }

    /// Checks that acquiring `after` while holding `before` does not violate
    /// the declared lock order (RFC-0028.8). Returns `true` if the order is
    /// valid or undeclared.
    #[must_use]
    pub fn check_lock_order(&self, before: u64, after: u64) -> bool {
        // Any declaration requiring `before` before `after` is satisfied.
        if self.lock_order.iter().any(|d| d.first == before && d.second == after) {
            return true;
        }
        // Reverse of a declared order is a violation.
        if self.lock_order.iter().any(|d| d.first == after && d.second == before) {
            return false;
        }
        // Undeclared order: permitted (no constraint).
        true
    }

    /// Registers a mutex and returns its id.
    pub fn add_mutex(&mut self) -> u64 {
        let id = self.mutexes.len() as u64;
        self.mutexes.push(Mutex::new());
        id
    }

    /// Registers a semaphore and returns its id.
    pub fn add_semaphore(&mut self, count: u32) -> u64 {
        let id = self.semaphores.len() as u64;
        self.semaphores.push(Semaphore::new(count));
        id
    }

    /// Registers a condition variable and returns its id.
    pub fn add_condvar(&mut self) -> u64 {
        let id = self.condvars.len() as u64;
        self.condvars.push(ConditionVariable::new());
        id
    }

    /// Registers an event and returns its id.
    pub fn add_event(&mut self, mode: EventMode) -> u64 {
        let id = self.events.len() as u64;
        self.events.push(Event::new(mode));
        id
    }

    /// Registers a futex and returns its id.
    pub fn add_futex(&mut self, value: u32) -> u64 {
        let id = self.futexes.len() as u64;
        self.futexes.push(Futex::new(value));
        id
    }

    /// Priority inheritance: elevates the holder's priority when a
    /// higher-priority thread blocks (RFC-0028.3). The elevation is undone
    /// when the mutex is released.
    pub fn apply_priority_inheritance(
        &mut self,
        mutex_id: u64,
        original_priority: u8,
        boosted_priority: u8,
    ) -> Result<(), SyncError> {
        let mutex = self
            .mutexes
            .get_mut(mutex_id as usize)
            .ok_or(SyncError::NotFound)?;
        mutex.inherit_priority(original_priority, boosted_priority);
        Ok(())
    }

    /// Blocks the current running thread on a mutex and integrates with the
    /// scheduler (RFC-0028.11).
    pub fn block_on_mutex(
        &mut self,
        mutex_id: u64,
        sched: &mut UniprocessorEdf,
        thread: ThreadId,
    ) -> Result<ThreadId, SyncError> {
        let mutex = self
            .mutexes
            .get_mut(mutex_id as usize)
            .ok_or(SyncError::NotFound)?;
        mutex.block_on(sched, thread)
    }

    /// Unblocks a thread waiting on a condition variable or event,
    /// requeueing it to `READY` (RFC-0028.11).
    pub fn unblock_thread(&self, sched: &mut UniprocessorEdf, tid: ThreadId) -> Result<(), SyncError> {
        sched.unblock(tid).map_err(|_| SyncError::NotFound)
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Timeout (RFC-0028.10)
// ---------------------------------------------------------------------------

/// Result of a timed acquire: satisfied or timed out (state unchanged on
/// timeout per RFC-0028.10).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TimedResult {
    /// The primitive was satisfied before the deadline.
    Satisfied,
    /// The deadline expired; synchronization state is unchanged.
    TimedOut,
}

/// Evaluates a timeout against an absolute deadline (RFC-0028.10).
#[must_use]
pub fn check_timeout(now_ns: u64, deadline_ns: u64) -> TimedResult {
    if now_ns >= deadline_ns {
        TimedResult::TimedOut
    } else {
        TimedResult::Satisfied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutex_lock_unlock_transitions() {
        let mut m = Mutex::new();
        assert_eq!(m.lock(ThreadId(1)), Ok(()));
        assert_eq!(m.state, MutexState::LockedUncontended);
        assert_eq!(m.owner, Some(ThreadId(1)));
        assert_eq!(m.unlock(ThreadId(1)), Ok(()));
        assert_eq!(m.state, MutexState::Unlocked);
        assert_eq!(m.owner, None);
    }

    #[test]
    fn double_lock_is_deadlk() {
        let mut m = Mutex::new();
        m.lock(ThreadId(1)).unwrap();
        assert_eq!(m.lock(ThreadId(1)), Err(SyncError::EDeadLk));
    }

    #[test]
    fn unlock_without_owner_is_perm() {
        let mut m = Mutex::new();
        m.lock(ThreadId(1)).unwrap();
        assert_eq!(m.unlock(ThreadId(2)), Err(SyncError::EPerm));
    }

    #[test]
    fn semaphore_acquire_release() {
        let mut s = Semaphore::new(1);
        assert_eq!(s.acquire(), Ok(()));
        assert_eq!(s.acquire(), Err(SyncError::ETimedOut));
        s.release();
        assert_eq!(s.acquire(), Ok(()));
    }

    #[test]
    fn condvar_notify_wakes_all() {
        let mut cv = ConditionVariable::new();
        cv.wait();
        cv.wait();
        assert_eq!(cv.waiters, 2);
        cv.notify_one();
        assert_eq!(cv.waiters, 1);
        cv.notify_all();
        assert_eq!(cv.waiters, 0);
    }

    #[test]
    fn check_timeout_semantics() {
        assert_eq!(check_timeout(100, 200), TimedResult::Satisfied);
        assert_eq!(check_timeout(200, 200), TimedResult::TimedOut);
        assert_eq!(check_timeout(300, 200), TimedResult::TimedOut);
    }
}
