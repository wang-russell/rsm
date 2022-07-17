#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

//根据三个数字，计算出来一个128bit的Hash值
pub fn hash_3value_128(v1: &[u8], v2: &[u8], v3: &[u8]) -> [u8; 16] {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(v1);
    buf.push(0x20);
    buf.extend_from_slice(v2);
    buf.push(0x20);
    buf.extend_from_slice(v3);

    let mut hasher = Sha256::new();
    hasher.update(&buf.as_slice());
    let res = hasher.finalize();
    let mut hv: [u8; 16] = [0; 16];
    unsafe {
        std::ptr::copy(&res[0] as *const u8, &mut hv[0] as *mut u8, 16);
    }
    return hv;
}

//生成一个128bit的随机数
pub fn get_rand_128() -> [u8; 16] {
    let mut nonce: [u8; 16] = [0; 16];    
    OsRng.fill_bytes(&mut nonce[0..16]);
    return nonce;
}
