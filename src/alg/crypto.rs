#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_variables)]
#![allow(dead_code)]

use aes::{self,Aes128,Aes256, Block};
use sm4::Sm4;
use crate::common::{self,errcode};
use super::*;
use rand::random;
use aes::cipher::{
    BlockEncrypt, BlockDecrypt, KeyInit,
    generic_array::GenericArray,
};
const MAX_ENCRYPT_BLOCK_SIZE:usize=2048;
const CIPHER_BLOCK_SIZE_128:usize=16;
const CIPHER_BLOCK_SIZE_256:usize=32;
const MAX_PASSWD_SIZE:usize=32;

#[derive(PartialEq,Eq,Copy,Clone)]
pub enum E_ENCRYPT_ALG {
    enc_alg_null = 0,
    enc_alg_aes_cbc_128=1,
    enc_alg_aes_cbc_256=2,
    enc_alg_aes_gcm_128=3,
    enc_alg_aes_gcm_256=4,
    enc_alg_sm4=5,
}

#[derive(Clone)]
pub struct crypto_alg_t {
    alg:E_ENCRYPT_ALG,
    passwd_len:usize,
    passwd:[u8;MAX_PASSWD_SIZE],
    cryptor_aes_128:Option<Aes128>,
    cryptor_sm4:Option<Sm4>,
    
}

impl crypto_alg_t {
    pub fn new(alg:E_ENCRYPT_ALG,passwd:&[u8])->Result<Self,errcode::RESULT> {
        let mut enc_alg = Self {            
            alg,
            passwd_len:std::cmp::min(passwd.len(),MAX_PASSWD_SIZE),
            passwd:[0;MAX_PASSWD_SIZE],
            cryptor_aes_128:None,
            cryptor_sm4:None,

        };
        unsafe {
            std::ptr::copy_nonoverlapping(passwd.as_ptr(), enc_alg.passwd.as_mut_ptr(),enc_alg.passwd_len);
        }
        match alg {
            E_ENCRYPT_ALG::enc_alg_aes_cbc_128=> {
                enc_alg.cryptor_aes_128=match Aes128::new_from_slice(&enc_alg.passwd[0..16]) {
                    Ok(e)=>Some(e),
                    Err(_)=>None,
                };
            },
            E_ENCRYPT_ALG::enc_alg_sm4=>{
                enc_alg.cryptor_sm4=match Sm4::new_from_slice(&enc_alg.passwd[0..16]) {
                    Ok(e)=>Some(e),
                    Err(_)=>None,
                };
            },
            _=>(),
        }
        return Ok(enc_alg)
    }

    #[inline(always)]
    fn encrypt_bock(&self,input:&Block,output:&mut Block) {
        match self.alg {
            E_ENCRYPT_ALG::enc_alg_aes_cbc_128=> {
                if let Some(e)=&self.cryptor_aes_128 {
                    e.encrypt_block_b2b(input, output);
                }
            },
            E_ENCRYPT_ALG::enc_alg_sm4=> {
                if let Some(e)=&self.cryptor_sm4 {
                    e.encrypt_block_b2b(input, output);
                }                
            },
            _=>(),
        }
    }
    ///AES 128加密
    pub fn encrypt_aes128_sm4(&self,src:&[u8],dst:&mut [u8])->Result<usize,errcode::RESULT> {
        let src_len = src.len();
        if dst.len()<src_len+18 {
            return Err(errcode::ERROR_BUFFER_TOO_SMALL)
        }        
        
        let iv=random::<u128>().to_be_bytes();
        let len_bytes = (src_len as u16).to_be_bytes();
        //起始两个字节存放原始报文长度
        dst[0]=len_bytes[0];
        dst[1]=len_bytes[1];
        let pad_len = common::ceiling(src_len as u64,CIPHER_BLOCK_SIZE_128 as  u64) as usize * CIPHER_BLOCK_SIZE_128;
        let mut pad_buf = Vec::with_capacity(pad_len);

        pad_buf.extend_from_slice(src);
        pad_buf.resize(pad_len,0);

        let mut input = GenericArray::from_slice(&iv[0..CIPHER_BLOCK_SIZE_128]);
        let mut output = GenericArray::from_mut_slice(&mut dst[2..2+CIPHER_BLOCK_SIZE_128]);
        
        self.encrypt_bock(&input,&mut output);
        let mut dst_start=2+CIPHER_BLOCK_SIZE_128;
        let mut src_start = 0;
        while src_start<src_len {
            slice_xor_simple(&iv,&mut pad_buf[src_start..src_start+CIPHER_BLOCK_SIZE_128]);
            input = GenericArray::from_slice(&pad_buf[src_start..src_start+CIPHER_BLOCK_SIZE_128]);         
            output = GenericArray::from_mut_slice(&mut dst[dst_start..dst_start+CIPHER_BLOCK_SIZE_128]);
            self.encrypt_bock(&input,&mut output);
            dst_start+=CIPHER_BLOCK_SIZE_128;
            src_start+=CIPHER_BLOCK_SIZE_128;         
        }

        return Ok(pad_len+18)

    }



