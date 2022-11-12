#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

//! # RSM
//! RSM = Realtime Software Middleware
//! Introduction
//! ===
//! Realtime system is defined as a system that can response the external request in certain deterministic time. To achieve this goal in generic computer systems, we must adopt a realtime shcedule policy on the software system, and keep from some time-consuming operation such as synchronous I/O operation, memory garbage collection and lock.
//!
//! RSM is a lightweight realtime middleware implementation written in rust, support event-driven, message oriented lock-free programming principle. in RSM, every software module is a **component**, which is normally a Finite State Machine, mainly proccess event loop. Each component can be instantiated to several tasks, and each task mapped to a dedicated **OS thread** and has its own message queue.
//!
//! Developer can set the task's schedule priority and their message queue length respectively,usually based on the service model and performance & latency requirements.
//!
//! RSM is suitable for the following applications:
//! ----
//! - network device control plane, e.g. routing protocol, service control
//! - embedded system application
//! - remote control system
//! - realtime telemetry and instrumentation
//!
//! Programming
//! ===
//!
//! Concept
//! ---
//!
//! each RSM component must implement the **rsm::Runnable** trait and provides a task creation Callback function.
//!
//! the code in *main.rs* is a sample RSM application implementation.
//!
//! pub trait Runnable {
//!
//!    fn on_init(&mut self,cid:&rsm_component_t);
//!
//!    fn on_timer(&mut self,cid:&rsm_component_t,timer_id:rsm_timer_id_t,timer_data:usize);
//!
//!    fn on_message(&mut self,cid:&rsm_component_t,msg_id:rsm_message_id_t,msg:&rsm_message_t);
//!
//!    fn on_close(&mut self,cid:&rsm_component_t);
//!
//! }
//!
//! *type rsm_new_task=fn(cid:&rsm_component_t)->&'static mut dyn Runnable*
//!
//!
//! Initialize the RSM
//! ---
//! using *rsm_init* function to init the rsm system, then the applicaition can register their components to RSM.
//!
//! rsm_init_cfg_t is the RSM's configuration file, which is in json format.
//! rsm_init(conf:&config::rsm_init_cfg_t)->errcode::RESULT
//!
//! *pub fn registry_component(cid:u32,attrs:&component_attrs_t,callback:rsm_new_task)->errcode::RESULT*
//!
//! After the component registration is finished, the *start_rsm()* function should be called to running the system.
//!
//!Runtime
//!---
//!every running task can be identified uniquely by **rsm_component_t**
//!
//!task can send message to each other, with normal message or a high priority message
//!*pub fn send_asyn_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESUL*
//!
//! *pub fn send_asyn_priority_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT*
//!
//! for the receiver side, the application use msg.decode::<T>(v) to restore the message to application defined type
//!
//! RSM also provides a timer service, application can set timer simply by calling **set_timer** function, once the timer is set and expired, rsm task will receive a on_timer event, which is defined in the Runnable trait.
//!
//! *pub fn set_timer(dur_msec:u64,loop_count:u64,timer_data:usize)->Option<rsm_timer_id_t>*
//! *pub fn kill_timer_by_id(timer_id:rsm_timer_id_t)->errcode::RESULT*
//!
//! Diagnostic
//! ===
//! Developer and user can use rest api get running status and statistics
//!
//! Built in api
//! ---
//! help,*curl http://127.0.0.1:12000/rsm/help*
//! get task running status, *curl http://127.0.0.1:12000/rsm/task?1:2*
//! get component configuration,*curl http://127.0.0.1:12000/rsm/component?1*
//! 
//! Application defined OAM API
//! ---
//! application Module must implement *OamReqCallBack* function, and invoke *RegisterOamModule* to register self
//! *OamReqCallBack=fn(op:E_RSM_OAM_OP,url:&String,param:&String)->oam_cmd_resp_t*
//! 
//! register a module callback, urls is a list of rest api url, the prefix /rsm and id following a "?" are not included
//! *RegisterOamModule(urls:&[String], callback:OamReqCallBack)*
//! 
//! Other service& lib function
//! ===
//! xlog service
//! ---
//! xlog service is based on client/server architecture, the client side simple send log message to the server which responsible for log file manipulation, keeping from write disk under the application's context, which is very important for the realtime application.
//! 
//! *let log = rsm::new_xlog(module_name:&str)->xlog::xlogger_t;*
//! 
//! *log.Errorf(postion, err, logDesc);*
//! 
//! Other thread safe algorithm and data structure
//! ---
//! + spin_lock_t, Atomic operation based lock.
//! + AtomicQueue, based on spin_lock
//! + TsIdAllocator, thread safe Id allocator
//! + bitmap
//! + ethernet packet parser
//! + Ip routing table
//! + several other network function and object wrapper
//! 
 
