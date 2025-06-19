use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub struct ThreadSafeRef<T>(Arc<Mutex<T>>);

impl<T> ThreadSafeRef<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl<T> From<ThreadSafeRef<T>> for Arc<Mutex<T>> {
    fn from(thread_safe_ref: ThreadSafeRef<T>) -> Self {
        thread_safe_ref.0
    }
}

impl<T> Clone for ThreadSafeRef<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
pub struct ThreadSafeRwRef<T>(Arc<RwLock<T>>);

impl<T> ThreadSafeRwRef<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        self.0
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn write(&self) -> RwLockWriteGuard<T> {
        self.0
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl<T> From<ThreadSafeRwRef<T>> for Arc<RwLock<T>> {
    fn from(thread_safe_rw_ref: ThreadSafeRwRef<T>) -> Self {
        thread_safe_rw_ref.0
    }
}

impl<T> Clone for ThreadSafeRwRef<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
