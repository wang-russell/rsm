#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
use super::*;
use std::alloc;
use errcode;

pub struct bitmap_t {
    bit_string:*mut u64,
    capacity:usize,
    used_count:usize,
    u64_count:usize,
}

impl bitmap_t {
    pub fn new(bits:i32)->Self {
        let u64_num=ceiling(bits as u64, (std::mem::size_of::<u64>()*8) as u64);
        let mut bitmap = Self {
            bit_string:std::ptr::null_mut(),
            capacity:bits as usize,
            used_count:0,
            u64_count:u64_num as usize,
        };
        unsafe {
        bitmap.bit_string = alloc::alloc_zeroed(alloc::Layout::from_size_align_unchecked(bitmap.u64_count*std::mem::size_of::<u64>(),1)) as *mut u64;
        }
        return bitmap;
    }

    ///给定位域索引值，返回指定的u64指针以及bit位域
    pub fn get_u64_by_index(&mut self,idx:usize)->(&mut u64, u64) {
        let u64_idx= idx as usize / (8*std::mem::size_of::<u64>());
        let u64_ptr = unsafe {
             &mut *((self.bit_string as usize + u64_idx*std::mem::size_of::<u64>()) as *mut u64)
        };
        let bit_offset = idx as usize % (std::mem::size_of::<u64>()*8);
        let test_bits = 1u64<< bit_offset;

        return (u64_ptr,test_bits)
    }

    ///设置一个bitmap位，从0开始
    pub fn set_bitmap(&mut self,idx:i32)->errcode::RESULT {
        if idx as usize>=self.capacity {
            return errcode::ERROR_OUTOF_MEM;
        }
        let (u64_ptr,test_bits) = self.get_u64_by_index(idx as usize);
        if *u64_ptr & test_bits !=0 {
            return errcode::ERROR_ALREADY_EXIST;
        }
        *u64_ptr |= test_bits;
        self.used_count+=1;
        errcode::RESULT_SUCCESS
    }

        ///设置一个bitmap位，从0开始
    pub fn unset_bitmap(&mut self,idx:i32)->errcode::RESULT {
            if idx as usize>=self.capacity {
                return errcode::ERROR_OUTOF_MEM;
            }
            let (u64_ptr,test_bits) = self.get_u64_by_index(idx as usize);
            if *u64_ptr & test_bits ==0 {
                return errcode::ERROR_NOT_FOUND;
            }
            *u64_ptr &= !test_bits;
            self.used_count-=1;
            errcode::RESULT_SUCCESS
    }

            ///设置一个bitmap位，从0开始
            pub fn clear_bitmap(&mut self)->errcode::RESULT {
                if self.bit_string==std::ptr::null_mut() {
                    return errcode::ERROR_NULL_POINTER;
                }
                for i in 0..self.u64_count {
                    let u64_ptr = unsafe { 
                        &mut *((self.bit_string as usize + i*std::mem::size_of::<u64>()) as *mut u64)
                    };
                    *u64_ptr = 0;
                }                
                errcode::RESULT_SUCCESS
        }


    pub fn is_bit_set(&mut self,idx:i32)->bool {
        if idx as usize>=self.capacity {
            return false;
        }

        let (u64_ptr,test_bits) = self.get_u64_by_index(idx as usize);
        if (*u64_ptr) & (test_bits) !=0 {
            return true;
        } else {
            return false;
        }
    }

    pub fn get_used_count(&self)->usize {
        return self.used_count
    }

    pub fn to_string(&self)->String {
        format!("bitmap capacity={},used_count={},u64_count={}",self.capacity,self.used_count,self.u64_count)
    }
}

impl Drop for bitmap_t {
    fn drop(&mut self) {
        if self.bit_string!=std::ptr::null_mut() {
            unsafe {
            alloc::dealloc(self.bit_string as *mut u8, alloc::Layout::from_size_align_unchecked(self.u64_count*std::mem::size_of::<u64>(), 1));
            self.bit_string=std::ptr::null_mut();
            }
        }
    }
}