use crate::common::{self,errcode};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr,SocketAddr};
use serde_json;

pub mod task;
pub mod rsm_sched;
pub mod rsm_timer;
pub mod os_timer;
pub mod socket;
pub mod config;
pub mod xlog;
pub mod oam;

const MAX_COMPONENT_NUM:usize = 256;
pub const RSM_MODULE_NAME: &str = "rust_rsm";
///RSM common Type definition
pub type rsm_component_id_t = u32;
pub type rsm_node_id_t = u32;
///timer id type
pub type rsm_timer_id_t = i32;

pub type rsm_message_id_t = u32;

///system & user cid scope definition
pub const RSM_INVALID_CID:u32 = 0;
///start of the CID reserved for system use
pub const RSM_SYSTEM_CID_START:u32 = 1;
///end of the CID reserved for system use
pub const RSM_SYSTEM_CID_END:u32 = 1023;
pub const RSM_USER_CID_START:u32 = 1024;
///maximum instance number per cid
pub const RSM_MAX_INST_PER_CID:usize=16;
///allowed max message queue len
pub const RSM_MAX_QUEUE_LEN:usize = 16384;
///allowed max message length
pub const RSM_MAX_MESSAGE_LEN:usize = 64000;

/// describe the task schedule priority, the REALTIME Priority is mapped to Linux/Windows Realtime priority
#[derive(Copy,Clone,PartialEq,Debug,Eq,Serialize)]
pub enum E_RSM_TASK_PRIORITY {
    THREAD_PRI_LOW = 0,
	THREAD_PRI_NORMAL = 1,
	THREAD_PRI_HIGH = 2,
	THREAD_PRI_REALTIME = 3,
	THREAD_PRI_REALTIME_HIGH = 4,
    THREAD_PRI_REALTIME_HIGHEST = 5,
}

/// identifier for a software module running instance, include the software module unique id and an instance id
/// in RSM, every software module running instance(component instance or task) is a Finite State Machine(FSM),
///  which mapped to an OS native thread, process message event loop
#[derive(Eq,PartialEq,Hash,Clone,Debug,Copy)]
pub struct rsm_component_t {
    cid:rsm_component_id_t,
    node_id:rsm_node_id_t,
    inst_id:usize,
}

impl rsm_component_t {
    pub fn new(id:rsm_component_id_t,node_id:u32,inst_id:usize)->Self {
        return Self {
            cid:id,
            node_id,
            inst_id,
        }
    }
    pub fn new_zero()->Self {
        return Self { cid: 0, node_id: 0, inst_id: 0 }
    }
    pub fn get_cid(&self)->rsm_component_id_t {
        self.cid
    }
    pub fn get_inst_id(&self)->usize {
        self.inst_id
    }
}

impl std::fmt::Display for rsm_component_t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(node_id={}, cid={},inst_id={})", self.node_id,self.cid, self.inst_id)
    }
}

