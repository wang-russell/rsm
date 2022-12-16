#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_variables)]
#![allow(dead_code)]

use super::super::*;
use libc;
use std::os::raw::{c_int,c_void};
use std::net::{UdpSocket,SocketAddr,SocketAddrV6,SocketAddrV4};
use mio::net::UdpSocket as mio_udpsocket;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd,RawFd};
use std::io::{self,Error, IoSliceMut,IoSlice};
use std::mem;

#[repr(C)]
#[derive(Clone,Copy)]
pub union sockaddr_t {
     pub v4: libc::sockaddr_in,
     pub v6: libc::sockaddr_in6,
}
        
impl sockaddr_t {
    pub fn as_ptr(&self) -> *const libc::sockaddr {
        self as *const _ as *const libc::sockaddr
    }

    pub fn from_socket_addr(addr:&SocketAddr)->(Self,libc::socklen_t){
        match addr {
            SocketAddr::V4(ipv4)=> {
                let sin_addr = libc::in_addr {
                    s_addr: u32::from_ne_bytes(ipv4.ip().octets()),
                };
                let sockaddr = libc::sockaddr_in{
                sin_family:libc::AF_INET as libc::sa_family_t,
                sin_port:ipv4.port().to_be(),
                sin_addr:sin_addr,
                sin_zero:[0;8],
                };
                return (Self{v4:sockaddr},mem::size_of::<libc::sockaddr_in>() as libc::socklen_t)
            },
            SocketAddr::V6(ipv6)=> {
                let sockaddr = libc::sockaddr_in6 {
                    sin6_family:libc::AF_INET6 as libc::sa_family_t,
                    sin6_port:ipv6.port().to_be(),
                    sin6_flowinfo:ipv6.flowinfo(),
                    sin6_addr:libc::in6_addr {
                        s6_addr: ipv6.ip().octets(),
                    },
                    sin6_scope_id:ipv6.scope_id(),
                };
                return (Self{v6:sockaddr},mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t)
            },
        }
        
    }
}

pub unsafe fn to_socket_addr(
    sock_addr: *const sockaddr_t,
) -> io::Result<SocketAddr> {
    if sock_addr==std::ptr::null() {
        return Err(io::ErrorKind::InvalidInput.into())
    }
    match (*sock_addr).v4.sin_family as libc::c_int {
        libc::AF_INET => {
            // AF_INET，采用sockaddr_in存储.
            let addr: &libc::sockaddr_in = &*(sock_addr as *const libc::sockaddr_in);
            let ip = Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes());
            let port = u16::from_be(addr.sin_port);
            Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
        }
        libc::AF_INET6 => {
            // AF_INET，采用sockaddr_in6存储
            let addr: &libc::sockaddr_in6 = &*(sock_addr as *const libc::sockaddr_in6);
            let ip = Ipv6Addr::from(addr.sin6_addr.s6_addr);
            let port = u16::from_be(addr.sin6_port);
            Ok(SocketAddr::V6(SocketAddrV6::new(
                ip,
                port,
                addr.sin6_flowinfo,
                addr.sin6_scope_id,
            )))
        }
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}

///设置socket接收缓冲区大小，以字节计算
pub fn set_socket_recvbuf(socket:RawFdType,buf_size:i32)->i32 {
    return  unsafe { 
        libc::setsockopt(socket as c_int, libc::SOL_SOCKET,
        libc::SO_RCVBUF,&(buf_size as c_int) as *const _ as *const c_void,std::mem::size_of::<i32>() as libc::socklen_t) 
    };
}

///设置socket发送缓冲区大小，以字节计算
pub fn set_socket_sendbuf(socket:RawFdType,buf_size:i32)->i32 {
    return unsafe {
        libc::setsockopt(socket as c_int, libc::SOL_SOCKET,
        libc::SO_SNDBUF,&(buf_size as c_int) as *const _ as *const c_void,std::mem::size_of::<i32>() as libc::socklen_t)
    };
}

///设置socket reuse addr参数，1=使用，0-不使用
pub fn set_socket_reuse_addr(socket:RawFdType,is_reuse:i32)->i32 {
    return unsafe {
        libc::setsockopt(socket as c_int, libc::SOL_SOCKET,
        libc::SO_REUSEADDR,&(is_reuse as c_int) as *const _ as *const c_void,std::mem::size_of::<i32>() as libc::socklen_t)
    };
}

///设置socketreuse port参数，1=使用，0-不使用
pub fn set_socket_reuse_port(socket:RawFdType,is_reuse:i32)->i32 {
    return unsafe {
        libc::setsockopt(socket as c_int, libc::SOL_SOCKET,
        libc::SO_REUSEPORT,&(is_reuse as c_int) as *const _ as *const c_void,std::mem::size_of::<i32>() as libc::socklen_t)
    };
}
///获得一个mio udp_socket的raw socket句柄
pub fn get_raw_socket(socket:&mio_udpsocket)->RawFdType {
    return socket.as_raw_fd();
}

