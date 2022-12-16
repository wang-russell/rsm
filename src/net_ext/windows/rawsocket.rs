#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_variables)]
#![allow(dead_code)]

use super::super::*;
use std::net::{UdpSocket,SocketAddr,SocketAddrV4,SocketAddrV6};
use mio::net::UdpSocket as mio_udpsocket;
use std::{mem,sync::Once};
use crate::common::{errcode};
//#[cfg(windows)]
//use std::os::windows::io::{RawHandle,AsRawHandle};

#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::{self,AF_INET,AF_INET6,SOCKADDR,SOCKADDR_IN,SOCKADDR_IN6,
IN_ADDR,IN_ADDR_0,IN6_ADDR,IN6_ADDR_0,SOCKADDR_IN6_0};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket,RawSocket};

#[cfg(windows)]
use windows_sys::core::{PSTR,PCSTR};

use std::io::{self,Error, IoSliceMut,IoSlice};


pub(crate) fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // Let standard library call `WSAStartup` for us, we can't do it
        // ourselves because otherwise using any type in `std::net` would panic
        // when it tries to call `WSAStartup` a second time.
        drop(std::net::UdpSocket::bind("127.0.0.1:0"));
    });
}

#[repr(C)]
#[derive(Clone,Copy)]
pub union sockaddr_t{
    pub v4:SOCKADDR_IN,
    pub v6:SOCKADDR_IN6,
    pub si_family:u16,
}
impl sockaddr_t {
    pub fn as_ptr(&self) -> *const SOCKADDR {
        self as *const _ as *const SOCKADDR
    }

    ///从socketAddr转换为
    pub fn from_socket_addr(addr:&SocketAddr)->(Self,i32){
        match addr {
            SocketAddr::V4(ref addr) => {
                // `s_addr` is stored as BE on all machine and the array is in BE order.
                // So the native endian conversion method is used so that it's never swapped.
                let sin_addr = unsafe {
                    let mut s_un = mem::zeroed::<IN_ADDR_0>();
                    s_un.S_addr = u32::from_ne_bytes(addr.ip().octets());
                    IN_ADDR { S_un: s_un }
                };
    
                let sockaddr_in = SOCKADDR_IN {
                    sin_family: AF_INET as u16, // 1
                    sin_port: addr.port().to_be(),
                    sin_addr,
                    sin_zero: [0; 8],
                };
    
                let sockaddr = sockaddr_t { v4: sockaddr_in };
                (sockaddr, std::mem::size_of::<SOCKADDR_IN>() as i32)
            }
            SocketAddr::V6(ref addr) => {
                let sin6_addr = unsafe {
                    let mut u = std::mem::zeroed::<IN6_ADDR_0>();
                    u.Byte = addr.ip().octets();
                    IN6_ADDR { u }
                };
                let u = unsafe {
                    let mut u = std::mem::zeroed::<SOCKADDR_IN6_0>();
                    u.sin6_scope_id = addr.scope_id();
                    u
                };
    
                let sockaddr_in6 = SOCKADDR_IN6 {
                    sin6_family: AF_INET6 as u16, // 23
                    sin6_port: addr.port().to_be(),
                    sin6_addr,
                    sin6_flowinfo: addr.flowinfo(),
                    Anonymous: u,
                };
    
                let sockaddr = sockaddr_t { v6: sockaddr_in6 };
                (sockaddr, std::mem::size_of::<SOCKADDR_IN6>() as i32)
            }
        }
    }
}