    pub fn encrypt_aes_256(&self,src:&[u8],dst:&mut [u8])->Result<usize,errcode::RESULT> {
        Err(errcode::ERROR_NOT_SUPPORT)
    }
    pub fn encrypt(&self,src:&[u8],dst:&mut [u8])->Result<usize,errcode::RESULT> {
        
        match self.alg {
            E_ENCRYPT_ALG::enc_alg_aes_cbc_128=>return self.encrypt_aes128_sm4(src, dst),
            E_ENCRYPT_ALG::enc_alg_aes_cbc_256=>return self.encrypt_aes_256(src, dst),
            E_ENCRYPT_ALG::enc_alg_sm4=>return self.encrypt_aes128_sm4(src, dst),
            _=>return Err(errcode::ERROR_NOT_SUPPORT),
        }
    
    }

    pub fn get_alg(&self)->E_ENCRYPT_ALG {
        return self.alg
    }

    #[inline(always)]
    fn decrypt_bock(&self,input:&Block,output:&mut Block) {
        match self.alg {
            E_ENCRYPT_ALG::enc_alg_aes_cbc_128=> {
                if let Some(e)=&self.cryptor_aes_128 {
                    e.decrypt_block_b2b(input, output);
                }
            },
            E_ENCRYPT_ALG::enc_alg_sm4=> {
                if let Some(e)=&self.cryptor_sm4 {
                    e.decrypt_block_b2b(input, output);
                }                
            },
            _=>(),
        }
    }

    pub fn decrypt_aes128_sm4(&self,src:&[u8],dst:&mut [u8])->Result<usize,errcode::RESULT> {
        let src_len = src.len();
        if dst.len()+18<src_len {
            return Err(errcode::ERROR_BUFFER_TOO_SMALL)
        }

        let mut iv=[0u8;16];
        
        //起始两个字节存放原始报文长度
        let data_len:u16 = ((src[0] as u16) << 8) +src[1] as u16;
        if src_len<data_len as usize+2 || ((src_len-2) % CIPHER_BLOCK_SIZE_128!=0) {
            //println!("src_len={},data_len={}",src_len,data_len);
            return Err(errcode::ERROR_INVALID_MSG)
        }
        let mut input = GenericArray::from_slice(&src[2..2+CIPHER_BLOCK_SIZE_128]);
        let mut output = GenericArray::from_mut_slice(&mut iv);
        self.decrypt_bock(&input,&mut output);
        let mut dst_start=0;
        let mut src_start = 2+CIPHER_BLOCK_SIZE_128;
        while src_start<src_len {
            //let step = std::cmp::min(CIPHER_BLOCK_SIZE_128, src_len-src_start);
            input = GenericArray::from_slice(&src[src_start..src_start+CIPHER_BLOCK_SIZE_128]);
            output = GenericArray::from_mut_slice(&mut dst[dst_start..dst_start+CIPHER_BLOCK_SIZE_128]);
            self.decrypt_bock(&input,&mut output);

            slice_xor_simple(&iv,&mut dst[dst_start..dst_start+CIPHER_BLOCK_SIZE_128]);
            dst_start+=CIPHER_BLOCK_SIZE_128;
            src_start+=CIPHER_BLOCK_SIZE_128;
        }

        return Ok(data_len as usize)

    }

    pub fn decrypt_aes_256(&self,src:&[u8],dst:&mut [u8])->Result<usize,errcode::RESULT> {
        Err(errcode::ERROR_NOT_SUPPORT)
    }


    pub fn decrypt(&self,src:&[u8],dst:&mut [u8])->Result<usize,errcode::RESULT> {

        match self.alg {
            E_ENCRYPT_ALG::enc_alg_aes_cbc_128=>return self.decrypt_aes128_sm4(src, dst),
            E_ENCRYPT_ALG::enc_alg_aes_cbc_256=>return self.decrypt_aes_256(src, dst),
            E_ENCRYPT_ALG::enc_alg_sm4=>return self.decrypt_aes128_sm4(src, dst),
            _=>return Err(errcode::ERROR_NOT_SUPPORT),
        }

    }
}