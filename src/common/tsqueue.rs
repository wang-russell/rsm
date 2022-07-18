//对VecDequeue的线程安全性封装
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::collections::vec_deque::{Iter, IterMut};
use std::collections::VecDeque;
use std::sync::{Mutex,Arc};
use std::sync::Condvar;
//use crate::common::errcode;

pub struct TsDequeue<T> {
    inner: Mutex<VecDeque<T>>,
    cond:Condvar,
    has_data:Arc<Mutex<bool>>,
}
//const MAX_BURST_SIZE:usize = 16;
impl<T> TsDequeue<T> {
    pub fn new(capacity:usize) -> Self {
        let queue: VecDeque<T> = VecDeque::with_capacity(capacity);
        return Self {
            inner: Mutex::new(queue),
            cond:Condvar::new(),
            has_data:Arc::new(Mutex::new(false)),
        };
    }

    pub fn push_back(&mut self, v: T) {
         let mut inner = self.inner.lock().unwrap();
         inner.push_back(v);
         //if inner.len()>MAX_BURST_SIZE {
         //   self.notify();
         //}           
    }
    pub fn push_front(&mut self, v: T) {
        let mut inner = self.inner.lock().unwrap();
        inner.push_front(v);
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.inner.lock().unwrap().pop_back()

    }
    pub fn pop_front(&mut self) -> Option<T> {
        self.inner.lock().unwrap().pop_front()
    }

    pub fn iter(&self) -> Iter<T> {
        let l = self.inner.lock().unwrap();
        let inner  = &*l as *const VecDeque<T>;
        return unsafe { (*inner).iter() };

    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        let mut l = self.inner.lock().unwrap();
        let inner  = &mut *l as *mut VecDeque<T>;
        return unsafe { (*inner).iter_mut() };
    }

    pub fn len(&self)->usize {
        return self.inner.lock().unwrap().len();        
    }

    pub fn capacity(&self)->usize {
        self.inner.lock().unwrap().capacity()
    }
    pub fn notify(&self) {
        
        let mut has_data = self.has_data.lock().unwrap();
        *has_data = true;
        self.cond.notify_all();
    }
    pub fn wait(&self) {
        let mut l = self.has_data.lock().unwrap();
        while !(*l) {
            l = self.cond.wait(l).unwrap();
        }
        *l=false;        
    }
}
