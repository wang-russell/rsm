#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use crate::common::errcode;
use std::collections::{VecDeque};
use crate::common::{spin_lock::spin_lock_t,bitmap::bitmap_t};

pub struct TsIdAllocator {
    start: i32,
    cap: i32,
    allocator: VecDeque<i32>,
    used: bitmap_t,
    lock: spin_lock_t,
}

impl TsIdAllocator {
    pub const INVALID_ID: i32 = -1;

    pub fn new(start: i32, capacity: i32) -> Self {
        let mut allocator = TsIdAllocator {
            start: start,
            cap: capacity,
            allocator: VecDeque::with_capacity(capacity as usize),
            used: bitmap_t::new(capacity),
            lock:  spin_lock_t::new(),
        };
        allocator.init();
        allocator
    }
    ///TsAllocator初始化
    fn init(&mut self) {
        for i in self.start..self.start + self.cap {
            self.allocator.push_back(i);
        }
    }
    pub fn allocate_id(&mut self) -> i32 {
        self.lock.lock();
        let id = match self.allocator.pop_front() {
                None =>  Self::INVALID_ID,
                Some(i) => {
                    let idx = i - self.start;
                    self.used.set_bitmap(idx);
                    i
                },
            };
        self.lock.unlock();
        return id
    }
    ///释放一个申请的ID
    pub fn release_id(&mut self, id: i32) -> errcode::RESULT {
        self.lock.lock();
        let idx = id - self.start;
        let res = match self.used.is_bit_set(idx) {
            false => errcode::ERROR_NOT_FOUND,
            true => {
                self.allocator.push_front(id);
                self.used.unset_bitmap(idx);
                errcode::RESULT_SUCCESS
            },
        };

        self.lock.unlock();
        return res

    }

    pub fn capacity(&self) -> i32 {
        self.cap
    }

    pub fn used_count(&self) -> i32 {
        self.lock.lock();
        let len = self.used.get_used_count() as i32;
        self.lock.unlock();
        return len;
    }
}
