use std::sync::{Mutex, Arc};
use std::iter::Iterator;
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Clone)]
pub struct SharedDeque<T: Clone + Send + Sync> {
    inner: Arc<Mutex<VecDeque<T>>>,
    capacity: usize,
}

impl<T: Clone + Send + Sync> SharedDeque<T> {

    pub fn new(capacity: usize) -> Self {

        let deque = VecDeque::with_capacity(capacity);
        Self {
            inner: Arc::new(Mutex::new(deque)),
            capacity,
        }
    }

    /// This will remove all the elements from inner and return
    /// them in another vector.
    pub fn drain(&mut self) -> Vec<T> {
        let mut inner = self.inner.lock().unwrap();
        let mut receiver = Vec::with_capacity(inner.len());
        let drain = inner.drain(..);

        for el in drain {
            receiver.push(el);
        }

        receiver
    }

    pub fn push(&mut self, data: T) {
        let mut inner = self.inner.lock().unwrap();
        if inner.len() == self.capacity {
            // Drop the oldest !
            let _ = inner.pop_front();
            // TODO debug statement here
        }

        inner.push_back(data);
    }

    pub fn push_all(&mut self, v: Vec<T>) {
        let mut inner = self.inner.lock().unwrap();
        for el in v {
            if inner.len() == self.capacity {
                let _ = inner.pop_front();
            }

            inner.push_back(el);
        }
    }
}
