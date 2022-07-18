#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::mem;
use super::mac_addr::mac_addr_t;
use super::ipnetwork::IpNetwork;
use crate::common::rawstring;
use std::net::IpAddr;
#[cfg(unix)]
use std::collections::HashMap;
#[cfg(unix)]
use libc;
use std::io::Error;
use super::*;

#[cfg(windows)]
use windows_sys::Win32::NetworkManagement::IpHelper::{GetAdaptersAddresses,IP_ADAPTER_ADDRESSES_LH,GetAdapterIndex};
#[cfg(windows)]
use windows_sys::Win32::NetworkManagement::IpHelper::{CreateUnicastIpAddressEntry, 
    MIB_UNICASTIPADDRESS_ROW,SetIfEntry,MIB_IFROW,MIB_IF_ADMIN_STATUS_UP,MIB_IF_ADMIN_STATUS_DOWN};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock;
#[cfg(windows)]
use crate::net_ext::windows::rawsocket;

#[cfg(unix)]
use super::unix::rawsocket;
///网络接口
#[derive(Clone,Debug)]
pub struct NetworkInterface {
    index:u32,
    name:String,
    mac_addr:mac_addr_t,
    ips:Vec<IpNetwork>,
    mtu:u16,
}

impl NetworkInterface {
    pub fn new(if_index:u32,name:&String,mac_addr:&mac_addr_t,ipnet:IpNetwork,mtu:u16)->Self {
        let mut ips=Vec::new();
        ips.push(ipnet);
        return Self {
            index:if_index,
            name:name.clone(),
            mac_addr:mac_addr.clone(),
            ips:ips,
            mtu:mtu,
        };
    }
    pub fn get_sys_interfaces()->Option<Vec<NetworkInterface>> {
        return get_network_interfaces()
    }

    pub fn get_interface_by_name(ifname:&String)->Result<NetworkInterface,errcode::RESULT> {
        let ifs = match get_network_interfaces() {
            None=>return Err(errcode::ERROR_NOT_FOUND),
            Some(vifs)=>vifs,
        };

        for nif in ifs {
            if nif.name.eq(ifname) {
                return Ok(nif)
            }
        }
        Err(errcode::ERROR_NOT_FOUND)
    }

    ///获取Interface的IP列表
    pub fn get_ip_list(&self)->Vec<IpNetwork> {
        self.ips.clone()
    }
    ///获取接口的第一个IP地址
    pub fn get_first_ip(&self)->Option<IpNetwork> {
        if self.ips.len()>0 {
            return Some(self.ips[0].clone());
        }
        None
    }

    pub fn get_mtu(&self)->u16 {
        self.mtu
    }

    pub fn get_mac_addr(&self)->mac_addr_t {
        self.mac_addr
    }

    pub fn get_if_index(&self)->u32 {
        self.index
    }

    pub fn get_if_name(&self)->String {
        self.name.clone()
    }

    pub fn set_ip_addr(&mut self,ip:&IpAddr,mask_len:u8)->errcode::RESULT {

        let res = add_ip_address(self.index,&self.name,ip,mask_len);
        if res!=errcode::RESULT_SUCCESS {
            return res
        }
        match IpNetwork::new(ip.clone(), mask_len) {
              Ok(ip1)=> {
                  let mut index=usize::MAX;
                  //ipv4只允许配置一个Ip地址，进行替代
                  if ip.is_ipv4() {
                    for i in 0..self.ips.len() {                    
                        if self.ips[i].get_ip_addr().is_ipv4() {
                            index=i;
                            break;
                        }
                      }
                  }
                  
                  if index==usize::MAX {
                    self.ips.push(ip1);
                  } else {
                    self.ips[index]=ip1;
                  }
                  
           },
           Err(ec)=>return ec,
        }
        //println!("set ip success,ip={},ips={:?}",ip,self.ips);
        errcode::RESULT_SUCCESS        
    }

