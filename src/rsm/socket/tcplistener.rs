#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::common::errcode;
use std::net::{SocketAddr};
use crate::net_ext::RawFdType;
use super::*;
use super::{socketpool};

///TcpListener API Implementation
///Tcp Listener is maintained by RSM, every tcp connection is accepted automatically by rsm
/// and each connection is dispatched to the component by loadbalance policy set by caller
/// the default LB policy is hashed by the component's task instanced number
impl TcpListener {
    pub fn new(local_addr:&SocketAddr,max_back_log:i32,policy:SOCKET_LB_POLICY)->Result<Self,errcode::RESULT> {
        let af=if local_addr.is_ipv4() {SOCKET_ADDRESS_FAMILY::SOCKET_INET} 
            else { SOCKET_ADDRESS_FAMILY::SOCKET_INET6  };
        let sock_id = match socketpool::new_socket(af, SOCKET_TYPE::PROTO_STREAM, net_ext::IpProtos::Ip_Proto_TCP) {
            Ok(s)=>s,
            Err(e)=>return Err(e),
        };
        let sock = match socketpool::get_socket_by_idx(sock_id) {
            None=>return Err(errcode::ERROR_INIT_FAILED),
            Some(s)=>s,
        };
        let ret = sock.bind(local_addr);
        if ret!=errcode::RESULT_SUCCESS {
            socketpool::close_socket(sock_id);
            return Err(ret)
        }
        let ret = sock.listen(max_back_log);
        if ret!=errcode::RESULT_SUCCESS {
            socketpool::close_socket(sock_id);
            return Err(ret)
        }
        sock.set_lb_policy(policy);
        let lis = Self {
            sck_idx:sock_id,
        };

        return Ok(lis)
    }

    pub fn close(&mut self)->errcode::RESULT {
        return socketpool::close_socket(self.sck_idx)
    }

    pub fn get_lb_policy(&self)->Result<SOCKET_LB_POLICY,errcode::RESULT> {
        let sock = match socketpool::get_socket_by_idx(self.sck_idx) {
            None=>return Err(errcode::ERROR_INVALID_INDEX),
            Some(s)=>s,
        };

        return sock.get_lb_policy()
    }

    pub fn get_sock_id(&self)->i32 {
        self.sck_idx
    }
    ///get underlying os raw socket id
    pub fn get_os_socket(&self)->RawFdType {
        let sock = match socketpool::get_socket_by_idx(self.sck_idx) {
            None=>return 0,
            Some(s)=>s,
        };

        return sock.get_raw_fd()
    }    
}