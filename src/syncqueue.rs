use std::collections::VecDeque;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Condvar;
use std::sync::Mutex;

const ERR_QUEUE_CLOSED: &'static str = "queue is closed";

#[derive(Debug)]
pub struct SyncQueue<T> {
    data: Mutex<VecDeque<T>>,
    closed: AtomicBool,
    cv: Condvar,
}

impl<T> SyncQueue<T> {
    /// Creates a new, empty queue
    pub fn new() -> Self {
        Self {
            data: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
            closed: AtomicBool::new(false),
        }
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }

    pub fn push(&self, value: T) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut data = self.data.lock().unwrap();
        if self.closed.load(Ordering::Relaxed) {
            return Err(ERR_QUEUE_CLOSED.into());
        }

        data.push_back(value);
        self.cv.notify_one();
        return Ok(());
    }

    pub fn pop(&self) -> Result<T, Box<dyn Error + Send + Sync>> {
        let mut data = self.data.lock().unwrap();
        let mut closed = self.closed.load(Ordering::Relaxed);
        while data.is_empty() && !closed {
            data = self.cv.wait(data).unwrap();
            closed = self.closed.load(Ordering::Relaxed);
        }

        if closed {
            return Err(ERR_QUEUE_CLOSED.into());
        }

        return Ok(data.pop_front().unwrap());
    }

    pub fn len(&self) -> usize {
        let data = self.data.lock().unwrap();
        data.len()
    }

    pub fn is_empty(&self) -> bool {
        let data = self.data.lock().unwrap();
        data.is_empty()
    }

    pub fn drain(&self) -> VecDeque<T> {
        let mut data = self.data.lock().unwrap();
        return data.drain(..).collect::<VecDeque<T>>();
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::SyncQueue;
    use crate::syncqueue::ERR_QUEUE_CLOSED;

    #[test]
    fn test_basic() {
        let queue: SyncQueue<i32> = SyncQueue::new();
        let pushed = queue.push(1);
        assert!(!pushed.is_err());
        for n in 2..101 {
            let pushed2 = queue.push(n);
            assert!(!pushed2.is_err());
        }
        assert_eq!(queue.len(), 100);
        let popped = queue.pop();
        assert!(!popped.is_err());
        assert!(popped.is_ok());
        for n in 2..101 {
            let popped2 = queue.pop();
            assert!(!popped2.is_err());
            assert_eq!(popped2.unwrap(), n);
        }
        queue.close();
        let popped2 = queue.pop();
        assert!(popped2.is_err());
    }

    #[test]
    fn test_thread_safety() {
        let queue = Arc::new(SyncQueue::<i32>::new());
        let q1 = queue.clone();
        let t1 = thread::spawn(move || {
            let pushed = q1.push(1);
            assert!(!pushed.is_err());
            let pushed2 = q1.push(2);
            assert!(!pushed2.is_err());
        });

        let q2 = queue.clone();
        let t2 = thread::spawn(move || {
            let pushed = q2.push(3);
            assert!(!pushed.is_err());
            let pushed2 = q2.push(4);
            assert!(!pushed2.is_err());
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert_eq!(queue.len(), 4);
    }

    fn consume(queue: Arc<SyncQueue<i32>>, max_value: i32) {
        loop {
            let popped = queue.pop();
            match popped {
                Ok(d) => {
                    assert!(d <= max_value)
                }
                Err(msg) => {
                    assert_eq!(format!("{}", msg), ERR_QUEUE_CLOSED);
                    queue.drain();
                    break;
                }
            }
        }
    }

    #[test]
    fn test_concurrent_push_pop() {
        let max_value = 101;
        // 1 producer and 2 consumers
        let queue = Arc::new(SyncQueue::<i32>::new());

        // producer
        let q1 = queue.clone();
        let t1 = thread::spawn(move || {
            for n in 1..(max_value + 1) {
                let pushed = q1.push(n);
                assert!(!pushed.is_err());
            }
            q1.close();
        });

        let q2 = queue.clone();
        let t2 = thread::spawn(move || consume(q2, max_value));

        let q3 = queue.clone();
        let t3 = thread::spawn(move || consume(q3, max_value));

        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();

        assert!(queue.is_empty())
    }
}
