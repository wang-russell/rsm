#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::common::{errcode, spin_lock};
use crate::net_ext::{RawFdType};
use super::*;
use crate::rsm::{self};
use core::ffi::c_void;
use crate::rsm::{SOCK_EVENT_READ, SOCK_EVENT_WRITE, SOCK_EVENT_CLOSE,SOCK_EVENT_ERR};

#[cfg(windows)]
use crate::net_ext::{windows::rawsocket};
#[cfg(unix)]
use crate::net_ext::{unix::rawsocket};
#[cfg(windows)]
use windows_sys::Win32::System::IO;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{ERROR_SUCCESS};

#[cfg(unix)]
use libc;
#[cfg(windows)]
use windows_sys::Win32::System::IO::OVERLAPPED;

pub struct Poll {
    poll_fd:RawFdType,
    capacity:usize,
    count:usize,
    lock:spin_lock::spin_lock_t,
}

impl Poll {
    pub fn new(capacity:usize)->Self {
        let h = create_poll_inst(capacity as i32);
        return Self { poll_fd: h,
         capacity:capacity,
         count:0,
         lock:spin_lock::spin_lock_t::new(),
         }
    }

    pub fn register(&mut self,fd:RawFdType,key:usize,event:rsm::SOCKET_EVENT,post_event:bool)->errcode::RESULT {
        self.lock.lock();
        let ret = poll_add_socket(self.poll_fd, fd, key, event);
        if ret==errcode::RESULT_SUCCESS {
            self.count+=1;
            println!("register socket event success,socket_id={},event={},ret={},current_count={}",key,event,ret,self.count);
        } else {
            println!("register socket event failed,socket_id={},event={},ret={},current_count={}",key,event,ret,self.count);
        }
       
        self.lock.unlock();
        if post_event {
            self.post_event(fd, key, rsm::SOCK_EVENT_READ);
        }
        return ret
    }
    #[cfg(windows)]
    fn post_event(&self,fd:RawFdType,key:usize,event:rsm::SOCKET_EVENT) {
        let os_ev=rsmev_to_osev(event);
        let mut ol = unsafe { std::mem::zeroed::<OVERLAPPED>() };
        ol.hEvent =  fd as isize;
        unsafe {
        IO::PostQueuedCompletionStatus(self.poll_fd as isize, os_ev,
                 key, &ol as *const _);
        }
    }

    #[cfg(unix)]
    fn post_event(&self,_fd:RawFdType,_key:usize,_event:rsm::SOCKET_EVENT) {
    }

    pub fn deregister(&mut self,fd:RawFdType)->errcode::RESULT {
        self.lock.lock();
        let ret = poll_del_socket(self.poll_fd, fd);
        if ret==errcode::RESULT_SUCCESS {
            self.count-=1;
        }
        self.lock.unlock();
        return ret
    }

    pub fn poll(&mut self,wait_msec:u32)->Option<Vec<socket_event_t>> {
        return poll_wait(self.poll_fd, wait_msec)
    }

    pub fn capacity(&self)->usize {
        self.capacity
    }

    pub fn used(&self)->usize {
        self.count
    }

}

impl Drop for Poll {
    fn drop(&mut self) {
        #[cfg(unix)]
        rawsocket::close_fd(self.poll_fd);
        #[cfg(windows)]
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.poll_fd as isize) };
    }
}

#[cfg(windows)]
fn create_poll_inst(_capacity:i32)->RawFdType {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

   let h = unsafe { IO::CreateIoCompletionPort(INVALID_HANDLE_VALUE, 0, 
    0, 1) };
    println!("[poll]create IOCP success,port={}",h);
   return h as RawFdType;
}

#[cfg(unix)]
fn create_poll_inst(capacity:i32)->RawFdType {
   return unsafe {libc::epoll_create(capacity)}
}

#[cfg(windows)]
fn poll_add_socket(inst:RawFdType,fd:RawFdType,key:usize,events:u32)->errcode::RESULT {
    //let mut len=0u32;
    let mut sock_reg=unsafe { std::mem::zeroed::<WinSock::SOCK_NOTIFY_REGISTRATION>() };
    sock_reg.completionKey = key as *mut c_void;
    sock_reg.operation = WinSock::SOCK_NOTIFY_OP_ENABLE as u8;
    sock_reg.socket = fd as usize;
    sock_reg.eventFilter = rsmev_to_osev(events) as u16;
    sock_reg.triggerFlags = (WinSock::SOCK_NOTIFY_TRIGGER_LEVEL | WinSock::SOCK_NOTIFY_TRIGGER_PERSISTENT) as u8;
    let ret = unsafe {
        WinSock::ProcessSocketNotifications(inst as isize, 1, 
            &mut sock_reg as * mut WinSock::SOCK_NOTIFY_REGISTRATION, 0, 
            0, 
            std::ptr::null_mut(), std::ptr::null_mut())
    };
    // >=Vec::with_capacity(MAX_WAIT_EVENTS_NUM);
    
    /*let ret = unsafe { IO::CreateIoCompletionPort(fd as isize, inst as isize, 
        key, 1) }; */

    if ret==ERROR_SUCCESS && sock_reg.registrationResult==ERROR_SUCCESS 
    {
        println!("[poll]add socket to IOCP success,IOCP={},sock_fd={},ret={},",inst,fd,ret);
        return errcode::RESULT_SUCCESS
    }
    
    println!("[poll]add socket event error,iocp={},fd={},events={},ret={},reg_result={},key={},os_err={}",
        inst,fd,sock_reg.eventFilter,ret,sock_reg.registrationResult,key,
        unsafe {WinSock::WSAGetLastError()});
    return errcode::ERROR_OS_CALL_FAILED

   
}

