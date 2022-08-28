#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::net_ext::{self,mac_addr::mac_addr_t};
use std::net::SocketAddr;
use crate::common::errcode;
use crate::rsm;
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock;

#[cfg(unix)]
use libc;

pub mod socket;
pub mod poll;
pub mod socketpool;
pub mod udpsocket;
pub mod tcplistener;
pub mod tcpsocket;



pub type socket_event_t= rsm::rsm_socket_event_t;

#[derive(Clone,Debug,PartialEq,Copy)]
pub enum SOCKET_ADDRESS_FAMILY {
    SOCKET_INET = 1,
    SOCKET_INET6 = 2,
    SOCKET_RAW_PACKET=4,
}

#[derive(Clone,Debug,PartialEq,Copy,Serialize,Deserialize)]
pub enum SOCKET_TYPE {
    PROTO_RAW = 0,
    PROTO_DGRAM = 1,
    PROTO_STREAM = 2,
}

#[derive(Clone,Debug,PartialEq,Copy)]
pub enum SOCKET_STATE {
    SOCK_INIT = 0,
    SOCK_BIND = 1,
    SOCK_LISTENING = 2,
    SOCK_CONNECTING = 3,
    SOCK_CONNECTED = 4,
}

///SOCKET_LB_POLICY, control the TCP Listener how to dispatch tcp client connection to the caller component instance
/// default policy is dispatch to all the component instance by hash result
#[derive(Clone,Debug,PartialEq,Copy)]
pub enum SOCKET_LB_POLICY {
    ///dispatch tcp client connections to all the component instance by hash result
    SOCK_LB_ALL_INSTANCE=0,
    ///tcp connections only handled by the caller instance
    SOCK_LB_CALLER_INSTANCE=1,
    ///tcp connections dispatch to the component instances except the caller 
    SOCK_LB_EXCLUDE_CALLER_INSTANCE=2,
}

#[derive(Clone,Debug)]
pub struct Socket {
    sock_id:i32,
    os_fd:net_ext::RawFdType,
    sock_af:SOCKET_ADDRESS_FAMILY,
    sock_type:SOCKET_TYPE,
    proto:u8,
    state:SOCKET_STATE,
    tcp_server:bool,
    lb_policy:SOCKET_LB_POLICY,
    local_addr:SocketAddr,
    peer_addr:SocketAddr,
}
pub trait AsSocket {
    fn as_socket(&mut self)->Socket;
}

#[derive(Clone,Debug)]
pub struct UdpSocket {
    sck_idx:i32,
}

#[derive(Clone,Debug)]
pub struct TcpListener {
    sck_idx:i32,
}

#[derive(Clone,Debug)]
pub struct TcpSocket {
    sck_idx:i32,
}

#[derive(Clone,Debug)]
pub struct RawSocket {
    sck_idx:i32,
}

#[derive(Clone,Debug)]
pub struct PacketSocket {
    sck_idx:i32,
    if_idx:u32,
    mac:mac_addr_t
}

#[cfg(windows)]
fn socket(sock_af:SOCKET_ADDRESS_FAMILY,sock_type:SOCKET_TYPE,proto:u8)->Result<net_ext::RawFdType,errcode::RESULT> {
    let af = match sock_af {
        SOCKET_ADDRESS_FAMILY::SOCKET_INET=>WinSock::AF_INET,
        SOCKET_ADDRESS_FAMILY::SOCKET_INET6=>WinSock::AF_INET6,
        _=>WinSock::AF_UNSPEC,
    };

    let stype = match sock_type {
        SOCKET_TYPE::PROTO_RAW=>WinSock::SOCK_RAW,
        SOCKET_TYPE::PROTO_STREAM=>WinSock::SOCK_STREAM,
        SOCKET_TYPE::PROTO_DGRAM=>WinSock::SOCK_DGRAM,
    };
    
    let s = unsafe { WinSock::WSASocketA(af as i32, stype as i32, proto as i32,
    std::ptr::null(),0,WinSock::WSA_FLAG_OVERLAPPED) };
    return Ok(s as net_ext::RawFdType)
}

#[cfg(unix)]
fn socket(sock_af:SOCKET_ADDRESS_FAMILY,sock_type:SOCKET_TYPE,proto:u8)->Result<net_ext::RawFdType,errcode::RESULT> {
    let af = match sock_af {
        SOCKET_ADDRESS_FAMILY::SOCKET_INET=>libc::AF_INET,
        SOCKET_ADDRESS_FAMILY::SOCKET_INET6=>libc::AF_INET6,
        SOCKET_ADDRESS_FAMILY::SOCKET_RAW_PACKET=>libc::AF_PACKET,
    };

    let stype = match sock_type {
        SOCKET_TYPE::PROTO_RAW=>libc::SOCK_RAW,
        SOCKET_TYPE::PROTO_STREAM=>libc::SOCK_STREAM,
        SOCKET_TYPE::PROTO_DGRAM=>libc::SOCK_DGRAM,
    };

    let s = unsafe { libc::socket(af as i32, stype as i32, proto as i32) };
    return Ok(s as net_ext::RawFdType)
}