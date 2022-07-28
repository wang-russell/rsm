#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use rust_rsm::alg::{self,crypto};
use rust_rsm::common;

const MAX_TEST_LEN:usize=130;
const MAX_TEST_ROUND:usize=128*1024+1;
#[test]
fn test_xor_alg() {
    let slice1=[0x41u8;MAX_TEST_LEN];
    let mut slice2=[0x65u8;MAX_TEST_LEN];

    let mut cur = common::get_now_usec64();
    for i in 0.. MAX_TEST_ROUND {
        alg::slice_xor_simple(&slice1,&mut slice2);
    }
    println!("\nslice xor simple {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&slice2));

     cur = common::get_now_usec64();
    for i in 0.. MAX_TEST_ROUND {
        alg::slice_xor(&slice1,&mut slice2);
    }
    println!("\nslice xor  {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&slice2));


}

#[test]
fn test_encrypt_alg() {
    let slice1=[0x41u8;MAX_TEST_LEN];
    let mut slice2=[0u8;MAX_TEST_LEN+32];
    let mut buffer=[0u8;MAX_TEST_LEN+32];
    let passwd="hello world";

    let enc = crypto::encrypt_alg_t::new(crypto::E_ENCRYPT_ALG::enc_alg_aes_cbc_128,passwd.as_bytes()).unwrap();
    let dec = crypto::decrypt_alg_t::new(crypto::E_ENCRYPT_ALG::enc_alg_aes_cbc_128,passwd.as_bytes()).unwrap();

    let enc_len = enc.encrypt(&slice1,&mut buffer).unwrap();
    let dec_len = match dec.decrypt(&buffer[0..enc_len],&mut slice2) {
        Err(e)=> {
            println!("decrypt error,e={}",e);
            0
         },
        Ok(l)=>l,
    };

    println!("origin slice={},\nencrypt slice={},\ndecrypt len={},content={}",
    common::rawstring::slice_to_hex_string(&slice1),
    common::rawstring::slice_to_hex_string(&buffer[0..enc_len]),
    dec_len,common::rawstring::slice_to_hex_string(&slice2[0..dec_len]));

    let mut cur = common::get_now_usec64();
    for _ in 0.. MAX_TEST_ROUND {
        let _ = enc.encrypt(&slice1,&mut buffer);
    }
    println!("\nencrypt {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&buffer[0..enc_len]));

     cur = common::get_now_usec64();
    for _ in 0.. MAX_TEST_ROUND {
        let _ = dec.decrypt(&buffer[0..enc_len],&mut slice2);
    }
    println!("\ndecrypt  {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&slice2[0..dec_len]));


}