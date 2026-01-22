use lockfree::stack::Stack as LockfreeStack;
use parking_lot::Mutex;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};

pub struct Stack<T> {
    stack: LockfreeStack<T>,
    waker: Mutex<Option<Waker>>,
    notified: AtomicBool,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self {
            stack: LockfreeStack::new(),
            waker: Mutex::new(None),
            notified: AtomicBool::new(false),
        }
    }

    /// MPSC push — lock-free, fast
    pub fn push(&self, value: T) {
        self.stack.push(value);
        self.notified.store(true, Ordering::Release);

        if let Some(w) = self.waker.lock().take() {
            w.wake();
        }
    }

    /// Async pop — waits without spinning
    pub async fn async_pop(self: &Arc<Self>) -> T {
        PopFuture {
            inner: Arc::clone(self),
        }
        .await
    }

    pub fn pop(&self) -> Option<T> {
        self.stack.pop()
    }
}

struct PopFuture<T> {
    inner: Arc<Stack<T>>,
}

impl<T> Future for PopFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        // Fast path: try pop immediately
        if let Some(v) = self.inner.stack.pop() {
            return Poll::Ready(v);
        }

        // Register waker
        {
            let mut w = self.inner.waker.lock();
            *w = Some(cx.waker().clone());
        }

        // Check again to avoid lost wakeups
        if let Some(v) = self.inner.stack.pop() {
            return Poll::Ready(v);
        }

        Poll::Pending
    }
}
