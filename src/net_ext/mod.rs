#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use crate::common::errcode;
use std::net::{IpAddr,SocketAddr,Ipv4Addr,Ipv6Addr};
#[cfg(unix)]
use std::os::raw::{c_int};
use std::process::Command;
#[cfg(unix)]
use std::fs::{OpenOptions};

#[cfg(unix)]
use std::os::unix::io::{RawFd};
#[cfg(windows)]
use std::os::windows::io::{RawHandle};
#[cfg(windows)]
use std::os::windows::io::{RawSocket};
use std::ptr;

pub mod arp;
pub mod mac_addr;
pub use mac_addr::mac_addr_t;

pub mod ethernet_pkt;

pub mod rawpacket;
pub mod pktbuf;
pub mod fec;
pub mod ipnetwork;
pub use ipnetwork::IpNetwork;

pub mod netinterface;
pub mod iprt;
pub mod restserver;

#[cfg(windows)]
pub mod windows;
#[cfg(unix)]
pub mod unix;


#[cfg(unix)]
pub type RawFdType = RawFd;
#[cfg(windows)]
pub type RawFdType = RawSocket;

#[cfg(unix)]
pub type RawFileFd = RawFd;
#[cfg(windows)]
pub type RawFileFd = RawHandle;

pub const MAC_ADDR_SIZE:usize = 6;
pub const ETHERNET_HDR_SIZE:usize = 14;
pub const IPV4_HDR_SIZE:usize = 20;
pub const IPV6_HDR_SIZE:usize = 40;
pub const UDP_HDR_SIZE:usize = 8;
pub const TCP_HDR_SIZE:usize = 20;
pub const ARP_PACKET_SIZE:usize = 28;
pub const DEFAULT_ETHERNET_MTU:u16 = 1500;
///net_ext共用的pakcet buffer size，比MTU大一些
const MAX_PKT_BUF_SIZE:u16 = 2048;

pub const AF_INET:u16=2;
pub const AF_INET6:u16=23;

pub mod EthernetTypes {
    pub const Etherhet_Ipv4: u16 = 0x0800;
    pub const Etherhet_ARP: u16 = 0x0806;
    //Reverse Arp
    pub const Etherhet_RARP: u16 = 0x8035;

    pub const Etherhet_Ipv6: u16 = 0x86dd;
    pub const Etherhet_Vlan: u16 = 0x8100;
    pub const Etherhet_SVlan: u16 = 0x88a8;

    pub const Etherhet_STP: u16 = 0x8181;
    pub const Etherhet_LLDP: u16 = 0x88cc;

    pub const Etherhet_MPLS: u16 = 0x8847;
    pub const Etherhet_MPLS_USL: u16 = 0x8848;

    pub const Etherhet_PPPoE_Discovery: u16 = 0x8863;
    pub const Etherhet_PPPoE_Session: u16 = 0x8864;
}

pub const IPV4_ADDR_LEN:usize = 4;
pub const IPV6_ADDR_LEN:usize = 16;

pub mod IpProtos {
    pub const Ip_Proto_ICMP: u8 = 1;
    pub const Ip_Proto_IGMP: u8 = 2;
    pub const Ip_Proto_IPv4inIP: u8 = 4;
    pub const Ip_Proto_TCP: u8 = 6;
    pub const Ip_Proto_UDP: u8 = 17;
    pub const Ip_Proto_DCCP: u8 = 33;
    pub const Ip_Proto_IPv6_Route: u8 = 43;
    pub const Ip_Proto_IPv6_Frag: u8 = 44;

    pub const Ip_Proto_RSVP: u8 = 46;
    pub const Ip_Proto_GRE: u8 = 47;

    pub const Ip_Proto_ESP: u8 = 50;
    pub const Ip_Proto_AH: u8 = 51;

    pub const Ip_Proto_ICMPv6: u8 = 58;

    pub const Ip_Proto_EIGRP: u8 = 88;
    pub const Ip_Proto_OSPF: u8 = 89;

    pub const Ip_Proto_PIM: u8 = 103; //Protocol independant Multicast
    pub const Ip_Proto_L2TP: u8 = 115;
    pub const Ip_Proto_ISIS_IPv4: u8 = 124;
    pub const Ip_Proto_SCTP: u8 = 132;
    pub const Ip_Proto_MANET: u8 = 138;
    pub const Ip_Proto_HIP: u8 = 139;
    pub const Ip_Proto_ROHC: u8 = 142;
}