#[cfg(unix)]
fn poll_add_socket(inst:RawFdType,fd:RawFdType,key:usize,events:u32)->errcode::RESULT {
    let mut ev = unsafe { std::mem::zeroed::<libc::epoll_event>() };
    ev.events = rsmev_to_osev(events) |  libc::EPOLLET as u32;
    ev.u64 = key as u64;
    let ret = unsafe { libc::epoll_ctl(inst,libc::EPOLL_CTL_ADD, fd,
        &mut ev as *mut libc::epoll_event) };
    if ret!=0 {
        return errcode::ERROR_OS_CALL_FAILED
    }
    errcode::RESULT_SUCCESS
}

#[cfg(windows)]
fn poll_del_socket(_inst:RawFdType,_fd:RawFdType)->errcode::RESULT {
    errcode::RESULT_SUCCESS
 }

#[cfg(unix)]
fn poll_del_socket(inst:RawFdType,fd:RawFdType)->errcode::RESULT {
    let mut ev = unsafe { std::mem::zeroed::<libc::epoll_event>() };
    ev.events = (libc::EPOLLIN | libc::EPOLLRDHUP) as u32;
    unsafe { libc::epoll_ctl(inst,libc::EPOLL_CTL_DEL, fd,
        &mut ev as *mut libc::epoll_event) };    
    errcode::RESULT_SUCCESS
 }

 const MAX_WAIT_EVENTS_NUM:usize=128;
 #[cfg(windows)]
fn poll_wait(inst:RawFdType,wait_msec:u32)->Option<Vec<socket_event_t>> {
    use windows_sys::Win32::System::IO::OVERLAPPED_ENTRY;

    let mut len=0u32;
    //let mut sock_reg:Vec<WinSock::SOCK_NOTIFY_REGISTRATION>=Vec::with_capacity(MAX_WAIT_EVENTS_NUM);
    let mut lpoverlapped:Vec<OVERLAPPED_ENTRY> = Vec::with_capacity(MAX_WAIT_EVENTS_NUM);

    unsafe { 
        lpoverlapped.set_len(MAX_WAIT_EVENTS_NUM)
    }
    let ret = unsafe {
        WinSock::ProcessSocketNotifications(inst as isize, 0, 
            std::ptr::null_mut(), wait_msec, MAX_WAIT_EVENTS_NUM as u32, 
            lpoverlapped.as_mut_ptr(), &mut len as *mut u32)      
        /*IO::GetQueuedCompletionStatusEx(inst as isize, lpoverlapped.as_mut_ptr(), 
        MAX_WAIT_EVENTS_NUM as u32,&mut len as *mut u32,
        wait_msec,1)*/
    
    };
    if len==0 || ret!=0 {
    //(ret!=ERROR_SUCCESS && ret !=WAIT_TIMEOUT) 
        //println!("pool event failed,ret={},os_error={}",ret,std::io::Error::last_os_error());
        return None
    }
    {
    let handle=if lpoverlapped[0].lpOverlapped.is_null() {0} else {
        unsafe { (*lpoverlapped[0].lpOverlapped).hEvent }
    };
    println!("[poll]poll event success,ret={},event_count={},ret_ev={},handle={}",ret,len,
        lpoverlapped[0].dwNumberOfBytesTransferred,handle);
    }
    let mut evs:Vec<socket_event_t>=Vec::new();
    for i in 0..len as usize {
        let ol=&lpoverlapped[i];
        let os_ev = ol.dwNumberOfBytesTransferred;
        let sock_ev = socket_event_t {
            socket_id:ol.lpCompletionKey as i32,
            sock_type:SOCKET_TYPE::PROTO_DGRAM,
            event:osev_to_rsmev(os_ev), 
        };
        evs.push(sock_ev);
    }
    return Some(evs)
 }