    pub fn set_mac_addr(&mut self,new_mac:&mac_addr_t)->errcode::RESULT {
        let res=set_macaddr_by_name(&self.name, new_mac,self.index);
        if res==errcode::RESULT_SUCCESS {
            self.mac_addr=new_mac.clone();
        }
        return res
    }

    pub fn to_string(&self)->String {
        format!("name:{},if_index:{},mac:{},mtu:{},ip:{:?}",self.name,self.index,self.mac_addr,self.mtu,self.ips)
    }
    
}

pub type Interfaces = Vec<NetworkInterface>;

#[cfg(unix)]
fn get_network_interfaces()->Option<Interfaces> {
    let mut netifs:Vec<NetworkInterface> = Vec::new();
    let mut ifmap:HashMap<String,NetworkInterface>=HashMap::new();
   
    let mut unix_ifs = std::ptr::null_mut() as *mut libc::ifaddrs;
    let res = unsafe { libc::getifaddrs(&mut unix_ifs as *mut *mut libc::ifaddrs) };
    if res<0 {
        return None;
    }
    let mut unix_if = unix_ifs;
    while !unix_if.is_null() {
        let if_ref = unsafe { &*(unix_if) };
        if if_ref.ifa_addr.is_null() {
            unix_if = if_ref.ifa_next;
            continue;
        }
        get_one_addr(&mut ifmap,if_ref);
        unix_if = if_ref.ifa_next;
    }
    
    if !unix_ifs.is_null() {
        unsafe {
        libc::freeifaddrs(unix_ifs);
        }
    }
    
    for (_,v) in ifmap {
        netifs.push(v);
    }
    Some(netifs)
}

///读取一个getifaddrs中的内容
#[cfg(unix)]
fn get_one_addr(ifmap:&mut HashMap<String,NetworkInterface>,unix_addr:&libc::ifaddrs) {
            
    let ifname = rawstring::array_to_string(unix_addr.ifa_name as *const u8, 33);
    let mut if_ip = IpAddr::from([0,0,0,0]);
    let mut if_mask = IpAddr::from([0,0,0,0]);
    let mut mac = mac_addr_t::zero();
    
    let mut af =  0;
    if !unix_addr.ifa_addr.is_null() {
        af = unsafe { (&*unix_addr.ifa_addr).sa_family as i32 };
        if af != libc::AF_INET && af!=libc::AF_INET6 && af!=libc::AF_PACKET {
            return
        }
    }
    if af== libc::AF_INET || af==libc::AF_INET6 {
        if_ip = match unsafe { rawsocket::to_socket_addr(unix_addr.ifa_addr as *const _ as *const rawsocket::sockaddr_t) } {
        Err(_)=>IpAddr::from([0,0,0,0]),
        Ok(a)=>a.ip(),
        };

        if_mask = match unsafe {rawsocket::to_socket_addr(unix_addr.ifa_netmask as *const _ as *const rawsocket::sockaddr_t)} {
        Err(_)=>IpAddr::from([0,0,0,0]),
        Ok(a)=>a.ip(),
        };
    } else if af== libc::AF_PACKET {
        let laddr =  unsafe { &(*(unix_addr.ifa_addr as *const libc::sockaddr_ll)) };
        mac = mac_addr_t::from_slice(&laddr.sll_addr);
    } 
    
    let ipnet = IpNetwork::from(if_ip,if_mask);            
    if !ifmap.contains_key(&ifname) {
        let mut ips=Vec::new();
        if af==libc::AF_INET || af==libc::AF_INET6 {
            ips.push(ipnet);
        }
        let mtu = match get_mtu_by_name(&ifname) {
            Ok(m)=>m,
            Err(_)=> {
                DEFAULT_ETHERNET_MTU
            },
        };
        let if_index = match get_ifindex_by_name(&ifname) {
            None=>0,
            Some(r)=>r,
        };
        
        let netif = NetworkInterface {
            index:if_index,
            name:ifname.clone(),
            mac_addr:mac,
            mtu:mtu,
            ips:ips,
        };
        ifmap.insert(ifname.clone(), netif);
    } else {
        if let Some(mut if_info) = ifmap.get_mut(&ifname) {
            if af==libc::AF_INET || af==libc::AF_INET6 {
                if_info.ips.push(ipnet);
            } else if af==libc::AF_PACKET {
                if_info.mac_addr = mac;
            }                
        }
        
    }
}

