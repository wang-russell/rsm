
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
// thread safe Dequeue
use std::sync::{Mutex,Arc};
use std::sync::Condvar;
use crate::common::{spin_lock::spin_lock_t,errcode};
use std::alloc::{self,Layout};
use std::{mem,ptr};
use std::iter::{Iterator};

const INVALID_INDEX:usize = usize::MAX; 
pub struct AtomicDequeue<T> {
    inner: *mut T,
    length:usize,
    limit:usize,
    head:usize,
    tail:usize,
    locked:spin_lock_t,
    cond:Condvar,
    has_data:Arc<Mutex<bool>>,
}
//const MAX_BURST_SIZE:usize = 16;
impl<T> AtomicDequeue<T> {
    pub fn new(capacity:usize) -> Self {
        use crate::alg;
        let new_cap = 1 << (alg::log2(capacity as u64) + 1);   
        let pdata = unsafe { alloc::alloc(Layout::from_size_align_unchecked(new_cap*mem::size_of::<T>(), 1)) as *mut T};
        return Self {
            inner: pdata,
            length:0,
            limit:new_cap,
            head:INVALID_INDEX,
            tail:INVALID_INDEX,
            locked:spin_lock_t::new(),
            cond:Condvar::new(),
            has_data:Arc::new(Mutex::new(false)),
        };
    }
    pub fn push_back(&mut self, v: T)->errcode::RESULT {
         self.locked.lock();
         let res = if self.length<self.limit {
            self.get_next_tail_index();
            self.buffer_write(self.tail, v);
            self.length+=1;
            errcode::RESULT_SUCCESS   
         } else {
            errcode::ERROR_OUTOF_MEM
         };
         
         self.locked.unlock();
         return res
    }
    pub fn push_front(&mut self, v: T)->errcode::RESULT {
        self.locked.lock();
        let res = if self.length<self.limit {
            self.get_prev_head_index();
            self.buffer_write(self.head, v);
            self.length+=1;
            errcode::RESULT_SUCCESS 
         } else {
            errcode::ERROR_OUTOF_MEM
         };
        self.locked.unlock();
        return res
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.locked.lock();
        if self.length>0 {            
            let data = self.buffer_read(self.tail);
            self.length-=1;
            self.get_prev_tail_index();
            self.locked.unlock();
            return Some(data);
                       
         } else {
            self.locked.unlock();
            return None
         }
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.locked.lock();
        if self.length>0 {            
            let data = self.buffer_read(self.head);
            self.length-=1;
            self.get_next_head_index();
            self.locked.unlock();
            return Some(data);
                       
         } else {
            self.locked.unlock();
            return None
         }
    }

    #[inline(always)]
    fn buffer_read(&mut self, off: usize) -> T {
        unsafe { ptr::read(self.inner.add(off)) }
    }

    /// Writes an element into the buffer, moving it.
    #[inline(always)]
    fn buffer_write(&mut self, off: usize, value: T) {
        unsafe {
            ptr::write(self.inner.add(off), value);
        }
    }

    #[inline(always)]
    fn get_next_tail_index(&mut self)->usize {
        if self.tail==INVALID_INDEX {
            self.tail=0;
            self.head=0;
            return 0;
        }
        self.tail  = (self.tail + 1) & !self.limit;
        self.tail
    }
    #[inline(always)]
    fn get_next_head_index(&mut self)->usize {
        if self.head==INVALID_INDEX {
            self.tail=0;
            self.head=0;
            return 0;
        }
        self.head  = (self.head + 1) & !self.limit;
        self.head
    }
    #[inline(always)]
    fn get_prev_tail_index(&mut self)->usize {
        if self.tail==INVALID_INDEX {
            self.tail=0;
            self.head=0;
            return 0;
        }        
        if self.tail==0 {
            self.tail = self.limit-1;
        } else {
            self.tail-=1;
        }
        self.tail

    }
    #[inline(always)]
    fn get_prev_head_index(&mut self)->usize {
        if self.tail==INVALID_INDEX {
            self.tail=0;
            self.head=0;
            return 0;
        }          
        if self.head==0 {
            self.head = self.limit-1;
        } else {
            self.head-=1;
        }
        self.head
    }


    ///iter()使用自动加锁，使用后必须要进行手工end_iter()
    pub fn iter(&self) -> Iter<T> {
        self.locked.lock();
        return Iter{cur_idx:0,inner:self}
    }

    pub fn end_iter(&self) {
        self.locked.unlock();
    }
   ///iter_mut()使用自动加锁，使用后必须要进行手工end_iter()
    pub fn iter_mut(&mut self) -> Iter<T> {
        self.locked.lock();
        return Iter{cur_idx:0,inner:self}
    }

    fn is_index_valid(&self,index:usize)->bool {
        if self.head < self.tail {
            return index>=self.head && index<=self.tail;
        }
        return index<=self.head || (index>=self.tail && index <self.limit)
    }

    pub fn len(&self)->usize {
        return self.length;
    }

    pub fn capacity(&self)->usize {
        self.limit
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

impl<T> Drop for AtomicDequeue<T> {
    fn drop(&mut self) {
        let pdata = self.inner as *mut u8;
        if pdata!=std::ptr::null_mut() {
            unsafe {
                alloc::dealloc(pdata, Layout::from_size_align_unchecked(self.limit*mem::size_of::<T>(), 1));
            }
            
        }
    }
}

pub struct Iter<'a,T> {
    cur_idx:usize,
    inner:&'a AtomicDequeue<T>,
}
impl <'a,T> Iterator for Iter<'a,T> {
    type Item = &'a T;
    fn next(&mut self)->Option<Self::Item> {
        if self.inner.length==0 {
            return None;
        }
        if self.cur_idx==INVALID_INDEX {
            self.cur_idx=0;
        } else {
            self.cur_idx=(self.cur_idx+1) % self.inner.limit;
        }
        if !self.inner.is_index_valid(self.cur_idx) {
            return None;
        }
        let p = unsafe { &mut *((self.inner.inner as usize+self.cur_idx*mem::size_of::<T>()) as *mut T) };
        
        return Some(p);
    }

}