///send_udp_msg，使用raw_fd作为入参，发送Message
pub fn send_udp_msg(fd:RawFdType,buf:&[u8],addr:&SocketAddr)->io::Result<usize> {
      
    unsafe {
        let (sockaddr,len) = sockaddr_t::from_socket_addr(addr);  
        let res= libc::sendto(fd as libc::c_int,buf.as_ptr() as  *const c_void,buf.len() as libc::size_t,0,
            sockaddr.as_ptr(),len) as usize;

            if res>0 {
                return Ok(res as usize)
            } else {
                return Err(Error::last_os_error())
            }
    }
}

///wait_for_single_fd_read，等待一个文件的读事件
/// fd:原始文件句柄,timeout_msec，超时毫秒数
pub fn wait_for_single_fd_read(fd:RawFdType,timeout_msec:i32)->Result<(),errcode::RESULT> {
    let mut fds = libc::pollfd{
        fd:fd as libc::c_int,
        events:libc::POLLIN,
        revents:0,
    };
    match unsafe { libc::poll(&mut fds as *mut libc::pollfd,1 as libc::nfds_t,timeout_msec as libc::c_int) } {
        0=>Err(errcode::ERROR_TIME_OUT),
        1=>Ok(()),
        _=>Err(errcode::ERROR_OS_CALL_FAILED),
    }
}

//对Windows/Linux原生网络接口的封装
//从而支持原生Socket接口的跨平台
pub fn create_rawsocket(is_l2_socket:bool)->RawFdType {
    let sock_type=if is_l2_socket {libc::SOCK_RAW} else {libc::SOCK_DGRAM};
    
    return unsafe { libc::socket(libc::AF_PACKET, sock_type, i32::from((libc::ETH_P_ALL as u16).to_be())) }
}

///根据network name获得if_index
pub fn get_netif_index_by_name(name: &str) -> Result<i32,errcode::RESULT> {
    if name.len() > libc::IFNAMSIZ {
        return Err(errcode::ERROR_INVALID_PARAM);
    }
    let mut buf = [0u8; libc::IFNAMSIZ];
        buf[..name.len()].copy_from_slice(name.as_bytes());
    let idx = unsafe { libc::if_nametoindex(buf.as_ptr() as *const libc::c_char) };
    if idx == 0 {
        return Err(errcode::ERROR_NOT_FOUND);
    }

    Ok(idx as i32)
}

///绑定指定index的接口，一般用于AF_PACKET类型的Socket
pub fn bind_by_index(fd:RawFdType,ifindex: i32) -> errcode::RESULT {
    unsafe {
        let mut ss: libc::sockaddr_storage = std::mem::zeroed();
        let sll: *mut libc::sockaddr_ll = &mut ss as *mut libc::sockaddr_storage as *mut libc::sockaddr_ll;
        (*sll).sll_family = libc::AF_PACKET as u16;
        (*sll).sll_protocol = (libc::ETH_P_ALL as u16).to_be();
        (*sll).sll_ifindex = ifindex;

        let sa = (&ss as *const libc::sockaddr_storage) as *const libc::sockaddr;
        let res = libc::bind(fd, sa, std::mem::size_of::<libc::sockaddr_ll>() as u32);
        if res == -1 {
            return errcode::ERROR_BIND_SOCKET;
        }
        let ignore_pkt:i32 = 1;
        const PACKET_IGNORE_OUTGOING:i32=23;
        libc::setsockopt(fd,libc::SOL_PACKET,PACKET_IGNORE_OUTGOING,
              &ignore_pkt as *const i32 as *const libc::c_void,std::mem::size_of::<i32>().try_into().unwrap());
    }

    errcode::RESULT_SUCCESS
}

pub fn bind(fd:RawFdType,addr:&SocketAddr)->errcode::RESULT {
    let (os_addr,len)=sockaddr_t::from_socket_addr(addr);

    let res = unsafe { libc::bind(fd,std::ptr::addr_of!(os_addr) as *const libc::sockaddr, len) };
    if res!=0 {
        println!("bind socket addr={},os_err={}",addr,std::io::Error::last_os_error());
        return errcode::ERROR_BIND_SOCKET
    }
    return errcode::RESULT_SUCCESS

}
pub fn listen(fd:RawFdType,back_log:i32)->errcode::RESULT {

    let res = unsafe { libc::listen(fd,back_log) };
    if res!=0 {
        return errcode::ERROR_BIND_SOCKET
    }
    return errcode::RESULT_SUCCESS

}

