#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::rsm::{SOCK_EVENT_READ, SOCK_EVENT_CLOSE, SOCK_EVENT_NEW};
use crate::{rsm,rsm::rsm_component_t};
use crate::{net_ext::RawFdType};
use crate::common::{tsidallocator::TsIdAllocator,spin_lock::spin_lock_t};
use super::*;
use crate::common::errcode;


#[macro_export]
macro_rules! return_error {
    ($e:expr,$ids:expr,$idx:expr) => ({
        $ids.release_id($idx);
        return $e;
    })
}

const MAX_SOCKET_NUM:usize=131072;
pub(crate) struct socket_info_t {
    pub(crate) s:Socket,
    pub(crate) owner:rsm_component_t,
}

pub(crate) struct SocketPool {
    sock_ids:TsIdAllocator,
    sockets:[Option<socket_info_t>;MAX_SOCKET_NUM+1],
    capacity:usize,
    lock:spin_lock_t,
    poll_instance:poll::Poll,
}

static mut gSocketPool:Option<Box<SocketPool>>=None;
impl SocketPool {
    pub fn new()->Box<Self> {
        let ids = TsIdAllocator::new(1, MAX_SOCKET_NUM as i32);
        let poll_inst = poll::Poll::new(MAX_SOCKET_NUM);
        let pool_ptr =  unsafe {  
            std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align_unchecked(std::mem::size_of::<SocketPool>(), 1))
            as * mut SocketPool
        };
        let mut pool = unsafe { Box::from_raw(pool_ptr) };
        pool.sock_ids = ids;   
        pool.capacity = MAX_SOCKET_NUM;
        pool.lock = spin_lock_t::new();
        pool.poll_instance=poll_inst;
            

