use std::sync::{Condvar, Mutex};

#[derive(Default)]
pub struct ShareThing<T> {
    data: Mutex<Option<T>>,
    wake: Condvar,
}

impl<T> ShareThing<T> {
    pub fn new() -> Self {
        let data = Mutex::default();
        let wake = Condvar::default();

        Self { data, wake }
    }

    pub fn put(&self, value: T) {
        let mut data = self.data.lock().unwrap();

        *data = Some(value);
        self.wake.notify_one();
    }

    pub fn take(&self) -> T {
        let data = self.data.lock().unwrap();

        let mut data = self.wake.wait_while(data, |data| data.is_none()).unwrap();

        data.take().unwrap()
    }
}