pub fn accept(fd:RawFdType)->Result<(RawFdType,SocketAddr),errcode::RESULT> {
    let mut sock_addr = unsafe { mem::zeroed::<sockaddr_t>() };
    let mut len=std::mem::size_of::<sockaddr_t>() as u32;
    let res = unsafe { libc::accept(fd,std::ptr::addr_of_mut!(sock_addr) as *mut libc::sockaddr,&mut len as * mut u32) };
    if res<0 {
        println!("[rawsocket]accept connection error,ret={},os_err={}",res,std::io::Error::last_os_error());
        return Err(errcode::ERROR_OS_CALL_FAILED)
    }
    let addr=match unsafe { to_socket_addr(std::ptr::addr_of!(sock_addr)) } {
        Err(_)=>return Err(errcode::ERROR_INVALID_IPADDR),
        Ok(a)=>a,
    };
    return Ok((res as RawFdType,addr))

}

pub fn connect(fd:RawFdType,dst:&SocketAddr)->errcode::RESULT {
    let (addr,len)=sockaddr_t::from_socket_addr(dst);
    let ret = unsafe {
        libc::connect(fd, std::ptr::addr_of!(addr) as *const libc::sockaddr, len)
    };

    if ret==0 {
        return errcode::RESULT_SUCCESS
    }
    return errcode::ERROR_OS_CALL_FAILED
}

///设置网卡的混杂模式
pub fn set_promisc_mode(fd:RawFdType,if_idx: i32, state: bool) ->errcode::RESULT {
    let packet_membership = if state {
        libc::PACKET_ADD_MEMBERSHIP
    } else {
        libc::PACKET_DROP_MEMBERSHIP
    };

    unsafe {
        let mut mreq: libc::packet_mreq = std::mem::zeroed();

        mreq.mr_ifindex = if_idx;
        mreq.mr_type = libc::PACKET_MR_PROMISC as u16;

        let res = libc::setsockopt(fd, libc::SOL_PACKET, packet_membership, 
            (&mreq as *const libc::packet_mreq) as *const libc::c_void, std::mem::size_of::<libc::packet_mreq>() as u32);
        if res == -1 {
            return errcode::ERROR_OS_CALL_FAILED;
        }
    }

    errcode::RESULT_SUCCESS
}

///read_fd，从文件句柄中读取一段数据
pub fn read_fd(fd: RawFd, buf: &mut [u8],flags:i32) -> Result<usize,errcode::RESULT> {
    let rv = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    if rv < 0 {
        return Err(errcode::ERROR_RECV_MSG);
    }

    Ok(rv as usize)
}

///write_fd，向文件句柄中写入一段数据
pub fn write_fd(fd: RawFd, buf: &[u8],flags:i32) -> Result<usize,errcode::RESULT> {
    let rv = unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len().try_into().unwrap()) };
    if rv < 0 {
        //println!("send packet error:{} ",Error::last_os_error());
        return Err(errcode::ERROR_RECV_MSG);
    }

    Ok(rv as usize)
}

pub fn send_to(fd: RawFdType, buf: &[u8],flags:i32,dst:&SocketAddr) -> Result<usize,errcode::RESULT> {
    let (addr,len)=sockaddr_t::from_socket_addr(dst);
    let rv = unsafe { 
        libc::sendto(fd, buf.as_ptr() as *const libc::c_void, buf.len(), flags,
        std::ptr::addr_of!(addr) as *const libc::sockaddr, len)
    };
    if rv < 0 {
        return Err(errcode::ERROR_SEND_MSG);
    }

    Ok(rv as usize)
}

///read_fd_vector，从文件中批量读取一批数据
#[cfg(unix)]
pub fn read_fd_vector(fd: RawFd, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
    let rv = unsafe { libc::readv(fd, bufs.as_mut_ptr() as *const libc::iovec, bufs.len() as i32) };
    if rv < 0 {
        return Err(Error::last_os_error());
    }

    Ok(rv as usize)
}

///write_fd_vector，向文件中批量写入一批数据
pub fn write_fd_vertor(fd: RawFd, bufs: &[IoSlice]) -> io::Result<usize> {
    let rv = unsafe { libc::writev(fd, bufs.as_ptr() as *const libc::iovec, bufs.len() as i32) };
    if rv < 0 {
        return Err(Error::last_os_error());
    }

    Ok(rv as usize)
}

    /// set_non_blocking 设置文件系统为Non-blocking模式.
    pub fn set_non_blocking(fd:RawFd) -> io::Result<()> {
        unsafe {
            let mut res = libc::fcntl(fd, libc::F_GETFL);
            if res != -1 {
                res = libc::fcntl(fd, libc::F_SETFL, res | libc::O_NONBLOCK);
            }
            if res == -1 {
                return Err(Error::last_os_error());
            }
        }
        Ok(())
    }