        for i in 1..MAX_SOCKET_NUM+1 {
            pool.sockets[i]=None;
        }
        println!("init socket pool success,capacity={}",MAX_SOCKET_NUM);
        return pool
    }

    
    pub fn new_socket(&mut self,sock_af:SOCKET_ADDRESS_FAMILY,sock_type:SOCKET_TYPE,proto:u8)->Result<i32,errcode::RESULT> {
        const _DEBUG:bool=true;
        let caller=match rsm::get_self_cid() {
            None=> {
                if _DEBUG {
                    rsm::rsm_component_t::new_zero()
                } else {
                    return Err(errcode::ERROR_INVALID_STATE)
                }
                
            },
            Some(c)=>c,
        };        
       
        let sid=self.sock_ids.allocate_id();
        if sid==TsIdAllocator::INVALID_ID {
            return Err(errcode::ERROR_OUTOF_MEM)
        }
        let mut sock = match Socket::new_socket(sid,sock_af,sock_type,proto) {
            Ok(s)=>s,
            Err(e)=>return_error!(Err(e),self.sock_ids,sid),
        };
        sock.set_non_blocking();
        self.add_poll_registry(&sock);
        let sck_info = socket_info_t {
            s:sock,
            owner:caller,
        };
        self.sockets[sid as usize]=Some(sck_info);

        
        return Ok(sid);
    }

    fn add_poll_registry(&mut self,sock:&Socket)->errcode::RESULT {
        return self.poll_instance.register(sock.get_raw_fd(), sock.get_socket_id() as usize, 
            SOCK_EVENT_READ | SOCK_EVENT_CLOSE,false)
    }

    fn del_poll_registry(&mut self,fd:RawFdType)->errcode::RESULT {
        return self.poll_instance.deregister(fd)
    }
    //close socket,release socket index, let drop method close the underlying socket
    pub fn close_socket(&mut self,sock_idx:i32,)->errcode::RESULT {
        if !self.check_socket_caller(sock_idx) {
            return errcode::ERROR_NO_PERMISSION
        }
        {
            let fd = match self.get_socket_by_idx(sock_idx) {
                None=>return errcode::ERROR_NOT_FOUND,
                Some(s)=>s.get_raw_fd(),
            };
            self.del_poll_registry(fd);
        }
        let ret = self.sock_ids.release_id(sock_idx);
        if ret !=errcode::RESULT_SUCCESS {
            return ret
        }
        if self.sockets[sock_idx as usize].is_none() {
            return errcode::ERROR_NOT_FOUND
        }
       
        self.sockets[sock_idx as usize]=None;
        errcode::RESULT_SUCCESS
    }

    fn check_socket_caller(&self,sock_idx:i32)->bool {
        if sock_idx>self.sock_ids.capacity() || self.sockets[sock_idx as usize].is_none() {
            return false
        }
        let caller=match rsm::get_self_cid() {
            None=>return false,
            Some(c)=>c,
        };
        if let Some(sinfo)=&self.sockets[sock_idx as usize] {
            if sinfo.owner==caller {
                return true
            } else {
                return false
            }
        }

        return false

    }

    fn is_valid_sock_id(&self,sock_id:i32)->bool {
        return sock_id>0 && sock_id as usize<=self.capacity && self.sockets[sock_id as usize].is_some()
    }

    pub(crate) fn is_tcp_server(&self,sock_id:i32)->bool {
        if !self.is_valid_sock_id(sock_id) {
            return false
        }

        return match &self.sockets[sock_id as usize] {
            None=>false,
            Some(info)=>info.s.is_tcp_server(),
        };
    }

    pub(crate) fn get_sock_binding_info(&mut self,sock_idx:i32)->Option<&mut socket_info_t> {
        if sock_idx>self.sock_ids.capacity() || self.sockets[sock_idx as usize].is_none() {
            return None
        }
        match &mut self.sockets[sock_idx as usize] {
            None=>return None,
            Some(info)=>return Some(info),
        }
    }

    fn get_socket_by_idx(&mut self,sock_idx:i32)->Option<&mut Socket> {
        if sock_idx>self.sock_ids.capacity() || self.sockets[sock_idx as usize].is_none() {
            return None
        }
        match &mut self.sockets[sock_idx as usize] {
            None=>return None,
            Some(info)=>return Some(&mut info.s),
        }
    }

    pub(crate) fn allocate_socket_id(&mut self)->Result<i32,errcode::RESULT> {
        let id = self.sock_ids.allocate_id();
        if id==TsIdAllocator::INVALID_ID {
            return Err(errcode::ERROR_OUTOF_MEM)
        }
        return Ok(id)
    }
    pub(crate) fn release_socket_id(&mut self,id:i32)->errcode::RESULT {
        return self.sock_ids.release_id(id)
    }

    ///accept a new tcp connection, as insert into the event listener
    pub(crate) fn accept(&mut self,server_idx:i32)->Result<i32,errcode::RESULT> {
        let sid = self.sock_ids.allocate_id();
        println!("[socket pool]New Tcp client connection,server_sock={},client_sock={}",server_idx,sid);
        if sid==TsIdAllocator::INVALID_ID {
            return Err(errcode::ERROR_OUTOF_MEM)
        }
        if server_idx as usize>self.capacity{            
            return_error!(Err(errcode::ERROR_NOT_FOUND),self.sock_ids,sid)
        }
        let server = match &mut self.sockets[server_idx as usize] {
            None=> {
                return_error!(Err(errcode::ERROR_NOT_FOUND),self.sock_ids,sid)
            },
            Some(s)=>s,
        };
        
        if !server.s.is_tcp_server()|| server.s.state!=SOCKET_STATE::SOCK_LISTENING {
            //println!("[socket pool]socket state error,server_idx={},state={:?}",server_idx,server.s.state);
            return_error!(Err(errcode::ERROR_INVALID_STATE),self.sock_ids,sid)
        }

        let client = match server.s.accept(sid) {
            Ok(s)=>s,
            Err(e)=>{
                 println!("accept connection error,sock_id={},ret={}",sid,e);
                 return_error!(Err(e),self.sock_ids,sid);
            },
        };
        add_poll_registry(&client,false);
        let dst= get_lb_task_id(&server.owner, sid,server.s.lb_policy);

        println!("[socketpool]New Tcp client connection,server_sock={},client_sock={},peer_addr={},raw_fd={},dst_tid={}",
            server_idx,sid,client.get_peer_addr(),client.get_raw_fd(),dst);
        let sck_info = socket_info_t {
            s:client,
            owner:dst,
        };

        self.sockets[sid as usize]=Some(sck_info);
        
        Ok(sid)
    }

    pub fn poll(&mut self,wait_msec:u32)->Option<Vec<socket_event_t>> {
        return self.poll_instance.poll(wait_msec)
    }

    pub fn get_used_count(&self)->i32 {
        self.sock_ids.used_count()
    }

    pub fn capacity(&self)->usize {
        self.capacity as usize
    }

}