#[cfg(windows)]
fn get_network_interfaces()->Option<Interfaces> {
    const MAX_BUFFER_SIZE:usize=32768;
    let mut netifs:Vec<NetworkInterface> = Vec::new();
    let mut netif_buffer:Vec<u8> = Vec::with_capacity(MAX_BUFFER_SIZE);
    netif_buffer.resize(MAX_BUFFER_SIZE,0);
    unsafe {
        
    let mut buffer_len:u32=MAX_BUFFER_SIZE as u32;
    let res = GetAdaptersAddresses(WinSock::AF_UNSPEC,0,std::ptr::null_mut(),
        netif_buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
        &mut buffer_len as *mut u32);
    if res!=0 {
        return None;
    }
    
    let mut win_if = netif_buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
    while win_if != std::ptr::null_mut() {
        let if_ref = &*(win_if);
        let ifname = rawstring::unicode_str_to_string(if_ref.FriendlyName);
        let if_index =if_ref.Anonymous1.Anonymous.IfIndex;
        let mut ips:Vec<IpNetwork>=Vec::new();
        let mut paddr = if_ref.FirstUnicastAddress;
        while paddr != std::ptr::null_mut(){
            let ref_addr = &*(paddr);
            let if_ip = match rawsocket::to_socket_addr(ref_addr.Address.lpSockaddr as *const _ as *const rawsocket::sockaddr_t) {
                Err(_)=>IpAddr::from([0,0,0,0]),
                Ok(a)=>a.ip(),
            };
           
            if let Ok(ipnet) = IpNetwork::new(if_ip,ref_addr.OnLinkPrefixLength as u8) {                
                ips.push(ipnet);
            }
            
            paddr = ref_addr.Next;
        }
        let netif = NetworkInterface {
            index:if_index,
            name:ifname,
            mac_addr:mac_addr_t::from_slice(&if_ref.PhysicalAddress),
            ips:ips,
            mtu:if_ref.Mtu as u16,
        };
        netifs.push(netif);
        win_if = if_ref.Next;
    }

    }
    Some(netifs)
}

#[cfg(unix)]
use unix::rawsocket::sockaddr_t;
#[cfg(windows)]
use windows::rawsocket::sockaddr_t;
extern "C" {
    pub fn c_get_if_mac(name:*const u8, mac:*mut u8)->i32;
    pub fn c_set_if_mac(name:*const u8, mac:*const u8)->i32;
    pub fn c_get_if_mtu(name:*const u8, mtu:*mut i32)->i32;
    pub fn c_set_if_ip(name:*const u8, ip:*const sockaddr_t,mask:*const sockaddr_t)->i32;
}

#[cfg(unix)]
const MAX_NAME_LEN:usize = 33;
#[cfg(unix)]
pub fn get_macaddr_by_name(ifname:&String)->Result<mac_addr_t,errcode::RESULT> {
    
    let mut mac=[0u8;MAC_ADDR_SIZE];
    let mut name_buf = [0u8;MAX_NAME_LEN];
    if ifname.len()>=MAX_NAME_LEN {
        return Err(errcode::ERROR_INVALID_PARAM)
    }
    name_buf[0..ifname.len()].copy_from_slice(ifname.as_bytes());
    
    let res = unsafe { c_get_if_mac(name_buf.as_ptr(), mac.as_mut_ptr()) };
    if res!= 0 {
        return Err(errcode::ERROR_OS_CALL_FAILED);
    }
    Ok(mac_addr_t::from_slice(&mac))
}

