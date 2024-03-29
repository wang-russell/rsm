#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
pub mod hash_alg;
pub mod crypto;

pub use crypto::crypto_alg_t;

use std::mem;
use crate::common::errcode;

pub mod spf;
pub use spf::spf_matrix_t;
pub mod prio_queue;
pub use prio_queue::priority_queue_t;

const MAX_SIZE:usize = 65536;

const SPF_MAX_NODE_NUM:usize = 1024;

///将两个切片进行Xor处理，结果写入dst中；需要dst的长度大于src，src不足的长度按照0进行补足
pub fn slice_xor(src:&[u8],dst:&mut [u8])->errcode::RESULT {
    let src_len = src.len();
    let dst_len = dst.len();

    if dst_len<src_len {
        return errcode::ERROR_BUFFER_TOO_SMALL;
    }
    let u64_src_len:usize = src_len/mem::size_of::<u64>();    

    let u64_dst = unsafe { &mut *(dst.as_mut_ptr() as *mut [u64;MAX_SIZE]) };
    let u64_src = unsafe { &*(src.as_ptr() as *const [u64;MAX_SIZE])};

    for i in 0..u64_src_len {
        u64_dst[i] ^=u64_src[i];
    }
    let u64_dst_len:usize = dst_len/mem::size_of::<u64>();
    for j in u64_src_len..u64_dst_len {
        u64_dst[j] ^=0u64;
    }

    for k in u64_dst_len*mem::size_of::<u64>()..dst_len {
        dst[k] ^=0u8;
    }
    return errcode::RESULT_SUCCESS
}

///将两个切片进行Xor处理，结果写入dst中；需要dst的长度大于src，src不足的长度按照0进行补足
pub fn slice_xor_simple(src:&[u8],dst:&mut [u8])->errcode::RESULT {
    let src_len = src.len();
    let dst_len = dst.len();

    if dst_len<src_len {
        return errcode::ERROR_BUFFER_TOO_SMALL;
    }
    
    for i in 0..src_len {
       dst[i] ^= src[i]
    }

    for k in src_len..dst_len {
        dst[k] ^=0u8;
    }
    return errcode::RESULT_SUCCESS
}

pub fn log2(v:u64)->usize {
    if v==0 {
        return 0
    }
    for i in 1..u64::BITS+1 {
        if v>>i == 0 {
            return (i-1) as usize
        }
    }
    return 0
    
}

///将第二个数组合并到第一个数组中，并且去重
pub fn merge_slice<T>(v1:&mut Vec<T>,v2:&Vec<T>) 
    where T:PartialEq+Clone {
        for s in v2 {
            if !v1.contains(s){
                v1.push((*s).clone())
            }
        }
}

///将第二个数组合并到第一个数组中，并且去重
use std::hash::Hash;
pub fn merge_slice2<T>(v1:&mut Vec<T>,v2:&Vec<T>) 
    where T:PartialEq+Clone+Eq+Hash+Sized {
        use std::collections::HashSet;
        let mut m:HashSet<T>=HashSet::with_capacity(v1.len()<<1);
        m=v1.iter().map(|x| x.clone()).collect();
        /*
        for i in 0..v1.len() {
            m.insert(v1[i].clone());
        }        */

        for s in v2{
            if !m.contains(s) {
                v1.push(s.clone())
            }
        }
}