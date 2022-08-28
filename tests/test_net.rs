#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::time::Duration;
use std::{self, net};
use rust_rsm::common::{self, errcode,rawstring};
use rust_rsm::net_ext::{self,mac_addr,ethernet_pkt};
use std::net::{IpAddr,SocketAddr};
use mio::{Poll,Interest,Token};

#[cfg(unix)]
use rust_rsm::net_ext::unix::rawsocket;

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};

mod test_common;

const MAX_TEST_BUF_SIZE:usize = 3000;

const MAX_RECV_BUF_SIZE:usize=2048;
static mut gRecvBuf1:[u8;MAX_RECV_BUF_SIZE*16]=[0;MAX_RECV_BUF_SIZE*16];
fn get_buffer(idx:usize)->&'static mut [u8] {
    return unsafe {&mut gRecvBuf1[MAX_RECV_BUF_SIZE*idx..MAX_RECV_BUF_SIZE*(idx+1)]};
}


#[test]
fn test_net_ext() {
    let mut mac=mac_addr::mac_addr_t::new(123,89,13,5,225,15);
    println!("mac={},is_broadcast={},is_multicast={}",mac,mac.is_broadcast(),mac.is_multicast());
    mac = mac_addr::mac_addr_t::new(255,255,255,255,255,255);
    println!("mac={},is_broadcast={},is_multicast={},size={}",
    mac,mac.is_broadcast(),mac.is_multicast(),std::mem::size_of::<mac_addr::mac_addr_t>());

    let eth:[u8;14]=[0xfe,1,2,3,4,5,0xfd,1,2,3,4,5,0x08,0];
    let ip:[u8;12] =[0x45,0,0,8,4,5,6,7,8,17,10,11];
    let udp:[u8;8] = [0,10,0,12,0,8,0,0];
    let payload:[u8;8]=[1,1,1,1,1,1,1,1];
    let mut packet:Vec<u8> = Vec::new();
    let src_ip = net::IpAddr::from([10,255,1,6]);
    let dst_ip = net::IpAddr::from([10,255,1,7]);
    let (_,ip1)=net_ext::ipaddr_to_array(&src_ip);
    let (_,ip2)=net_ext::ipaddr_to_array(&dst_ip);
    packet.extend_from_slice(&eth);
    packet.extend_from_slice(&ip);
    packet.extend_from_slice(&ip1[0..4]);
    packet.extend_from_slice(&ip2[0..4]);
    packet.extend_from_slice(&udp);
    packet.extend_from_slice(&payload);
    let packet = ethernet_pkt::ethernet_packet_info_t::from_ethernet_packet(packet.as_slice()).unwrap();
    println!("Packet:{}",packet.to_string());
    assert_eq!(packet.ip_proto,17);
    assert_eq!(packet.tp_src,10);

}

#[test]
fn test_network_if() {
    use rust_rsm::net_ext::netinterface::{self,NetworkInterface};
    let ifs = NetworkInterface::get_sys_interfaces();
    for oneif in ifs {
        println!("System interface is {:?}",oneif);
    }
    let ifname="woc_1".to_string();
    match NetworkInterface::get_interface_by_name(&ifname) {
        Ok(nif)=> {
            println!("get interface {} is {:?}",ifname,nif);
        },
        Err(ec)=> println!("get interface {} err {}",ifname,ec),
    }
    
    match netinterface::get_macaddr_by_name(&ifname) {
        Ok(m)=>println!("interface {} mac is {}",ifname,m),
        Err(ec)=>println!("get interface {} mac failed {}",ifname,ec),
    }
}

const MAX_ROUTE_REC_COUNT:usize = 100000;
use rust_rsm::net_ext::iprt;
#[test]
fn test_ip_route() {
   
    let mut iprt_tbl = iprt::ip_route_table_t::<u64>::new(MAX_ROUTE_REC_COUNT);

    let mut failed=0;
    let vrf:i32=1;
    let mut masklen:u8=24;
    let mut ip:u32 = u32::from_ne_bytes([1,1,1,1]);
    let mut cur = common::get_now_usec64();
    for i in 0.. MAX_ROUTE_REC_COUNT {
        let ip_addr = IpAddr::from(ip.to_be_bytes());
        let res = iprt_tbl.add_ip_route(vrf,&ip_addr,masklen,10,1,&(i as u64));
        if res!=errcode::RESULT_SUCCESS {
            failed+=1;
        }
        masklen = 16+(masklen + 1) % 10;
        ip+=1<<(32-masklen);
    }
    println!("ip table insert {} records, actual size is {},failed={},spend {} us",
        MAX_ROUTE_REC_COUNT,iprt_tbl.len(),failed,common::get_now_usec64()-cur);
    iprt_tbl.print_stats();

    cur = common::get_now_usec64();
    failed=0;
    ip = u32::from_ne_bytes([1,1,1,1]);
    for _ in 0.. MAX_ROUTE_REC_COUNT {
        let ip_addr = IpAddr::from(ip.to_be_bytes());
        match iprt_tbl.lookup_ip_route(vrf,&ip_addr) {
            None=> {
                failed+=1;
            },
            Some(_)=>(),
        }        
        ip+=64;
    }
    println!("ip table lookup {} records, actual size is {},failed={},spend {} us",
        MAX_ROUTE_REC_COUNT,iprt_tbl.len(),failed,common::get_now_usec64()-cur);
        delete_ip_route(&mut iprt_tbl);
}

fn delete_ip_route(iptbl:&mut iprt::ip_route_table_t<u64>) {

    let mut failed=0;
    let vrf:i32=1;
    let mut masklen:u8=24;
    let mut ip:u32 = u32::from_ne_bytes([1,1,1,1]);
    let cur = common::get_now_usec64();
    for i in 0.. MAX_ROUTE_REC_COUNT {
        let ip_addr = IpAddr::from(ip.to_be_bytes());
        let res = iptbl.delete_ip_route(vrf,&ip_addr,masklen,10,1,&(i as u64));
        if res!=errcode::RESULT_SUCCESS {
            failed+=1;
        }
        masklen = 16+(masklen + 1) % 10;
        ip+=1<<(32-masklen);
    }
    println!("ip table delete {} records, final capacityis {},size is {},failed={},spend {} us",
        MAX_ROUTE_REC_COUNT,iptbl.capacity(),iptbl.len(),failed,common::get_now_usec64()-cur);
        iptbl.print_stats();
}