#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use crate::common::{tsidallocator::TsIdAllocator,errcode,spin_lock::spin_lock_t};
use std::fmt;
use std::alloc;
use super::*;

pub mod PktBufType {
    pub const BufTypeRecv:u16 = 0;
    pub const BufTypeSend:u16 = 1;
}
///pkt_buf_handle_t, packet Buffer Handle
#[derive(Debug)]
pub struct pkt_buf_handle_t {
    buf_id:i32,
    buf_capacity:u16,
    buf_type:u16,
    buf_len:u16,
    buf_ptr:*mut u8,
}
impl pkt_buf_handle_t {
    pub fn new_zero()->pkt_buf_handle_t {
        return pkt_buf_handle_t {
            buf_id:TsIdAllocator::INVALID_ID,
            buf_capacity:0,
            buf_type:0,
            buf_len:0,
            buf_ptr:std::ptr::null_mut(),
        }
    }
    pub fn len(&self)->usize {
        return self.buf_len as usize;
    }
    pub fn capacity(&self)->usize {
        return self.buf_capacity as usize;
    }

    pub fn as_mut_ptr(&mut self)->*mut u8 {
        return self.buf_ptr
    }
    pub fn as_ptr(&self)->*const u8 {
        return self.buf_ptr
    }
    pub fn as_slice(&self)->&[u8] {
        unsafe {
        return & (*(self.buf_ptr as *const [u8;MAX_PKT_BUF_SIZE as usize]))[0..self.buf_len as usize]
        }
    }
    pub fn as_mut_slice(&mut self)->&mut [u8] {
        unsafe {
        return &mut (*(self.buf_ptr as *mut [u8;MAX_PKT_BUF_SIZE as usize]))[0..self.buf_len as usize]
        }
    }
    pub fn set_len(&mut self,len:usize) {
        self.buf_len = std::cmp::min(len as u16,self.buf_capacity)
    }

    pub fn extend_from_slice(&mut self,buf:&[u8]) {
        if self.buf_len+buf.len() as u16 >self.buf_capacity {
            return;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(buf.as_ptr(), (self.buf_ptr as usize+self.buf_len as usize) as *mut u8, buf.len());
        }
        self.buf_len+=buf.len() as u16;
    }
    pub fn is_valid_buffer(&self)->bool {
        return self.buf_id!=TsIdAllocator::INVALID_ID && self.buf_capacity>0
    }
}
impl fmt::Display for pkt_buf_handle_t {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"id:{},capacity:{},len={},ptr:{:?}",self.buf_id,self.buf_capacity,self.buf_len,self.buf_ptr)
    }
}

impl Clone for pkt_buf_handle_t {
    fn clone(&self)->Self {
        let mut new_pkt = allocate_pkt_buf(self.buf_type);
        if !new_pkt.is_valid_buffer() {
            return Self::new_zero()
        }
        new_pkt.buf_len = self.buf_len;
        unsafe {
            std::ptr::copy_nonoverlapping(self.buf_ptr, new_pkt.buf_ptr, self.buf_len as usize);
        }

        return new_pkt;
    }
}

impl Drop for pkt_buf_handle_t {
    fn drop(&mut self) {
        if self.buf_id==TsIdAllocator::INVALID_ID {
            return
        }
        let res = free_pkt_buf(self.buf_type,self.buf_id as usize);
        assert_eq!(res,errcode::RESULT_SUCCESS);
    }
}
const MAX_PKT_BUF_COUNT:usize = 65536;

///pkt_global_cb_t：全局packet buffer分配控制块
struct pkt_global_cb_t {
    ids:TsIdAllocator,
    packet_bufs:*mut [*mut u8;MAX_PKT_BUF_COUNT],
    buf_used:[bool;MAX_PKT_BUF_COUNT],
    lock:spin_lock_t,
    buf_type:u16,
    capacity:usize,
    alloc_count:u64,
    free_count:u64,
}

impl pkt_global_cb_t {
    //分配一个Buffer
    pub fn allocate_pkt_buf(&mut self)->pkt_buf_handle_t {
        self.lock.lock();        
        let id =self.ids.allocate_id();
        
        if id==TsIdAllocator::INVALID_ID {
            self.lock.unlock();
            return pkt_buf_handle_t::new_zero()
        }
        let idx = (id-1) as usize;
        self.buf_used[idx]=true;
        self.alloc_count+=1;
        self.lock.unlock();
        let handle = pkt_buf_handle_t {
            buf_id:id,
            buf_capacity:MAX_PKT_BUF_SIZE,
            buf_type:self.buf_type,
            buf_len:0,
            buf_ptr:unsafe { (&*self.packet_bufs)[idx] },
        };
        
        
        return handle
    }