pub(crate) unsafe fn to_socket_addr(
    sock_addr: *const sockaddr_t,
) -> io::Result<SocketAddr> {
    if sock_addr==std::ptr::null() {
        return Err(io::ErrorKind::InvalidInput.into())
    }
    match (*sock_addr).v4.sin_family as WinSock::ADDRESS_FAMILY {
        WinSock::AF_INET => {
            // AF_INET，采用sockaddr_in存储.
            let addr: &WinSock::SOCKADDR_IN = &*(sock_addr as *const WinSock::SOCKADDR_IN);
            let ip = Ipv4Addr::from(addr.sin_addr.S_un.S_addr.to_ne_bytes());
            let port = u16::from_be(addr.sin_port);
            Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
        }
        WinSock::AF_INET6 => {
            // AF_INET，采用sockaddr_in6存储
            let addr: &WinSock::SOCKADDR_IN6 = &*(sock_addr as *const WinSock::SOCKADDR_IN6);
            let ip = Ipv6Addr::from(addr.sin6_addr.u.Byte);
            let port = u16::from_be(addr.sin6_port);
            Ok(SocketAddr::V6(SocketAddrV6::new(
                ip,
                port,
                addr.sin6_flowinfo,
                addr.Anonymous.sin6_scope_id,                
            )))
        }
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}


///wait_for_single_fd_read,等待一个文件的读事件
pub fn wait_for_single_fd_read(fd:RawFdType,timeout_msec:i32)->Result<(),errcode::RESULT> {
    let mut fds = WinSock::WSAPOLLFD{
        fd:fd as usize,
        events:WinSock::POLLIN as i16,
        revents:0,
    };
    match unsafe { WinSock::WSAPoll(&mut fds as *mut WinSock::WSAPOLLFD, 1, timeout_msec) } {
        0=>Err(errcode::ERROR_TIME_OUT),
        1=>Ok(()),
        _=>Err(errcode::ERROR_OS_CALL_FAILED),
    }
}

///send_udp_msg,发送一个UDP消息
pub fn send_udp_msg(fd:RawFdType,buf:&[u8],addr:&SocketAddr)->io::Result<usize> {
    let (sockaddr,len) = sockaddr_t::from_socket_addr(addr);    
    unsafe {
        let res= WinSock::sendto(fd as usize,buf.as_ptr() as PCSTR,buf.len() as i32,0,
        sockaddr.as_ptr() ,len);
        if res>0 {
            return Ok(res as usize)
        } else {
            return Err(Error::last_os_error())
        }
    }
}

///获取mio UdpSoket文件的句柄
pub fn get_raw_socket(socket:&mio_udpsocket)->RawFdType {
    return socket.as_raw_socket();
}

#[cfg(windows)]
pub fn set_socket_recvbuf(socket:RawFdType,buf_size:i32)->i32 {
    unsafe {
    return WinSock::setsockopt(socket as usize, WinSock::SOL_SOCKET as i32, WinSock::SO_RCVBUF as i32, &buf_size as *const i32 as windows_sys::core::PCSTR,4);
    }
    
}

#[cfg(windows)]
pub fn set_socket_sendbuf(socket:RawFdType,buf_size:i32)->i32 {
    unsafe {
    return WinSock::setsockopt(socket as usize, WinSock::SOL_SOCKET as i32, WinSock::SO_SNDBUF as i32, &buf_size as *const i32 as windows_sys::core::PCSTR,4);
    }
    
}

///设置socket reuse addr参数，1=使用，0-不使用
pub fn set_socket_reuse_addr(socket:RawFdType,is_reuse:i32)->i32 {
        -1
}
pub fn set_socket_reuse_port(socket:RawFdType,is_reuse:i32)->i32 {
    -1
}
//对Windows/Linux原生网络接口的封装
//从而支持原生Socket接口的跨平台
#[cfg(windows)]
pub fn create_rawsocket(is_l2_socket:bool)->RawFdType {
    unsafe {
    WinSock::socket(WinSock::AF_INET as i32,WinSock::SOCK_RAW as i32,0) as RawFdType
    }
}


#[cfg(windows)]
pub fn get_netif_index_by_name(name: &str) -> Result<i32,errcode::RESULT> {
    Ok(1)
}

#[cfg(windows)]
pub fn bind_by_index(fd:RawFdType,ifindex: i32) -> errcode::RESULT {
    errcode::RESULT_SUCCESS
}

pub fn bind(fd:RawFdType,addr:&SocketAddr)->errcode::RESULT {
    let (os_addr,len)=sockaddr_t::from_socket_addr(addr);

    let res = unsafe { WinSock::bind(fd as usize,std::ptr::addr_of!(os_addr) as *const SOCKADDR, len) };
    if res!=0 {
        println!("bind socket addr={},os_err={}",addr,std::io::Error::last_os_error());
        return errcode::ERROR_BIND_SOCKET
    }
    return errcode::RESULT_SUCCESS

}
pub fn listen(fd:RawFdType,back_log:i32)->errcode::RESULT {

    let res = unsafe { WinSock::listen(fd as usize,back_log) };
    if res!=0 {
        return errcode::ERROR_BIND_SOCKET
    }
    
    return errcode::RESULT_SUCCESS

}

pub fn accept(fd:RawFdType)->Result<(RawFdType,SocketAddr),errcode::RESULT> {
    let mut sock_addr = unsafe { mem::zeroed::<sockaddr_t>() };
    let mut len=std::mem::size_of_val(&sock_addr) as i32;
    let res = unsafe {        
        WinSock::accept(fd as usize,std::ptr::addr_of_mut!(sock_addr) as *mut SOCKADDR,&mut len as * mut i32) 
    };
    let addr=match unsafe { to_socket_addr(std::ptr::addr_of!(sock_addr)) } {
        Err(_)=>return Err(errcode::ERROR_INVALID_IPADDR),
        Ok(a)=>a,
    };
    return Ok((res as RawFdType,addr))

}

pub fn connect(fd:RawFdType,dst:&SocketAddr)->errcode::RESULT {
    let (addr,len)=sockaddr_t::from_socket_addr(dst);
    let ret = unsafe {
        WinSock::connect(fd as usize, std::ptr::addr_of!(addr) as *const SOCKADDR, len)
    };

    if ret==0 {
        return errcode::RESULT_SUCCESS
    }
    return errcode::ERROR_OS_CALL_FAILED
}

#[cfg(windows)]
pub fn set_promisc_mode(fd:RawFdType,if_idx: i32, state: bool) ->errcode::RESULT {
    errcode::RESULT_SUCCESS
}

//从Socket中读取一段内容
#[cfg(windows)]
pub fn read_fd(fd: RawSocket, buf: &mut [u8],flags:i32) -> Result<usize,errcode::RESULT> {
    let rv = unsafe { 
        WinSock::recv(fd as usize, buf.as_mut_ptr() as PSTR , buf.len() as i32, flags) 
    };
    if rv < 0 {
        return Err(errcode::ERROR_RECV_MSG);
    }

    Ok(rv as usize)
}

//从Socket中读取一段内容
#[cfg(windows)]
pub fn write_fd(fd: RawSocket, buf: &[u8],flags:i32) -> Result<usize,errcode::RESULT> {
    let rv = unsafe { 
        WinSock::send(fd as usize, buf.as_ptr() as PCSTR, buf.len() as i32, flags)
    };
    if rv < 0 {
        return Err(errcode::ERROR_SEND_MSG);
    }

    Ok(rv as usize)
}

pub fn send_to(fd: RawFdType, buf: &[u8],flags:i32,dst:&SocketAddr) -> Result<usize,errcode::RESULT> {
    let (addr,len)=sockaddr_t::from_socket_addr(dst);
    let rv = unsafe { 
        WinSock::sendto(fd as usize, buf.as_ptr() as PCSTR, buf.len() as i32, flags,
        std::ptr::addr_of!(addr) as *const SOCKADDR, len)
    };
    if rv < 0 {
        return Err(errcode::ERROR_SEND_MSG);
    }

    Ok(rv as usize)
}

///read_fd_vector，从文件中批量读取一批数据
#[cfg(windows)]
pub fn read_fd_vector(fd: RawSocket, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
    
    Ok(0)
}

#[cfg(windows)]
pub fn write_fd_vertor(fd: RawSocket, bufs: &[IoSlice]) -> io::Result<usize> {
    Ok(0)
}

    // 设置文件系统为Non-blocking模式.
    #[cfg(windows)]
    pub fn set_non_blocking(fd:RawSocket) -> io::Result<()> {       
        let mut iMode:u32=1;
        let res = unsafe { WinSock::ioctlsocket(fd as usize, WinSock::FIONBIO,&mut iMode as *mut u32) };
        if res!=0 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }
#[cfg(windows)]
pub fn close_fd(fd:RawFdType) {
    unsafe {
    WinSock::closesocket(fd as usize);
    }
}


///recv_from,接收Socket消息，返回大小和对端地址
pub fn recv_from(fd: RawSocket, buf: &mut [u8]) -> io::Result<(usize,SocketAddr)> {
    //println!("begin recv {} msg,content={}",bufs_count,bufs[0].to_string());
    unsafe { 
    let mut addr=mem::zeroed::<sockaddr_t>();
    let mut addr_len = mem::size_of::<sockaddr_t>() as i32;
    let rv = 
        WinSock::recvfrom(fd as usize, buf as *mut _ as windows_sys::core::PSTR, buf.len() as i32, 0, 
        &mut addr as *mut _ as *mut SOCKADDR, &mut addr_len as *mut i32);
    if rv < 0 {
        let err = Error::last_os_error();
        //println!("recv msg error:{}-{},buf_len={}",rv,err,buf.len());
        return Err(err);
    }
    let sockaddr = match to_socket_addr(&addr as *const sockaddr_t) {
        Ok(a)=>a,
        Err(_)=>return Err(Error::last_os_error()),
    };
    Ok((rv as usize,sockaddr))
}
}

#[repr(C)]
#[derive(Copy,Clone)]
pub struct iovec {
    pub iov_len: usize,
    pub iov_base: *mut u8,    
}
impl iovec {
    pub fn from_raw_parts(buf_ptr:*mut u8,len:usize)->iovec {
        return Self{
            iov_base:buf_ptr,
            iov_len:len,
        }
    }

    pub fn from_slice(buf:&mut[u8])->iovec {
        return Self{
            iov_base:buf.as_mut_ptr(),
            iov_len:buf.len(),
        }
    }
    pub fn to_slice(&mut self)->&[u8] {
        let p = unsafe { std::slice::from_raw_parts_mut(self.iov_base, self.iov_len) };
        return p
    }
}

///批量发送接收Socket消息，send_mmsg/recv_mmsg，采用Linux兼容格式，Windows适配Linux格式
#[repr(C)]
pub struct msg_hdr_t {
    pub msg_name: *mut sockaddr_t,
    pub msg_namelen: u32,
    pub msg_iov: *mut iovec,
    pub msg_iovlen: usize,
    pub msg_control: *mut u8,
    pub msg_control_len: usize,
    pub msg_flags: i32,
}
#[repr(C)]
pub struct mmsg_hdr_t {
    pub msg_hdr: msg_hdr_t,
    pub msg_len: u32,
}

impl mmsg_hdr_t {
    ///创建一个为recvmmsg/sendmmsg使用的mmsghdr头部
    /// capacity为最大消息数量,buffs缓冲区
    /// 
   
    pub fn new(capacity:usize, buffs:&mut [iovec],addr:&mut sockaddr_t)->Self {        

        //let hdr = 
        return mmsg_hdr_t{
            msg_hdr:msg_hdr_t {
                msg_name: addr as * mut sockaddr_t,
                msg_namelen:std::mem::size_of::<sockaddr_t>() as u32,
                msg_iov: buffs.as_mut_ptr(),
                msg_iovlen:capacity,
                msg_control:std::ptr::null_mut(),
                msg_control_len:0,
                msg_flags:0
            },
            msg_len:0 as u32,
        }
    }

    pub fn get_nth_buf_len(&self,idx:usize)->usize {
        if idx>=self.msg_hdr.msg_iovlen {
            return 0;
        }
        let buf= unsafe {
            &*((self.msg_hdr.msg_iov as usize + mem::size_of::<iovec>()*idx) as *const iovec)
        };
        return buf.iov_len;
        
    }
    pub fn to_string(&self)->String {
        unsafe {
        format!("msg_len={},msg_name={:?},msg_namelen={},msg_iov={:?},msg_iov_len={},msg_control:{:?},msg_control_len={},recv_addr={}",
        self.msg_len,self.msg_hdr.msg_name,self.msg_hdr.msg_namelen,self.msg_hdr.msg_iov,
        self.msg_hdr.msg_iovlen,self.msg_hdr.msg_control,self.msg_hdr.msg_control_len,
        if self.msg_len>0 { to_socket_addr(self.msg_hdr.msg_name).unwrap() } else {std::mem::zeroed::<SocketAddr>()})
        }
    }
}


///recv_from_mmsg：从Windows Socket批量接收一批报文
pub fn recv_from_mmsg(fd: RawSocket, bufs: &mut mmsg_hdr_t,bufs_count:u32) -> io::Result<usize> {
    let mut addr_len:i32=mem::size_of::<sockaddr_t>() as i32;

    //let mut res = 0;
    let mut count=0;
    let mut bytes=0;
    for i in 0..bufs_count {
        unsafe {
        let vec_buf = &mut *((bufs.msg_hdr.msg_iov as usize + i as usize*mem::size_of::<iovec>()) as *mut iovec);
        let sock_ptr = (bufs.msg_hdr.msg_name as usize + i as usize*addr_len as usize) as *mut SOCKADDR;
        let res = WinSock::recvfrom(fd as usize, vec_buf.iov_base as windows_sys::core::PSTR, 
            vec_buf.iov_len as i32, 0, bufs.msg_hdr.msg_name as *mut SOCKADDR, &mut addr_len);
       
         if res<0 {
            break;
         } else {
            vec_buf.iov_len = res as usize;
            count+=1;
            bytes+=res;
         }
        }
    }

    if count <= 0 {
        return Err(Error::last_os_error());
    }
    bufs.msg_len = bytes as u32;
    Ok(count as usize)
}

//从Windows Socket批量发送一批报文
#[cfg(windows)]
pub fn send_mmsg(fd: RawSocket, bufs: &mmsg_hdr_t,bufs_count:u32) -> io::Result<usize> {
    let mut bytesSent = 0u32;
    let addr_len:i32=(bufs_count as usize*mem::size_of::<sockaddr_t>()) as i32;
    let rv = unsafe {  
        WinSock::WSASendTo(fd as usize, bufs.msg_hdr.msg_iov as *const WinSock::WSABUF, bufs_count, 
        &mut bytesSent as  *mut u32, 0, 
        bufs.msg_hdr.msg_name as  *mut WinSock::SOCKADDR, addr_len as i32, 
        std::ptr::null_mut(), None) };
    if rv < 0 {
        return Err(Error::last_os_error());
    }

    Ok(rv as usize)
}

///将一个Rust格式的IP地址转换为Windows In Addr格式
pub fn rust_ipaddr_to_windows(ip:&IpAddr)->WinSock::SOCKADDR_INET {
    let (stub_sockaddr,_)=sockaddr_t::from_socket_addr(&SocketAddr::new(ip.clone(),0));
    return unsafe { *(&stub_sockaddr as *const sockaddr_t as *const _ as *const WinSock::SOCKADDR_INET) }
}

///将一个Windows格式的IP地址转换为Rust IpAddr格式
pub fn windows_ipaddr_to_rust(ip:&WinSock::SOCKADDR_INET)->IpAddr {
    match unsafe { to_socket_addr(ip as *const WinSock::SOCKADDR_INET as *const _ as *const sockaddr_t) }{
        Ok(addr)=>addr.ip(),
        Err(_)=>IpAddr::from([0,0,0,0]),
    }

}