//典型业务的端口号
pub mod ServicePorts {
    pub const TP_PORT_FTP_DATA: u16 = 20;
    pub const TP_PORT_FTP: u16 = 21;
    pub const TP_PORT_SSH: u16 = 22;
    pub const TP_PORT_TELNET: u16 = 23;
    pub const TP_PORT_TACACS: u16 = 49;
    pub const TP_PORT_DNS: u16 = 53;
    pub const TP_PORT_DHCP_CLIENT: u16 = 67;
    pub const TP_PORT_DHCP_SERVER: u16 = 68;
    pub const TP_PORT_HTTP: u16 = 80;

    pub const TP_PORT_POP3: u16 = 110;
    pub const TP_PORT_SFTP: u16 = 115;
    pub const TP_PORT_NTP: u16 = 123;

    pub const TP_PORT_SNMP: u16 = 161;
    pub const TP_PORT_SNMP_TRAP: u16 = 162;

    pub const TP_PORT_BGP: u16 = 179;
    pub const TP_PORT_LDAP: u16 = 389;
    pub const TP_PORT_HTTPS: u16 = 443;
    pub const TP_PORT_HTTPS_PCSYNC: u16 = 8443;
    pub const TP_PORT_IKE: u16 = 500;
    pub const TP_PORT_IPSEC_NAT: u16 = 4500;

    pub const TP_PORT_SYSLOG: u16 = 514;
    pub const TP_PORT_RIPNG: u16 = 521;
    pub const TP_PORT_DHCPv6_CLIENT: u16 = 546;
    pub const TP_PORT_DHCPv6_SERVER: u16 = 547;
    pub const TP_PORT_RTSP: u16 = 554; //RealTime Streaming Protocol
    pub const TP_PORT_LDAPS: u16 = 636; //
    pub const TP_PORT_NETCONF_SSH: u16 = 830; //
    pub const TP_PORT_NETCONF_HTTPS: u16 = 832; //
    pub const TP_PORT_TWAMP: u16 = 862; 
    pub const TP_PORT_FTPS: u16 = 990; // Ftp Over TLS/SSL
    pub const TP_PORT_OPEN_VPN: u16 = 1194; //
    pub const TP_PORT_L2TP: u16 = 1701; //
    pub const TP_PORT_RADIUS: u16 = 1812; //
    pub const TP_PORT_RADIUS_ACCT: u16 = 1813; //

    pub const TP_PORT_SIP: u16 = 5060;
    pub const TP_PORT_SIPS: u16 = 5061;

    pub const TP_PORT_BFD_CTRL: u16 = 3784;
    pub const TP_PORT_BFD_ECHO: u16 = 785;
    pub const TP_PORT_BFD_LAG: u16 = 6784;

    //私有的WOT隧道
    pub const TP_PORT_WOT: u16 = 38000;
}

pub fn default_socket_addr() -> SocketAddr {
    return SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 0);
}
//将一个IP地址转化为一个16字节数组
pub fn ipaddr_to_array(ip:&IpAddr)->(usize,[u8;IPV6_ADDR_LEN]) {
    let mut ip_addr:[u8;IPV6_ADDR_LEN]=[0;IPV6_ADDR_LEN];
    match ip{
        IpAddr::V4(addr)=> {
            let a = addr.octets();
            unsafe {
            std::ptr::copy(&a[0] as *const u8, &mut ip_addr[0] as *mut u8,IPV4_ADDR_LEN);
            }
            (IPV4_ADDR_LEN,ip_addr)
        }
        IpAddr::V6(addr)=> {
            (IPV6_ADDR_LEN, addr.octets())
        }
    }
}
///copy IpAddr to a specified slice
pub fn copy_ipaddr_to_slice(src:&IpAddr,buf:&mut[u8])->Result<usize,errcode::RESULT> {
    match src {
        IpAddr::V4(ip)=> {
            if buf.len()<IPV4_ADDR_LEN {
                return Err(errcode::ERROR_OUTOF_MEM)
            }
            unsafe {
                ptr::copy_nonoverlapping(ip.octets().as_ptr(), buf.as_mut_ptr(), IPV4_ADDR_LEN);
            }
            return Ok(IPV4_ADDR_LEN)
        },
        IpAddr::V6(ip)=> {
            if buf.len()<IPV6_ADDR_LEN {
                return Err(errcode::ERROR_OUTOF_MEM)
            }
            unsafe {
                ptr::copy_nonoverlapping(ip.octets().as_ptr(), buf.as_mut_ptr(), IPV6_ADDR_LEN);
            }
            return Ok(IPV6_ADDR_LEN)
        }
    }
}
//将一个数组转化为IP地址，长度为4转化为IPv4地址，16转化为IPv6地址，其它为0
pub fn bytes_array_to_ipaddr(ip:&[u8])->IpAddr {
    let ip_addr =  match ip.len() {
        IPV4_ADDR_LEN=>{
            IpAddr::from([ip[0],ip[1],ip[2],ip[3]])
        },
        IPV6_ADDR_LEN=>{
            let mut a:[u8;IPV6_ADDR_LEN]=[0;IPV6_ADDR_LEN];
            unsafe {
            std::ptr::copy(&ip[0] as *const u8, &mut a[0] as *mut u8, IPV6_ADDR_LEN);
            }
            IpAddr::from(a)
        },
        _=>IpAddr::from([0,0,0,0]),
    };
    return ip_addr;
}