///close_fd，关闭文件句柄
pub fn close_fd(fd:RawFdType) {
    unsafe {
    libc::close(fd);
    }
}

///recv_from,接收Socket消息，返回大小和对端地址
pub fn recv_from(fd: RawFd, buf: &mut [u8]) -> io::Result<(usize,SocketAddr)> {
    //println!("begin recv {} msg,content={}",bufs_count,bufs[0].to_string());
    unsafe { 
    let mut addr=mem::zeroed::<sockaddr_t>();
    let mut addr_len = mem::size_of::<sockaddr_t>() as u32;
    let rv = 
        libc::recvfrom(fd, buf.as_mut_ptr() as *mut libc::c_void, 
            buf.len(),0,&mut addr as *mut _ as *mut libc::sockaddr, &mut addr_len as *mut u32);
    
    if rv < 0 {
        let err = Error::last_os_error();
        //println!("recv msg error:{}-{},content={:?}",rv,err,bufs);
        return Err(err);
    }
    let sockaddr = match to_socket_addr(&addr as *const sockaddr_t) {
        Ok(a)=>a,
        Err(_)=>return Err(Error::last_os_error()),
    };
    Ok((rv as usize,sockaddr))
}
}


//pub type iovec = libc::iovec;

#[repr(C)]
#[derive(Copy,Clone)]
pub struct iovec {
    pub iov_base: *mut u8, 
    pub iov_len: usize,   
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
/// 
#[derive(Debug)]
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
#[derive(Debug)]
#[repr(C)]
pub struct mmsg_hdr_t {
    pub msg_hdr: msg_hdr_t,
    pub msg_len: u32,
}

impl mmsg_hdr_t {
    ///创建一个为recvmmsg/sendmmsg使用的mmsghdr头部
    /// capacity为最大消息数量,buffs缓冲区
   
    pub fn new(capacity:usize, buffs:&mut [iovec],addr:&mut sockaddr_t)->Self {        

        //let hdr = 
        return mmsg_hdr_t{
            msg_hdr:msg_hdr_t {
                msg_name: addr as * mut sockaddr_t,
                msg_namelen: std::mem::size_of::<sockaddr_t>() as u32,
                msg_iov: buffs.as_mut_ptr(),
                msg_iovlen:capacity,
                msg_control:std::ptr::null_mut(),
                msg_control_len:0,
                msg_flags:0
            },
            msg_len:0,
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
            let mut content = Vec::with_capacity(64);
            std::ptr::copy((*self.msg_hdr.msg_iov).iov_base, content.as_mut_ptr(), 32);
            content.set_len(32);
            format!("msg_len={},msg_name={:?},msg_namelen={},msg_iov={:?},msg_iov_len={},msg_control:{:?},msg_control_len={},
            content={:?}",
            self.msg_len,self.msg_hdr.msg_name,self.msg_hdr.msg_namelen,self.msg_hdr.msg_iov,
            self.msg_hdr.msg_iovlen,self.msg_hdr.msg_control,self.msg_hdr.msg_control_len,
            content.as_slice())
            }
    }
}

///recv_from_mmsg,从Socket批量接收一批报文，成功则返回接收的报文数量
pub fn recv_from_mmsg(fd: RawFd, bufs: &mut [mmsg_hdr_t],bufs_count:u32) -> io::Result<usize> {
    //println!("begin recv {} msg,content={}",bufs_count,bufs[0].to_string());
    let rv = unsafe { 
        libc::recvmmsg(fd, bufs.as_mut_ptr() as *mut mmsg_hdr_t as *mut u8 as *mut libc::mmsghdr, 
            bufs_count,0,std::ptr::null_mut() as *mut libc::timespec) 
    };
    if rv < 0 {
        let err = Error::last_os_error();
        //println!("recv msg error:{}-{},content={:?}",rv,err,bufs);
        return Err(err);
    }
 
    Ok(rv as usize)
}

///send_mmsg，从Socket批量发送一批报文，类型由mmsg_hdr_t决定
pub fn send_mmsg(fd: RawFd, bufs: &mut mmsg_hdr_t,bufs_count:u32) -> io::Result<usize> {
    let rv = unsafe { libc::sendmmsg(fd, bufs as *mut mmsg_hdr_t as *mut u8 as *mut libc::mmsghdr, 
        bufs_count,0) };
    if rv < 0 {
        return Err(Error::last_os_error());
    }

    Ok(rv as usize)
}
