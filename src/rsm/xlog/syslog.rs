#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use super::*;
use crate::common::{errcode};
use std::net::{SocketAddr,UdpSocket,IpAddr};

pub struct sys_log_client_t {
	server_addr:SocketAddr,
    self_addr:SocketAddr,
	sock:     UdpSocket,
    seq:u64,
    sent_msg:u64,
    drop_msg:u64,
}

impl sys_log_client_t {
    pub fn new(self_addr:&SocketAddr)->Result<Self,errcode::RESULT>{
        let sck = match UdpSocket::bind(self_addr) {
            Ok(s)=>s,
            Err(_)=>return Err(errcode::ERROR_BIND_SOCKET),
        };
        let log= Self {
            server_addr:SocketAddr::new(IpAddr::from([127,0,0,1]),SYSLOG_DEF_UDP_PORT),
            self_addr:self_addr.clone(),
            sock:sck,
            seq:1,
            sent_msg:0,
            drop_msg:0,
        };
        return Ok(log)
    }

    pub fn sendto_server(&mut self,msg:&InnerLogMsg) {
        let msg = LogFormat(msg,self.get_cur_msg_seq(),&self.self_addr);
        match self.sock.send_to(msg.as_bytes(), self.server_addr) {
            Ok(_)=>self.sent_msg+=1,
            Err(_)=>self.drop_msg+=1,
        }
   
    }

    pub fn send_encoded_msg(&mut self,encoded_msg:&String) {
        match self.sock.send_to(encoded_msg.as_bytes(), self.server_addr) {
            Ok(_)=>self.sent_msg+=1,
            Err(_)=>self.drop_msg+=1,
        }
    }

    fn get_cur_msg_seq(&mut self)->u64 {
        let seq=self.seq;
        self.seq+=1;
        return seq
    }

    pub fn set_server_addr(&mut self,server_addr:&SocketAddr)->errcode::RESULT {
        if server_addr.eq(&self.server_addr) {
            return errcode::ERROR_ALREADY_EXIST
        }
    
        self.server_addr = server_addr.clone();
    
        return errcode::RESULT_SUCCESS
    }    
}