///socket event definition
pub type SOCKET_EVENT=u32;
///socket is readable
pub const SOCK_EVENT_READ:SOCKET_EVENT= 1;
///socket is writable
pub const SOCK_EVENT_WRITE:SOCKET_EVENT= 1<<1;
///new socket created, usually a tcp client connection
pub const SOCK_EVENT_NEW:SOCKET_EVENT= 1<<2;
//Error Connection
pub const SOCK_EVENT_ERR:SOCKET_EVENT= 1<<3;
///socket has been closed by remote peer
pub const SOCK_EVENT_CLOSE:SOCKET_EVENT= 1<<4;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct rsm_socket_event_t {
    pub socket_id:i32,
    pub sock_type:socket::SOCKET_TYPE,
    pub event:SOCKET_EVENT,
    
}
///Task create callback function, which must return a valid object reference implement **Runnale** trait
type rsm_new_task=fn(cid:&rsm_component_t)->&'static mut dyn Runnable;
///Component must implement the Runnable Trait
pub trait Runnable {
    ///task init, called first when the task instance is created
    fn on_init(&mut self,cid:&rsm_component_t);
    /// called when a timer expiry event occured, timer_id indicate which timer fired
    fn on_timer(&mut self,cid:&rsm_component_t,timer_id:rsm_timer_id_t,timer_data:usize);
    /// socket event, if the task use rsm socket to send/recv message
    /// upon recv this message, task should use correspondant Upd/Tcp/Raw Socket to recv packet, util no more packet
    /// rsm automatically accept the tcp connection request from client, the notify the app, app can close the socket to reject the connection
    fn on_socket_event(&mut self,cid:&rsm_component_t,event:rsm_socket_event_t);

    ///an ordinary message received, the app should call msg.decode method to get original data structure
    fn on_message(&mut self,cid:&rsm_component_t,msg_id:rsm_message_id_t,msg:&rsm_message_t);
    ///task has been destroyed, reserved for future use
    fn on_close(&mut self,cid:&rsm_component_t);
}

/// describe the component attribute while register to the RSM
#[derive(Eq,PartialEq,Clone,Serialize)]
pub struct component_attrs_t {
    pub cid:rsm_component_id_t,    
    pub name:String,
    pub inst_num:usize, //实例数量
    pub qlen:usize,
    pub priority:E_RSM_TASK_PRIORITY,
    pub need_init_ack:bool,
}

impl component_attrs_t {
    pub fn new(cid:&rsm_component_id_t,name:&str,inst_num:usize,qlen:usize,prio:E_RSM_TASK_PRIORITY,need_init_ack:bool)->Self {
        return Self {
            cid:cid.clone(),    
            name:String::from(name),
            inst_num:inst_num, //实例数量
            qlen:qlen,
            priority:prio,
            need_init_ack:need_init_ack,        
        }
    }
    
}

///begin of the rsm message id using by system
pub const RSM_SYS_MESSAGE_ID_START:u32 = 1;
///end of the rsm message id using by system
pub const RSM_SYS_MESSAGE_ID_END:u32 = 8191;
///user message ID start, application should use message id large than this value
pub const RSM_USER_MESSAGE_ID_START:u32 = 8192;
pub const RSM_INVALID_MESSAGE_ID:u32 = 0;

///predefined rsm system message for inner use, application should not use these message id
pub const RSM_MSG_ID_MASTER_POWER_ON:u32 = 1;
pub const RSM_MSG_ID_SLAVE_POWER_ON:u32 = 2;
pub const RSM_MSG_ID_POWER_ON_ACK:u32 = 3;
pub const RSM_MSG_ID_POWER_OFF:u32 = 4;
pub const RSM_MSG_ID_TIMER:u32 = 10;
pub const RSM_MSG_ID_SOCKET:u32 = 12;

///message object
#[derive(Clone,Debug)]
pub struct rsm_message_t {
    msg_id:u32,
    timer_id:rsm_timer_id_t,
    timer_data:usize,
    sender:rsm_component_t,
    msg_body:String,
}
impl rsm_message_t {
    pub fn new<'de,T>(msg_id:rsm_message_id_t,body:&T)->Option<rsm_message_t> 
    where T:Sized+Serialize+Deserialize<'de> {
        let msg_body = match serde_json::to_string(body) {
            Ok(s)=>s,
            Err(_)=>return None,
        };
        let sender = match get_self_cid() {
            None=>rsm_component_t::new_zero(),
            Some(c)=>c,
        };
        let msg=Self {
            msg_id:msg_id,
            timer_id:0,
            timer_data:0,
            sender:sender,
            msg_body:msg_body,
        };
        return Some(msg);
    }

    pub(crate) fn new_timer_msg(timer_id:rsm_timer_id_t,timer_data:usize)->Option<rsm_message_t> {
        let sender = match get_self_cid() {
            None=>rsm_component_t::new_zero(),
            Some(c)=>c,
        };
        let msg=Self {
            msg_id:RSM_MSG_ID_TIMER,
            timer_id:timer_id,
            timer_data:timer_data,
            sender:sender,
            msg_body:String::default(),
        };
        return Some(msg);
    }
    /// on the receiving side, using decode to restore the original data format
    pub fn decode<'a,T>(&'a self)->Option<T>
    where T:Deserialize<'a> {
        match serde_json::from_slice::<T>(self.msg_body.as_bytes()) {
            Ok(v)=>Some(v),
            Err(_)=>None,
        }
    }
}
static mut gRsmConfig:Option<config::rsm_init_cfg_t>=None;
///initialize rsm subsystem, which should be called before register any component
pub fn rsm_init(conf:&config::rsm_init_cfg_t)->errcode::RESULT {
    unsafe {
    if gRsmConfig.is_some() {
        return errcode::ERROR_ALREADY_EXIST
    }
    gRsmConfig=Some(conf.clone());
    }
    oam::init_oam(&conf.oam_server_addr, &conf.log_config.self_addr);
    rsm_sched::init_scheduler(conf.max_component_num);
    rsm_timer::init_timer();
    //let mut log_conf = xlog::log_service_config_t::new_default();
    
    xlog::xlog_server::InitLogService(&conf.log_config);
    socket::socketpool::init_socket_pool();
    errcode::RESULT_SUCCESS
}