pub fn ipv4_to_u32(ipv4: &Ipv4Addr) -> u32 {
    let u8a = ipv4.octets();
    return u32::from_be_bytes(u8a);
}

//将IP地址转化为两个64bit数字
pub fn ipv6_to_u64(ipv6: &Ipv6Addr) -> (u64, u64) {
    let u8a = ipv6.octets();
    let v128 = u128::from_be_bytes(u8a);

    let h64 = (v128 >> 64) as u64;
    let l64 = (v128 & 0xFFFFFFFFFFFFFFFF) as u64;
    return (l64, h64);
}

#[cfg(unix)]
#[link(name = "os_linux", kind = "static")]
extern "C" {
    pub fn tuntap_setup(fd: c_int, name: *mut u8, mode: c_int, packet_info: c_int) -> c_int;
}
#[cfg(unix)]
pub fn create_tap_if(
    name: &str,
    mtu: u16,
    ip_addrs: &[IpNetwork],
    pkt_info: bool,
) -> errcode::RESULT {
    use std::os::unix::io::{AsRawFd};

    let fd = match OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/net/tun")
    {
        Err(_) => return errcode::ERROR_OPEN_FILE,
        Ok(f) => f,
    };

    // The buffer is larger than needed, but who cares… it is large enough.
    let mut name_buffer = Vec::new();
    name_buffer.extend_from_slice(name.as_bytes());
    name_buffer.extend_from_slice(&[0; 33]);
    let name_ptr: *mut u8 = name_buffer.as_mut_ptr();
    let result = unsafe {
        tuntap_setup(
            fd.as_raw_fd(),
            name_ptr,
            2 as c_int,
            if pkt_info { 1 } else { 0 },
        )
    };
    if result < 0 {
        return errcode::ERROR_OS_CALL_FAILED;
    }
    //fd.close();
    if ip_addrs.len() > 0 {
        let addr_str = format!("{}/{}", ip_addrs[0].ip(), ip_addrs[0].prefix());
        let _ = Command::new("ifconfig")
            .args([
                name,
                "mtu",
                format!("{}", mtu).as_str(),
                "add",
                addr_str.as_str(),
                "up",
            ])
            .spawn();
    } else {
       let _= Command::new("ifconfig")
            .args([name, "mtu", format!("{}", mtu).as_str(), "up"])
            .spawn();
    }

    errcode::RESULT_SUCCESS
}

pub fn get_pnetif_by_name(name: &String) -> Result<netinterface::NetworkInterface, errcode::RESULT> {
    return netinterface::NetworkInterface::get_interface_by_name(name);
}


#[cfg(windows)]
pub fn create_tap_if(
    name: &str,
    mtu: u16,
    ip_addrs: &[ipnetwork::IpNetwork],
    _pkt_info: bool,
) -> errcode::RESULT {
    let mtu_cmd = format!("mtu={}", mtu);
    if ip_addrs.len() > 0 {
       match Command::new("netsh")
            .args([
                "interface",
                "ip",
                "set",
                "address",
                name,
                "static",
                format!("{}", ip_addrs[0].ip()).as_str(),
                format!("{}", ip_addrs[0].mask()).as_str(),
            ])
            .spawn() {
                Ok(_)=>(),
                Err(_)=> {
                    return errcode::ERROR_OS_CALL_FAILED 
                },
            }

        match Command::new("netsh")
            .args([
                "interface",
                "ip",
                "set",
                "interface",
                name,
                mtu_cmd.as_str(),
            ])
            .spawn() {
                Ok(_)=>(),
                Err(_)=> return errcode::ERROR_OS_CALL_FAILED,               
            }
    } else {
        match Command::new("netsh")
            .args([name, "mtu", format!("{}", mtu).as_str(), "up"])
            .spawn() {
                Ok(_)=>(),
                Err(_)=> return errcode::ERROR_OS_CALL_FAILED,
            }
    }

    errcode::RESULT_SUCCESS
}

