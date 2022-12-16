#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::common::errcode;
use std::net::{SocketAddr};
use crate::net_ext::RawFdType;
use super::*;
use super::{socketpool};

///UdpSocket API Implementation
/// the default socket implementation is aynchronous socket, do not require the app to set non-blocking mode
impl UdpSocket {
    pub fn new(local_addr:&SocketAddr)->Result<Self,errcode::RESULT> {
        let af=if local_addr.is_ipv4() {SOCKET_ADDRESS_FAMILY::SOCKET_INET} 
            else { SOCKET_ADDRESS_FAMILY::SOCKET_INET6  };
        let sock_id = match socketpool::new_socket(af, SOCKET_TYPE::PROTO_DGRAM, net_ext::IpProtos::Ip_Proto_UDP) {
            Ok(s)=>s,
            Err(e)=>return Err(e),
        };
        let sock = match socketpool::get_socket_by_idx(sock_id) {
            None=>return Err(errcode::ERROR_INIT_FAILED),
            Some(s)=>s,
        };
        sock.set_reuse_addr(true);
        let ret = sock.bind(local_addr);
        if ret!=errcode::RESULT_SUCCESS {
            return Err(ret)
        }
        let udp = Self {
            sck_idx:sock_id,
        };

        return Ok(udp)
    }

    pub fn get_socket_by_id(id:i32)->Self {
        return Self { sck_idx: id }
    }

    pub fn set_send_buf_size(&mut self,buf_size:usize)->errcode::RESULT {
        let sock = match socketpool::get_socket_by_idx(self.sck_idx) {
            None=>return errcode::ERROR_INVALID_INDEX,
            Some(s)=>s,
        };
        return sock.set_send_buffer(buf_size)
    }

    pub fn set_recv_buf_size(&mut self,buf_size:usize)->errcode::RESULT {
        let sock = match socketpool::get_socket_by_idx(self.sck_idx) {
            None=>return errcode::ERROR_INVALID_INDEX,
            Some(s)=>s,
        };
        return sock.set_recv_buffer(buf_size)
    }

    pub fn send_to(&mut self,dst:&SocketAddr,buf:&[u8])->Result<usize,errcode::RESULT> {
        let sock = match socketpool::get_socket_by_idx(self.sck_idx) {
            None=>return Err(errcode::ERROR_INVALID_INDEX),
            Some(s)=>s,
        };

        return sock.send_to(dst, buf)
    }

    pub fn recv_from(&mut self,buf:&mut [u8])->Result<(usize,SocketAddr),errcode::RESULT> {
        let sock = match socketpool::get_socket_by_idx(self.sck_idx) {
            None=>return Err(errcode::ERROR_INVALID_INDEX),
            Some(s)=>s,
        };
        
        return sock.recv_from(buf)
    }

    pub fn get_local_addr(&self)->Option<SocketAddr> {
        let sock = match socketpool::get_socket_by_idx(self.sck_idx) {
            None=>return None,
            Some(s)=>s,
        };

        return Some(sock.get_local_addr())
    }
    
    pub fn get_socket_id(&self)->i32 {
        return self.sck_idx       
    }

    ///get underlying os raw socket id
    pub fn get_os_socket(&self)->RawFdType {
        let sock = match socketpool::get_socket_by_idx(self.sck_idx) {
            None=>return 0,
            Some(s)=>s,
        };

        return sock.get_raw_fd()
    }

    pub fn close(&mut self)->errcode::RESULT {
        socketpool::close_socket(self.sck_idx)
    }

}