///after application initialize RSM and register all their running component, then invoke start_rsm
pub fn start_rsm() {
    println!("Start RSM, current={}",common::format_datetime(&std::time::SystemTime::now()));
    std::thread::spawn(|| rsm_timer::start_timer_thread());
    std::thread::spawn(|| rsm_sched::run());
    
}

///Register a component to RSM, with the configuration is specified by attrs parameter
/// callback is a TASK creation call back function, which is invoke by RSM before schedule the task instance
pub fn registry_component(cid:u32,attrs:&component_attrs_t,callback:rsm_new_task)->errcode::RESULT {
    return rsm_sched::registry_component(cid, attrs, callback)
}

/// get self component id
pub fn get_self_cid()->Option<rsm_component_t>{
    return rsm_sched::get_self_cid();
}

/// get the sender cid under the message receive context
pub fn get_sender_cid()->Option<rsm_component_t>{
    return rsm_sched::get_sender_cid()
}

///power_on or init_ack, to keep task start order, not implement yet
pub fn power_on_ack() {
    return rsm_sched::power_on_ack();
}
///send asyn message, normally put into the receiver's message queue
pub fn send_asyn_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT {
    return rsm_sched::send_asyn_msg(dst, msg);
}

pub fn send_asyn_msg_ext<'de,T>(dst:&rsm_component_t,msg_id:u32,body:&T)->errcode::RESULT
    where T:Sized+Serialize+Deserialize<'de> {
    let msg=match rsm_message_t::new(msg_id, body) {
        None=>return errcode::ERROR_ENCODE_MSG,
        Some(m)=>m,
    };
    return rsm_sched::send_asyn_msg(dst, msg);
}

///send high priority asyn message, this type message is ensure delivery to the component before other normal message
pub fn send_asyn_priority_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT {
    return rsm_sched::send_asyn_priority_msg(dst, msg);
}

///set a timer, loop for **loop_count** times every **dur_msec** milliseconds. if *loop_count* is 0, the timer will not stop util application kill the timer
pub fn set_timer(dur_msec:u64,loop_count:u64,timer_data:usize)->Option<rsm_timer_id_t>{
    return rsm_timer::set_timer(dur_msec, loop_count, timer_data);
}

/// stop the timer, given the timer_id returned by *set_timer* function
pub fn kill_timer_by_id(timer_id:rsm_timer_id_t)->errcode::RESULT {
    return rsm_timer::kill_timer_by_id(timer_id);
}

///create a xlog client instance, then using the instance to output logs
pub fn new_xlog(module_name:&str)->xlog::xlogger_t {
    let serv_addr = match unsafe {&gRsmConfig} {
        None=>SocketAddr::new(IpAddr::from([127,0,0,1]),xlog::LOG_DEF_SERVICE_PORT),
        Some(c)=>c.log_config.self_addr,
    };
    return xlog::xlogger::new_xlogger(module_name, 
        &IpAddr::from([127,0,0,1]), 0, 
        &serv_addr.ip(),serv_addr.port());
}

