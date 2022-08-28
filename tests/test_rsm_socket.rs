#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use rust_rsm::{common::{self,errcode}, rsm::socket::Socket};
use std::net::SocketAddr;
use std::time::Duration;
use std::thread;
use rust_rsm::rsm::{self,socket,rsm_timer,xlog};

fn init_rsm() {
    let log_addr=SocketAddr::new("127.0.0.1".parse().unwrap(),15000);
    let oam_addr=SocketAddr::new("127.0.0.1".parse().unwrap(),12000);

    let cfg = rsm::config::rsm_init_cfg_t::new(1, Some(log_addr), 
        Some(oam_addr), None);
    rsm::rsm_init(&cfg);
}

#[test]

fn test_rsm_socket() {
    init_rsm();
    let mut p=socket::poll::Poll::new(128);
    let addr1=SocketAddr::new("0.0.0.0".parse().unwrap(), 14010);
    let lis=match socket::TcpListener::new(&addr1, 1024, socket::SOCKET_LB_POLICY::SOCK_LB_ALL_INSTANCE) {
        Ok(s)=>s,
        Err(e)=>{
            println!("Create TCP Listener failed,ret={},local_addr={}",e,addr1);
            assert!(false);
            return
        },
    };
    
    let ret = p.register(lis.get_os_socket(), lis.get_sock_id() as usize, rsm::SOCK_EVENT_READ | rsm::SOCK_EVENT_CLOSE,false);
    assert_eq!(ret,errcode::RESULT_SUCCESS);
}