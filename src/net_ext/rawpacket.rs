#![allow(unused_variables)]
#![allow(dead_code)]

#[cfg(unix)]
use libc;

use std::io::{self,Error, Read, Write};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
#[cfg(windows)]
//use std::os::windows::io::{AsRawSocket,RawSocket};
#[cfg(unix)]
use libc::{sockaddr_ll, sockaddr_storage, socket, packet_mreq, setsockopt};

//#[cfg(windows)]
//use windows_sys::Win32::Networking::WinSock::{self,AF_INET,SOCK_RAW};

use std::io::{IoSlice,IoSliceMut};
#[cfg(unix)]
use super::unix::rawsocket;
#[cfg(windows)]
use super::windows::rawsocket;

use crate::common::{errcode};
use super::*;

#[derive(Clone,Default,Debug)]
pub struct Config {
    pub is_l2_socket:bool,
    pub recv_outgoing_pkt:bool,
    pub promiscuous:bool,
    pub non_blocking:bool,
    pub read_timeout:u32, //毫秒
    pub write_timeout:u32, //毫秒
    pub read_buf_size:i32,
    pub write_buf_size:i32,//字节
    pub mtu:u16,
}

pub struct RawPacket {
    fd:RawFdType,
    if_name:String,
    if_idx:i32,
    config:Config,
}
impl RawPacket {
    pub fn new(ifname:&str,config:&Config) -> Result<Self,errcode::RESULT> {
        let fd = rawsocket::create_rawsocket(config.is_l2_socket);
        if fd <=0 {
            return Err(errcode::ERROR_BIND_SOCKET);
        }
        let idx = match rawsocket::get_netif_index_by_name(ifname) {
            Ok(id)=>id,
            Err(ec)=>return Err(ec),
        };
        rawsocket::bind_by_index(fd,idx);
        if config.non_blocking {
            let _ =rawsocket::set_non_blocking(fd);
        }
        if config.promiscuous {
            rawsocket::set_promisc_mode(fd, idx, config.promiscuous);
        }
        let mut rp = Self{
            fd:fd,
            if_name:String::from(ifname),
            if_idx:idx,
            config:config.clone(),
        };
        if rp.config.mtu==0 {
            rp.config.mtu=DEFAULT_ETHERNET_MTU;
        }
        Ok(rp)
    }

    /// Bind socket to an interface (by name).
    pub fn bind(&mut self, name: &str) -> errcode::RESULT {
        self.bind_internal(name)
    }

    // should take an &mut to unsure not just anyone can call it,
    // but async wrapper needs this variant
    pub fn bind_internal(&self, name: &str) -> errcode::RESULT {
        let idx = match rawsocket::get_netif_index_by_name(name) {
            Ok(id)=>id,
            Err(_)=>return errcode::ERROR_NOT_FOUND,
        };
        rawsocket::bind_by_index(self.fd,idx)
    }
    //等待是否可读
    pub fn wait_read_event(&self,timeou_msec:i32)->Result<(),errcode::RESULT> {
        #[cfg(unix)]
        return rawsocket::wait_for_single_fd_read(self.fd, timeou_msec);
        #[cfg(windows)]
        return Ok(());
    }
    pub fn recv_packet(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        return self.read(buf);
    }
    pub fn recv_packet_batch(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        return self.read_vectored(bufs);
    }

    pub fn send_packet(&mut self, buf: &[u8]) -> io::Result<usize> {
        return self.write(buf);
    }

    pub fn send_packet_batch(&mut self, bufs: &mut [IoSlice]) -> io::Result<usize> {
        return self.write_vectored(bufs);
    }

    pub fn drain(&mut self) {
        self.drain_internal()
    }

    pub(crate) fn drain_internal(&self) {
        let mut buf = [0u8; 10];
        loop {
            match rawsocket::read_fd(self.fd, &mut buf[..], 0)  {
                Err(_)=>break,
                Ok(0)=>break,
                Ok(_)=>(),
            }
        }
    }

}

impl std::fmt::Display for RawPacket{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"socket_fd:{:#x},name:{},if_index:{},config:{:?}",self.fd,self.if_name,self.if_idx,self.config)
    }
}

impl Read for RawPacket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match rawsocket::read_fd(self.fd, buf,0) {
            Ok(len)=>Ok(len),
            Err(_)=>Err(Error::last_os_error()),
        }
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        match rawsocket::read_fd_vector(self.fd, bufs) {
            Ok(len)=>Ok(len),
            Err(_)=>Err(Error::last_os_error()),
        }
    }
}


impl<'a> Read for &'a RawPacket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match rawsocket::read_fd(self.fd, buf,0)  {
            Ok(len)=>Ok(len),
            Err(_)=>Err(Error::last_os_error()),
        }
    }
}


impl Write for RawPacket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut cur_len:usize = 0;
        let mut max_len = std::cmp::min(self.config.mtu as usize,buf.len());
        while cur_len<buf.len() {
            match rawsocket::write_fd(self.fd, &buf[cur_len..max_len],0)  {
                Ok(len)=> {
                    cur_len+=len;
                    max_len = std::cmp::min(max_len+len as usize,buf.len());
                },
                Err(_)=>return Err(Error::last_os_error()),
            }
           
        }
        return Ok(cur_len)
       
    }

    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        match rawsocket::write_fd_vertor(self.fd, bufs) {
            Ok(len)=>Ok(len),
            Err(_)=>Err(Error::last_os_error()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> Write for &'a RawPacket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match rawsocket::write_fd(self.fd, buf,0) {
            Ok(len)=>Ok(len),
            Err(_)=>Err(Error::last_os_error()),
        }
    }


    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
impl IntoRawFd for RawPacket {
    fn into_raw_fd(self) -> RawFd {
        self.fd
    }
}

#[cfg(unix)]
impl AsRawFd for RawPacket {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

#[cfg(unix)]
impl Drop for RawPacket {
    fn drop(&mut self) {
      rawsocket::close_fd(self.fd);
    }
}