#[cfg(unix)]
fn poll_wait(fd:RawFdType,wait_msec:u32)->Option<Vec<socket_event_t>> {

    let mut events:Vec<libc::epoll_event> = Vec::with_capacity(MAX_WAIT_EVENTS_NUM);
    unsafe { 
        //keys.set_len(128);
        events.set_len(MAX_WAIT_EVENTS_NUM)
    }
    let ret = unsafe { libc::epoll_wait(fd,events.as_mut_ptr(),MAX_WAIT_EVENTS_NUM as i32,wait_msec as i32) };
    if ret<=0 {
        return None
    }
    let mut evs:Vec<socket_event_t>=Vec::new();
    for i in 0..ret as usize {
        let ev = &events[i];
        let sock_ev = socket_event_t {
            socket_id:ev.u64 as i32,
            sock_type:SOCKET_TYPE::PROTO_DGRAM,
            event:osev_to_rsmev(ev.events as i32),
        };
        evs.push(sock_ev);
    }

    return Some(evs)
 }


 #[cfg(windows)]
 fn rsmev_to_osev(ev:u32)->u32 {
    let mut os_ev = 0u32;
    if ev & rsm::SOCK_EVENT_READ != 0 {
        os_ev |= WinSock::SOCK_NOTIFY_REGISTER_EVENT_IN;
    }    

    if ev & rsm::SOCK_EVENT_WRITE != 0 {
        os_ev |= WinSock::SOCK_NOTIFY_REGISTER_EVENT_OUT;
    }

    if ev & rsm::SOCK_EVENT_CLOSE != 0 {
        os_ev |= WinSock::SOCK_NOTIFY_REGISTER_EVENT_HANGUP;
    }

    return os_ev
  }
 
 #[cfg(unix)]
 fn rsmev_to_osev(ev:u32)->u32 {
    
    let mut os_ev = 0i32;
    if ev & SOCK_EVENT_READ != 0 {
        os_ev |= libc::EPOLLIN;
    }
    if ev & SOCK_EVENT_WRITE != 0 {
        os_ev |= libc::EPOLLOUT;
    }  

    if ev & SOCK_EVENT_CLOSE != 0 {
        os_ev |= libc::EPOLLRDHUP;
    }
    return os_ev as u32
}

 #[cfg(windows)]
 fn osev_to_rsmev(ev:u32)->rsm::SOCKET_EVENT {

    let mut rsm_ev = 0u32;
    if ev & WinSock::SOCK_NOTIFY_EVENT_IN !=0 {
        rsm_ev |= SOCK_EVENT_READ
    }
    if ev & WinSock::SOCK_NOTIFY_EVENT_OUT !=0 {
        rsm_ev |= SOCK_EVENT_WRITE
    }
    if ev & WinSock::SOCK_NOTIFY_EVENT_HANGUP !=0 {
        rsm_ev |= SOCK_EVENT_CLOSE
    }
    if ev & WinSock::SOCK_NOTIFY_EVENT_ERR !=0 {
        rsm_ev |= SOCK_EVENT_ERR
    }
    return rsm_ev
  }
 
 #[cfg(unix)]
 fn osev_to_rsmev(ev:i32)->rsm::SOCKET_EVENT {
    let mut rsm_ev = 0u32;
    if ev & libc::EPOLLIN != 0 {
        rsm_ev |= SOCK_EVENT_READ;
    }
    if ev & libc::EPOLLOUT != 0 {
        rsm_ev |= SOCK_EVENT_WRITE;
    }  

    if ev &  libc::EPOLLRDHUP!= 0 {
        rsm_ev |= SOCK_EVENT_CLOSE;
    }
    if ev & libc::EPOLLERR!=0 {
        rsm_ev |= SOCK_EVENT_ERR
    }

    return rsm_ev
}

#[cfg(test)]
mod tests {
    use std::net::{SocketAddr};
    use super::*;
    use super::socketpool;
    #[test]
    fn test_rsm_poll() {
       socketpool::init_socketpool_data();

        let mut p=Poll::new(128);
        let addr1=SocketAddr::new("0.0.0.0".parse().unwrap(), 14010);
        let lis=match TcpListener::new(&addr1, 1024, SOCKET_LB_POLICY::SOCK_LB_ALL_INSTANCE) {
            Ok(s)=>s,
            Err(e)=>{
                println!("Create TCP Listener failed,ret={},local_addr={}",e,addr1);
                assert!(false);
                return
            },
        };
        let s1=match socket(SOCKET_ADDRESS_FAMILY::SOCKET_INET, SOCKET_TYPE::PROTO_DGRAM, net_ext::IpProtos::Ip_Proto_UDP) {
            Ok(s)=>s,
            Err(e)=>{
                assert_eq!(e,errcode::RESULT_SUCCESS);
                return
            }
        };
        let ret = p.register(s1, 20000 as usize, rsm::SOCK_EVENT_READ | rsm::SOCK_EVENT_CLOSE,false);
        println!("Register event listener,ret={},socket={}",ret,s1);
        assert_eq!(ret,errcode::RESULT_SUCCESS);
    }
}