    ///释放Buf
    fn free_pkt_buf(&mut self,buf_id:usize)->errcode::RESULT {
        let idx = buf_id-1;
        self.lock.lock();
        if self.ids.release_id(buf_id as i32) != errcode::RESULT_SUCCESS {            
            assert!(buf_id>0 && buf_id<self.capacity);
            self.lock.unlock();
            return errcode::ERROR_NOT_FOUND;
        }
        
        self.buf_used[idx]=false;
        self.free_count+=1;
        self.lock.unlock();
        errcode::RESULT_SUCCESS
    }
}
static mut GlobalRecvPktBufs:Option<pkt_global_cb_t>=None;
static mut GlobalSendPktBufs:Option<pkt_global_cb_t>=None;

fn get_one_buf_cb(buf_count:usize,buf_type:u16)->Option<pkt_global_cb_t> {
    let capacity = std::cmp::min(buf_count, MAX_PKT_BUF_COUNT);

    let buf_ptr = unsafe { 
        alloc::alloc_zeroed(alloc::Layout::from_size_align_unchecked(MAX_PKT_BUF_SIZE as usize *capacity, 1))
    };

    if buf_ptr==std::ptr::null_mut() {
        return None;
    }
    let blocks = unsafe {
        alloc::alloc_zeroed(alloc::Layout::from_size_align_unchecked(std::mem::size_of::<* mut u8>()*capacity, 1)) as *mut [*mut u8;MAX_PKT_BUF_COUNT]
    };
    for i in 0..capacity {
        unsafe {
            (&mut *blocks)[i]=(buf_ptr as usize+i*MAX_PKT_BUF_SIZE as usize) as *mut u8;
        }
    }
    let buf_cb = pkt_global_cb_t {
        ids:TsIdAllocator::new(1,capacity as i32),
        packet_bufs:blocks,
        buf_used:[false;MAX_PKT_BUF_COUNT],
        lock:spin_lock_t::new(),
        buf_type:buf_type,
        capacity:capacity,
        alloc_count:0,
        free_count:0,
    };
    return Some(buf_cb);
}

///外部接口，初始化Packet Buffer
pub fn init_pkt_buf(buf_count:usize)->errcode::RESULT {

    let buf_cb1= match get_one_buf_cb(buf_count,PktBufType::BufTypeRecv) {
        Some(cb)=>cb,
        None=>return errcode::ERROR_OUTOF_MEM,
    };
    let buf_cb2=match get_one_buf_cb(buf_count,PktBufType::BufTypeSend) {
        Some(cb)=>cb,
        None=>return errcode::ERROR_OUTOF_MEM,
    };
    unsafe {        
        GlobalRecvPktBufs=Some(buf_cb1);
        GlobalSendPktBufs=Some(buf_cb2);
    }
    errcode::RESULT_SUCCESS
}

fn get_gbuf_ref(buf_type:u16)->Option<&'static mut pkt_global_cb_t> {
    let cb_ptr = match buf_type {
        PktBufType::BufTypeRecv=> {
            match  unsafe { &mut GlobalRecvPktBufs } {
                None=>return None,
                Some(cb)=>cb,
            }
        },
        PktBufType::BufTypeSend=> {
            match unsafe { &mut GlobalSendPktBufs } {
                None=>return None,
                Some(cb)=>cb,
            }
        },
        _=>return None,
    };

    return Some(cb_ptr);
}
///allocate_pkt_buf 申请一个pkt_buf，尺寸固定,buf_type参见PktBufType常量
pub fn allocate_pkt_buf(buf_type:u16)->pkt_buf_handle_t {
    let gbuf = match get_gbuf_ref(buf_type) {
        None=>return pkt_buf_handle_t::new_zero(),
        Some(bufs)=>bufs,
    };

    return gbuf.allocate_pkt_buf()
}

///free_pkt_buf 释放一个pkt_buf，尺寸固定
fn free_pkt_buf(buf_type:u16,buf_id:usize)->errcode::RESULT {
    let gbuf = match get_gbuf_ref(buf_type) {
        None=> {
            assert!(false);
            return errcode::ERROR_NOT_INITIALIZED;
        },
        Some(bufs)=>bufs,
    };  
    return gbuf.free_pkt_buf(buf_id);    
}

pub fn get_dbg_string()->String {
    let gbuf = match get_gbuf_ref(PktBufType::BufTypeRecv) {
        None=>return String::new(),
        Some(bufs)=>bufs,
    };
    let recv_str = format!("Global Recv Packet Buffer, Capacity={},used={},alloc_call={},free_call={}",
        gbuf.ids.capacity(),gbuf.ids.used_count(),gbuf.alloc_count,gbuf.free_count);

    let sbuf = match get_gbuf_ref(PktBufType::BufTypeSend) {
        None=> return String::new(),
        Some(bufs)=>bufs,
    };

    format!("{},Global Send Packet Buffer, Capacity={},used={},alloc_call={},free_call={}",
        recv_str,sbuf.ids.capacity(),sbuf.ids.used_count(),sbuf.alloc_count,sbuf.free_count)

}
pub fn print_stats() {
    println!("{}",get_dbg_string());
}