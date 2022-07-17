#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::sync::atomic::{AtomicBool,Ordering};
use std::hint;

pub struct spin_lock_t {
    locked:AtomicBool,
}
impl spin_lock_t {
    pub fn new()->Self {
        Self{ locked:AtomicBool::new(false)}
    }

    #[inline(always)]
    pub fn lock(&self) {
        while self.locked.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_err() {
            hint::spin_loop();
        } 
    }

    #[inline(always)]
    pub fn unlock(&self) {
        while self.locked.compare_exchange(true, false, Ordering::SeqCst, Ordering::Relaxed).is_err() {
            hint::spin_loop();
        } 
    }
    pub fn value(&self)->bool {
        self.locked.load(Ordering::Acquire)
    }

}

impl Drop for spin_lock_t {
    fn drop(&mut self) {
        if self.locked.load(Ordering::Acquire)==true {
            self.unlock()
        }
       
    }
}