#[cfg(unix)]
pub fn set_macaddr_by_name(ifname:&String,mac:&mac_addr_t,_index:u32)->errcode::RESULT {
    
    let mut name_buf = [0u8;MAX_NAME_LEN];
    if ifname.len()>=MAX_NAME_LEN {
        return errcode::ERROR_INVALID_PARAM
    }
    name_buf[0..ifname.len()].copy_from_slice(ifname.as_bytes());
    
    let res = unsafe { c_set_if_mac(name_buf.as_ptr(), mac.as_ptr()) };
    if res!= 0 {
        return errcode::ERROR_OS_CALL_FAILED;
    }
    errcode::RESULT_SUCCESS
}

#[cfg(unix)]
pub fn get_mtu_by_name(ifname:&String)->Result<u16,errcode::RESULT> {
    let mut mtu=DEFAULT_ETHERNET_MTU as i32;
    let mut name_buf = [0u8;MAX_NAME_LEN];
    if ifname.len()>=MAX_NAME_LEN {        
        return Err(errcode::ERROR_INVALID_PARAM)
    }
    name_buf[0..ifname.len()].copy_from_slice(ifname.as_bytes());

    let res = unsafe { c_get_if_mtu(name_buf.as_ptr(), &mut mtu as *mut i32) };
    if res!= 0 {
        return Err(errcode::ERROR_OS_CALL_FAILED);
    }
    Ok(mtu as u16)
}

#[cfg(unix)]
pub fn get_ifindex_by_name(ifname:&String)->Option<u32> {
    let mut name_buf = [0u8;MAX_NAME_LEN];
    if ifname.len()>=MAX_NAME_LEN {        
        return None
    }
    name_buf[0..ifname.len()].copy_from_slice(ifname.as_bytes());
    #[cfg(target_arch = "x86_64")]
    let res = unsafe { libc::if_nametoindex(name_buf.as_ptr() as *const i8) };
    #[cfg(target_arch = "aarch64")]
    let res = unsafe { libc::if_nametoindex(name_buf.as_ptr() as *const u8) };
    if res>0 {
        return Some(res);
    }
    None
}

#[cfg(windows)]
pub fn get_macaddr_by_name(_ifname:&String)->Result<mac_addr_t,errcode::RESULT> {
    Err(errcode::ERROR_NOT_SUPPORT)
}
#[cfg(windows)]
pub fn get_ifindex_by_name(name:*const u16)->Option<u32> {
    let mut index = 0;
    let res =  unsafe { GetAdapterIndex(name,&mut index as *mut u32) };
    if res==0 {
        
        return Some(index)
    } else {
        println!("ret={},err={}",res,std::io::Error::last_os_error());
        return None;
    }
}

///移除一个0结尾的字符串中"-"he ":""
fn remove_non_hex_char(slice_str:&mut [u8])->usize {
    let len = slice_str.len();
    let mut used:usize=0;
    for i in 0..len {
        if slice_str[i]==b'-' || slice_str[i]==b':' {
            continue;
        }
        slice_str[used]=slice_str[i];
        used+=1;
        if slice_str[i]==0 {
            break;
        }
    }
    used
}

