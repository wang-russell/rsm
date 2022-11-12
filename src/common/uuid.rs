use rand;
use serde::{Deserialize,Serialize};

#[derive(Debug,Clone,Deserialize,Serialize)]
pub struct uuid_t {
    v:u128,
}
impl uuid_t {
    pub fn new()->Self {
        return Self {
            v:rand::random(),
        }
    }
    pub fn from_u128(v:u128)->Self {
        return Self { v: v }
    }
    pub fn from_u64(high:u64,low:u64)->Self {
        return Self { v: (high as u128) << 64 | (low as u128) }
    }
    pub fn as_u128(&self)->u128 {
        self.v
    }
    
    fn fromat(&self)->String {
        let va=self.v.to_be_bytes();
        format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        va[0],va[1],va[2],va[3],va[4],va[5],va[6],va[7],va[8],va[9],va[10],va[11],va[12],va[13],va[14],va[15])
    }
}

impl ToString for uuid_t {
    fn to_string(&self) -> String {
        self.fromat()
    }
}