///get a loadbalanced component_id by the sock_id,
fn get_lb_task_id(caller:&rsm::rsm_component_t,sock_id:i32,policy:SOCKET_LB_POLICY)->rsm_component_t {
    let mut dst=caller.clone();
    let attr=match rsm::rsm_sched::get_component_registry(caller.get_cid()) {
        None=>return dst,
        Some(a)=>a,
    };
    let inst = match policy {
        SOCKET_LB_POLICY::SOCK_LB_ALL_INSTANCE=> {
            sock_id as usize % attr.cattr.inst_num +1
        },
        SOCKET_LB_POLICY::SOCK_LB_CALLER_INSTANCE=> {
            caller.inst_id
        },
        SOCKET_LB_POLICY::SOCK_LB_EXCLUDE_CALLER_INSTANCE=>{
            if attr.cattr.inst_num<=1 {
                return dst;
            }
            let mut vec_inst=Vec::new();
            for i in 1..attr.cattr.inst_num+1 {
                if i!=caller.inst_id {
                    vec_inst.push(i);
                }
            }
            let idx = sock_id as usize %vec_inst.len();
            vec_inst[idx]            
        },
       
    };
    dst.inst_id = inst;
    return dst;
}
///创建一个Socket
pub(crate) fn new_socket(sock_af:SOCKET_ADDRESS_FAMILY,sock_type:SOCKET_TYPE,proto:u8)->Result<i32,errcode::RESULT> {
    let pool = match unsafe {&mut gSocketPool} {
        None=> return Err(errcode::ERROR_NOT_INITIALIZED),
        Some(p)=>p,
    };

    return pool.new_socket(sock_af,sock_type, proto);
}

pub(crate) fn close_socket(idx:i32)->errcode::RESULT {
    let pool = match unsafe {&mut gSocketPool} {
        None=> return errcode::ERROR_NOT_INITIALIZED,
        Some(p)=>p,
    };

    return pool.close_socket(idx);
}

fn add_poll_registry(sock:&Socket,post_event:bool)->errcode::RESULT {
    let pool = match unsafe {&mut gSocketPool} {
        None=> return errcode::ERROR_NOT_INITIALIZED,
        Some(p)=>p,
    };    
    return pool.poll_instance.register(sock.get_raw_fd(), sock.get_socket_id() as usize, 
        SOCK_EVENT_READ | SOCK_EVENT_CLOSE,post_event)
}

///init socket pool, and start socket pool thread
pub(crate) fn init_socket_pool() {
    os_sock_start();
    unsafe {
        gSocketPool = Some(SocketPool::new());
    }
    
    let _ = std::thread::spawn(pool_thread_main);
    //std::thread::Builder::new().stack_size(4*1024*1024).name("socket_pool".to_string()).spawn(pool_thread_main);
}

pub(crate) fn init_socketpool_data() {
    os_sock_start();
    unsafe {
        gSocketPool = Some(SocketPool::new());
    }   
}


///get underlying Socket instance by socket index
pub(crate) fn get_socket_by_idx<'a>(sock_id:i32)->Option<&'a mut Socket> {
    let pool = match unsafe {&mut gSocketPool} {
        None=> return None,
        Some(p)=>p,
    };
    
    return pool.get_socket_by_idx(sock_id)
}

