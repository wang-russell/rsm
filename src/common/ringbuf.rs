#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

///环形队列实现，添加数据从队尾开始，取数据从队头开始
/// 可以检查ringbuf中是否存在一个完整的应用层报文
use std::{cmp,alloc,mem,ptr};
use super::errcode;

///回调函数，确定目前的ringbuf中是否至少已经有一个完整的报文了，方便进行组包处理;
/// 回调函数第一个参数为当前ringbuf中的前8个字节,如果报文完整，则返回完整的报文长度，否则返回失败
pub type fn_check_msg_interity=fn(start_bytes:&[u8],req_size:usize,total:usize)->Result<usize,errcode::RESULT>;

pub struct ring_buffer_t {
    buffer:*mut u8,
    size:usize,
    head:usize,
    tail:usize,
    fn_call_back:Option<fn_check_msg_interity>,
}


//Head==usize::MAX，the buffer length=0, head=tail, the buffer length=self.capacity
impl ring_buffer_t {
    pub fn new(capacity:usize,call_back:Option<fn_check_msg_interity>)->Option<Self> {
        let p=unsafe { alloc::alloc(alloc::Layout::from_size_align_unchecked(capacity, 1)) };
        if p.is_null() {
            return None
        }
        return Some(Self {
            buffer:p,
            size:capacity,
            head:usize::MAX,
            tail:0,
            fn_call_back:call_back,
        })
    }

    pub fn len(&self)->usize {
        if self.head==usize::MAX {
            return 0
        }
        if self.head==self.tail {
            return self.size
        }
        (self.tail+self.size-self.head) % self.size
    }

    pub fn buffer_available(&self)->usize {
        self.size-self.len()
    }

    //往ringbuf添加一块数据，添加到尾部
    pub fn put_data(&mut self,buf:&[u8])->errcode::RESULT {
        if buf.len()>self.buffer_available() {
            return errcode::ERROR_OUTOF_MEM
        }
        if buf.len()==0 {
            return errcode::ERROR_NO_DATA
        }
        let first_slice_len=cmp::min(buf.len(),self.size-self.tail);
        unsafe {
        if first_slice_len>=buf.len() {
            ptr::copy_nonoverlapping(buf.as_ptr(), self.buffer.offset(self.tail as isize), buf.len());
        } else {
            ptr::copy_nonoverlapping(buf.as_ptr(), self.buffer.offset(self.tail as isize), first_slice_len);
            ptr::copy_nonoverlapping(buf[first_slice_len..].as_ptr(), self.buffer, buf.len()-first_slice_len);
        }
        }
        if self.head==usize::MAX {
            self.head=self.tail;
        }
        self.tail=(self.tail+buf.len()) % self.size;

        errcode::RESULT_SUCCESS
    }

    ///从指定的偏移量处复制长度为copy_len的数据, the caller must checked the buffer and length is valid
    fn copy_data_uncheck(&self,offset:usize,copy_len:usize,buf:&mut [u8]) {
        let index=(self.head+offset) % self.size;
        let first_slice_len = if index>self.tail {
            cmp::min(copy_len,self.size-index)
        } else {
            copy_len
        };
        unsafe {
        if first_slice_len==copy_len {
            ptr::copy_nonoverlapping(self.buffer.offset(index as isize), buf.as_mut_ptr(),copy_len);
        } else {
            ptr::copy_nonoverlapping(self.buffer.offset(index as isize), buf.as_mut_ptr(), first_slice_len);
            ptr::copy_nonoverlapping(self.buffer, buf[first_slice_len..].as_mut_ptr(),copy_len-first_slice_len);
        }
        }       
    }
    //从ringbuf首部开始取一块数据，并从buffer中移除，会回调检查buffer中是否有至少一个报文
    pub fn pull_data(&mut self,max_len:usize,buf:&mut [u8])->Result<usize,errcode::RESULT> {
        if self.len()==0 {
            return Err(errcode::ERROR_NO_DATA)
        }
        if buf.len()==0 {
            return Err(errcode::ERROR_BUFFER_TOO_SMALL)
        }
        let req_len = cmp::min(max_len, buf.len());
        let data_len = match self.fn_call_back {
            None=> {
                cmp::min(req_len, self.len())
            },
            Some(fn_cb)=>{
                let mut hdr=[0u8;8];
                let l=match self.peek_data(0, hdr.len(), &mut hdr) {
                    Ok(l)=>l,
                    Err(e)=>return Err(e),
                };

                if let Ok(len)= fn_cb(&hdr[0..l],req_len,self.len()) {
                    len
                } else {
                    return Err(errcode::ERROR_NO_OP)
                }
            },
        };
        
        self.copy_data_uncheck(0,data_len,buf);
        if self.len()<=data_len {
            self.tail=0;
            self.head=usize::MAX;
        } else {
            self.head=(self.head+data_len) % self.size;
        }        
 
        Ok(data_len)
    }
    //从ringbuf指定偏移量取指定长度的报文,并不从缓冲区中移除数据
    pub fn peek_data(&self,offset:usize,max_len:usize,buf:&mut [u8])->Result<usize,errcode::RESULT> {
        if offset>=self.len() {
            return Err(errcode::ERROR_NO_DATA)
        }

        let copy_len=cmp::min(self.len()-offset,cmp::min(max_len,buf.len()));
        self.copy_data_uncheck(offset,copy_len, buf);

        Ok(copy_len)
    }

}

impl Drop for ring_buffer_t {
    fn drop(&mut self) {
        if self.buffer.is_null() {
            return
        }
        unsafe {
            alloc::dealloc(self.buffer, alloc::Layout::from_size_align_unchecked(self.size, 1));
            self.buffer=ptr::null_mut();
        }
    }
}

use super::spin_lock_t;
pub struct ts_ring_buffer_t {
    lock:spin_lock_t,
    ring:ring_buffer_t,
}

//thread safe wrapper for ring_buffer implementation
impl ts_ring_buffer_t {
    pub fn new(capacity:usize,call_back:Option<fn_check_msg_interity>)->Option<Self> {
        let ring=ring_buffer_t::new(capacity, call_back);
        match ring {
            None=>return None,
            Some(r)=>{
                return Some(Self { lock: spin_lock_t::new(), ring: r })
            }
        }
        
    }

    pub fn len(&self)->usize {
        self.lock.lock();
        let l=self.ring.len();
        self.lock.unlock();
        l
    }

    pub fn buffer_available(&self)->usize {
        self.lock.lock();
        let r=self.ring.buffer_available();
        self.lock.unlock();
        r
    }

    pub fn put_data(&mut self,buf:&[u8])->errcode::RESULT {
        self.lock.lock();
        let r=self.ring.put_data(buf);
        self.lock.unlock();
        r
    }
    pub fn pull_data(&mut self,max_len:usize,buf:&mut [u8])->Result<usize,errcode::RESULT> {
        self.lock.lock();
        let r=self.ring.pull_data(max_len, buf);
        self.lock.unlock();
        r        
    }

    pub fn peek_data(&self,offset:usize,max_len:usize,buf:&mut [u8])->Result<usize,errcode::RESULT> {
        self.lock.lock();
        let r=self.ring.peek_data(offset, max_len, buf);
        self.lock.unlock();
        r    
    }

}
