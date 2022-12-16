use std::fmt::Display;

use rand;
use serde::{Deserialize,Serialize};

#[derive(Debug,Clone,Deserialize,Serialize)]
pub struct uuid_t ([u8;16]);
impl uuid_t {
    pub fn new()->Self {
        let v=rand::random::<u128>().to_be_bytes();
        return Self (v)
    }
    pub fn from_u128(v:u128)->Self {
        return Self(v.to_be_bytes())
    }
    pub fn from_u64(high:u64,low:u64)->Self {
        let v =(high as u128) << 64 | (low as u128);
        return Self (v.to_be_bytes())
    }
    pub fn as_u128(&self)->u128 {
        u128::from_ne_bytes(self.0)
    }
    
    fn fromat(&self)->String {
        let va=&self.0;
        format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        va[0],va[1],va[2],va[3],va[4],va[5],va[6],va[7],va[8],va[9],va[10],va[11],va[12],va[13],va[14],va[15])
    }
}

impl Display for uuid_t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.fromat())
    }
}
impl Default for uuid_t {
    fn default() -> Self {
        return Self([0;16])
    }
    
}