#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use super::*;
use std::fmt;
use serde::{Serialize,Deserialize};
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Default,Debug,Deserialize,Serialize,Hash)]
pub struct mac_addr_t {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
}
const MULTICAST_BIT: u8 = 0x01;
impl fmt::Display for mac_addr_t {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self.to_string())
    }
}
impl mac_addr_t {
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> mac_addr_t {
        return mac_addr_t { a, b, c, d, e, f };
    }
    pub fn new_broadcast() -> mac_addr_t {
        return Self::new(0xff, 0xff, 0xff, 0xff, 0xff, 0xff);
    }
    pub fn zero() -> mac_addr_t {
        return Self::new(0, 0, 0, 0, 0, 0);
    }

    pub fn from_array(addr:&[u8;MAC_ADDR_SIZE])->mac_addr_t {
        return mac_addr_t {a:addr[0],b:addr[1],c:addr[2],d:addr[3],e:addr[4],f:addr[5]};
    }
    pub fn from_slice(addr:&[u8])->mac_addr_t {
        if addr.len()<MAC_ADDR_SIZE {
            return Self::new(0, 0, 0, 0, 0, 0);
        }
        return   mac_addr_t {a:addr[0],b:addr[1],c:addr[2],d:addr[3],e:addr[4],f:addr[5]};
    }
    pub fn is_zero(&self)->bool {
        return self.to_u64()==0
    }
    pub fn from_u64(addr: u64) -> mac_addr_t {
        let p = addr.to_be_bytes();

        return Self::new(p[2], p[3], p[4], p[5], p[6], p[7]);
    }

    pub fn to_u64(&self) -> u64 {
        let p: [u8; 8] = [0,0,self.a, self.b, self.c, self.d, self.e, self.f];
            return u64::from_be_bytes(p)
    }
    pub fn to_slice(&self)->&[u8] {
        let p = unsafe { &*(&self.a as *const u8 as *const [u8;MAC_ADDR_SIZE]) };
        return p;
    }
    pub fn is_broadcast(&self) -> bool {
        return *self == Self::new_broadcast();
    }
    pub fn is_multicast(&self) -> bool {
        return self.a & MULTICAST_BIT == MULTICAST_BIT;
    }

    pub fn as_ptr(&self)->*const u8 {
        std::ptr::addr_of!(self.a)
    }

    pub fn from_string(mac_str:&String)->Option<Self> {
        let mac_a:Vec<&str>=mac_str.split(":").collect();
        if mac_a.len()<6 {
            return None
        }

        let mut abytes=[0u8;6];

        for i in 0..6 {
            abytes[i]=match u8::from_str_radix(mac_a[i], 16) {
                Ok(v)=>v,
                Err(_)=>0,
            }
        }

        Some(Self::from_array(&abytes))
    }
    #[cfg(windows)]
    pub fn to_string(&self)->String {
        format!("{:02x}-{:02x}-{:02x}-{:02x}-{:02x}-{:02x}",self.a,self.b,self.c,self.d,self.e,self.f)
    }
    #[cfg(unix)]
    pub fn to_string(&self)->String {
        format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",self.a,self.b,self.c,self.d,self.e,self.f)
    }
}