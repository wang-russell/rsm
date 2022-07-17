#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
pub mod hash_alg;
pub mod crypto;

use std::mem;
use crate::common::errcode;

const MAX_SIZE:usize = 65536;
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