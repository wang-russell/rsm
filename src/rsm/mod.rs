#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

///RSM=Realtime Software Middleware
/// 本文件为RSM的公共接口定义
use crate::common::{self,errcode};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr,SocketAddr};
use serde_json;

pub mod task;
pub mod rsm_sched;
pub mod rsm_timer;
pub mod os_timer;
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
pub const RSM_SYSTEM_CID_START:u32 = 1;
pub const RSM_SYSTEM_CID_END:u32 = 1023;
pub const RSM_USER_CID_START:u32 = 1024;
///maximum instance number per cid
pub const RSM_MAX_INST_PER_CID:usize=16;
///allowed max message queue len
pub const RSM_MAX_QUEUE_LEN:usize = 16384;

pub const RSM_MAX_MESSAGE_LEN:usize = 64000;

///5 priority for a given task
#[derive(Copy,Clone,PartialEq,Debug,Eq,Serialize)]
pub enum E_RSM_TASK_PRIORITY {
    THREAD_PRI_LOW = 0,
	THREAD_PRI_NORMAL = 1,
	THREAD_PRI_HIGH = 2,
	THREAD_PRI_REALTIME = 3,
	THREAD_PRI_REALTIME_HIGH = 4,
    THREAD_PRI_REALTIME_HIGHEST = 5,
}
///every runnable component is defined as rsm_component_t
#[derive(Eq,PartialEq,Hash,Clone,Debug)]
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

type rsm_new_task=fn(cid:&rsm_component_t)->&'static mut dyn Runnable;
///Component must implement Runnable Trait
pub trait Runnable {
    fn on_init(&mut self,cid:&rsm_component_t);
    fn on_timer(&mut self,cid:&rsm_component_t,timer_id:rsm_timer_id_t,timer_data:usize);
    fn on_message(&mut self,cid:&rsm_component_t,msg_id:rsm_message_id_t,msg:&rsm_message_t);
    fn on_close(&mut self,cid:&rsm_component_t);
}

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

///rsm message associated definition
pub const RSM_SYS_MESSAGE_ID_START:u32 = 1;
pub const RSM_SYS_MESSAGE_ID_END:u32 = 8191;
pub const RSM_USER_MESSAGE_ID_START:u32 = 8192;
pub const RSM_INVALID_MESSAGE_ID:u32 = 0;

pub const RSM_MSG_ID_MASTER_POWER_ON:u32 = 1;
pub const RSM_MSG_ID_SLAVE_POWER_ON:u32 = 2;
pub const RSM_MSG_ID_POWER_ON_ACK:u32 = 3;
pub const RSM_MSG_ID_POWER_OFF:u32 = 4;
pub const RSM_MSG_ID_TIMER:u32 = 10;


#[derive(Clone,Debug)]
pub struct rsm_message_t {
    msg_id:u32,
    timer_id:rsm_timer_id_t,
    timer_data:usize,
    msg_body:String,
}
impl rsm_message_t {
    pub fn new<'de,T>(msg_id:rsm_message_id_t,body:&T)->Option<rsm_message_t> 
    where T:Sized+Serialize+Deserialize<'de> {
        let msg_body = match serde_json::to_string(body) {
            Ok(s)=>s,
            Err(_)=>return None,
        };
        let msg=Self {
            msg_id:msg_id,
            timer_id:0,
            timer_data:0,
            msg_body:msg_body,
        };
        return Some(msg);
    }

    pub(crate) fn new_timer_msg(timer_id:rsm_timer_id_t,timer_data:usize)->Option<rsm_message_t> {
        let msg=Self {
            msg_id:RSM_MSG_ID_TIMER,
            timer_id:timer_id,
            timer_data:timer_data,
            msg_body:String::default(),
        };
        return Some(msg);
    }

    pub fn decode<'a,T>(msg:&'a Self)->Option<T>
    where T:Deserialize<'a> {
        match serde_json::from_slice::<T>(msg.msg_body.as_bytes()) {
            Ok(v)=>Some(v),
            Err(_)=>None,
        }
    }
}
static mut gRsmConfig:Option<config::rsm_init_cfg_t>=None;
///rsm_init(), initialize rsm subsystem
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
    errcode::RESULT_SUCCESS
}

///after application initialize RSM and register all their running component, then invoke start_rsm
pub fn start_rsm() {
    println!("Start RSM, current={}",common::format_datetime(&std::time::SystemTime::now()));
    std::thread::spawn(|| rsm_timer::start_timer_thread());
    std::thread::spawn(|| rsm_sched::run());
    
}

///rsm api
pub fn registry_component(cid:u32,attrs:&component_attrs_t,callback:rsm_new_task)->errcode::RESULT {
    return rsm_sched::registry_component(cid, attrs, callback)
}

pub fn get_self_cid()->Option<rsm_component_t>{
    return rsm_sched::get_self_cid();
}

pub fn power_on_ack() {
    return rsm_sched::power_on_ack();
}

pub fn send_asyn_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT {
    return rsm_sched::send_asyn_msg(dst, msg);
}
pub fn send_asyn_priority_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT {
    return rsm_sched::send_asyn_priority_msg(dst, msg);
}

pub fn set_timer(dur_msec:u64,loop_count:u64,timer_data:usize)->Option<rsm_timer_id_t>{
    return rsm_timer::set_timer(dur_msec, loop_count, timer_data);
}

pub fn kill_timer_by_id(timer_id:rsm_timer_id_t)->errcode::RESULT {
    return rsm_timer::kill_timer_by_id(timer_id);
}
pub fn new_xlog(module_name:&str)->xlog::xlogger_t {
    let serv_addr = match unsafe {&gRsmConfig} {
        None=>SocketAddr::new(IpAddr::from([127,0,0,1]),xlog::LOG_DEF_SERVICE_PORT),
        Some(c)=>c.log_config.self_addr,
    };
    return xlog::xlogger::new_xlogger(module_name, 
        &IpAddr::from([127,0,0,1]), 0, 
        &serv_addr.ip(),serv_addr.port());
}