const MAX_POLL_MSEC:u32=500;
///SocketPool main thread, poll the socket events, and send message to correspondant component
fn pool_thread_main() {

    let pool_inst = match unsafe { &mut gSocketPool} {
        None=>{
            println!("Init Socket pool failed,thread exit");
            return;
        },
        Some(p)=>p,
    };
    register_oam();
    loop {
        let events=match pool_inst.poll(MAX_POLL_MSEC) {
            None=>continue,
            Some(ev)=>ev,
        };
        if events.len()>0 {
            process_events(pool_inst,events);
        }
        
    }

}

///process socket event
/// the events for tcp listener are processed only by socketpool
/// other socket events are sent to application
fn process_events(pool:&mut SocketPool,events:Vec<socket_event_t>) {
    //println!("process events,number={}",events.len());
    for mut ev in events {
        println!("[socket pool]process events,ev={},socket_id={}",ev.event,ev.socket_id);
       
        if pool.is_tcp_server(ev.socket_id) && (ev.event & SOCK_EVENT_READ)!=0{
            let client_id = match pool.accept(ev.socket_id) {
                Err(e)=>{
                    println!("Accept Socket error,ret={},server_sock_id={}",e,ev.socket_id);
                    continue;
                },
                Ok(idx)=>idx,
            };
            //continue;
            ev.socket_id=client_id;
            ev.event=SOCK_EVENT_NEW;            
        }
        
        let sck = match pool.get_sock_binding_info(ev.socket_id) {
            None=>continue,
            Some(s)=>s,
        };
        ev.sock_type = sck.s.get_sock_type();
        let dst =  sck.owner.clone();
        //get_lb_task_id(&sck.owner, ev.sock_id, sck.s.get_lb_policy());   
        let msg = match rsm::rsm_message_t::new::<rsm::rsm_socket_event_t>(rsm::RSM_MSG_ID_SOCKET,&ev) {
            None=>continue,
            Some(m)=>m,
        };
        if (ev.event & (SOCK_EVENT_CLOSE | rsm::SOCK_EVENT_ERR))!=0 {
            pool.close_socket(ev.socket_id);
        }
        
        rsm::send_asyn_msg(&dst, msg);

    }
}

///OAM implementation
const err_not_imp:&str="not implement";
use rsm::oam;
fn register_oam() {
    let urls = ["/socket".to_string()];
    rsm::oam::RegisterOamModule(&urls, socket_oam_callback);
}

fn socket_oam_callback(op:oam::E_RSM_OAM_OP,url:&String,param:&String)->oam::oam_cmd_resp_t {
    let resp = oam::oam_cmd_resp_t::new(errcode::ERROR_NOT_SUPPORT, &err_not_imp.to_string());
    match op {
        oam::E_RSM_OAM_OP::CLI_OP_SHOW=>{
            return read_socket_stats(url, param)
        },
        _=>(),
    }

    resp
}

fn read_socket_stats(url:&String,param:&String)->rsm::oam::oam_cmd_resp_t {
    let mut resp = rsm::oam::oam_cmd_resp_t::new(errcode::ERROR_NOT_SUPPORT, &err_not_imp.to_string());
    println!("recv oam call,url={},param={}",url,param);
    let pool = match unsafe{&mut gSocketPool} {
        None=>return resp,
        Some(p)=>p,
    };

    let desc= format!("[socket pool,capacity={},used={}]",pool.capacity(), pool.get_used_count());
    resp.Description = desc;
    resp.RetCode=errcode::RESULT_SUCCESS;
    resp
}

#[cfg(windows)]
fn os_sock_start() {
    let mut wsaData = unsafe {std::mem::zeroed::<WinSock::WSAData>()};
    let wVersion=((2 as u16)<<8) | (2 as u16);
    let ret = unsafe {
        WinSock::WSAStartup(wVersion, &mut wsaData as *mut WinSock::WSAData)
    };

    if ret!=0 {
        println!("init winsock err,ret={},os_err={}",ret,std::io::Error::last_os_error());
    } else {
        println!("init winsock success,version=0x{:x}",wVersion);
    }
    
}

#[cfg(unix)]
fn os_sock_start() {
        
}