#[cfg(windows)]
pub fn set_macaddr_by_name(_ifname:&String,mac:&mac_addr_t,index:u32)->errcode::RESULT {
    use windows_sys::Win32::System::Registry;
    let key_name = "NetworkAddress";
    let reg_path = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e972-e325-11ce-bfc1-08002be10318}";
    let reg_mac_key = format!("{}\\{:04}",reg_path,index);
    let mut hKey:Registry::HKEY=0;
    let mut OpRes:u32=0;
    let res = unsafe { Registry::RegCreateKeyExA(Registry::HKEY_LOCAL_MACHINE, reg_mac_key.as_ptr(), 
        0, std::ptr::null(), 0, Registry::KEY_ALL_ACCESS, 
        std::ptr::null(), &mut hKey as *mut Registry::HKEY, &mut OpRes as *mut u32) };
    if res!=0 {
        println!("open mac address registry failed,key_path={},ret={},err={}",reg_mac_key,res,Error::last_os_error());
        return errcode::ERROR_OS_CALL_FAILED;
    }
    let mut sz_mac=[0u8;18];
    let mut sz_name=[0u8;32];
    sz_mac[0..17].copy_from_slice(mac.to_string().as_bytes());
    let vlen = remove_non_hex_char(&mut sz_mac[0..17]);
    sz_name[0..key_name.len()].copy_from_slice(key_name.as_bytes());
    let res2=unsafe { Registry::RegSetValueExA(hKey, sz_name.as_ptr(), 0, Registry::REG_SZ,
             sz_mac.as_ptr(), vlen as u32) };
    if res2!=0 {
        println!("set mac address registry failed,key_path={},err={}",reg_mac_key,Error::last_os_error());
    }
    unsafe {
        Registry::RegCloseKey(hKey);
    }
    if res2==0 {
        set_interface_admin_status(index,false);
        set_interface_admin_status(index,true);
        errcode::RESULT_SUCCESS
    } else {
        errcode::ERROR_OS_CALL_FAILED
    }
    
    
}

#[cfg(windows)]
fn set_interface_admin_status(index:u32,isUp:bool)->errcode::RESULT {
    let mut if_info = unsafe { mem::zeroed::<MIB_IFROW>() };
    if_info.dwIndex = index;
    if_info.dwAdminStatus = if isUp {MIB_IF_ADMIN_STATUS_UP} else {MIB_IF_ADMIN_STATUS_DOWN};
    //rawstring::ansi_str_to_unicode(ifname.as_bytes(),&mut if_info.wszName[..]);
    let res = unsafe {SetIfEntry(&if_info as *const MIB_IFROW) };
    if res!=0 {
        println!("Set interface state error,res={},os err={},if_index={}",res,Error::last_os_error(),index);
        return errcode::ERROR_OS_CALL_FAILED
    }
    errcode::RESULT_SUCCESS
}

#[cfg(windows)]
pub fn add_ip_address(if_index:u32,_name:&String,ip:&IpAddr,mask_len:u8)->errcode::RESULT {
    let mut ip_rec:MIB_UNICASTIPADDRESS_ROW = unsafe { mem::zeroed::<MIB_UNICASTIPADDRESS_ROW>() };
    let winIp = rawsocket::rust_ipaddr_to_windows(ip);
    ip_rec.Address=winIp;
    ip_rec.OnLinkPrefixLength = mask_len;
    ip_rec.InterfaceIndex = if_index;
    ip_rec.PreferredLifetime = 0xFFFFFFFF;
    ip_rec.ValidLifetime = 0xFFFFFFFF;
    let res = unsafe { CreateUnicastIpAddressEntry(&ip_rec as *const MIB_UNICASTIPADDRESS_ROW) };
    if res!=0 {
        println!("Add Ip address failed,ip={},mask={},error={}",ip,mask_len,Error::last_os_error());
        
        return errcode::ERROR_OS_CALL_FAILED
    }
    errcode::RESULT_SUCCESS
}

#[cfg(unix)]
pub fn add_ip_address(_if_index:u32,name:&String,ip:&IpAddr,mask_len:u8)->errcode::RESULT {
    let (ip_addr,_) = sockaddr_t::from_socket_addr(&SocketAddr::new(ip.clone(), 0));
    let mask = match IpNetwork::get_ip_mask(ip.is_ipv6(), mask_len) {
        None=>return errcode::ERROR_INVALID_PARAM,
        Some(m)=>m,
    };
    let (raw_mask,_) = sockaddr_t::from_socket_addr(&SocketAddr::new(mask,0));

    let res = unsafe { c_set_if_ip(name.as_ptr(), &ip_addr as *const sockaddr_t, &raw_mask as *const sockaddr_t) };
    if res!=0 {
        return errcode::ERROR_OS_CALL_FAILED;
    }
    errcode::RESULT_SUCCESS
}