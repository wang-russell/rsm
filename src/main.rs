#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

///this is a sample application for RSM(realtime system middleware)
/// 1. init rsm, include oam/log service
/// 2. register application module
/// 3. start RSM, RSM will schedule the registered application module

use std::thread;
use std::time::Duration;
use std::env;
use rust_rsm::common::errcode;

use crate::rsm::config::{self,rsm_init_cfg_t,rsm_cfg_t};
use crate::rsm::{rsm_timer};
use std::net::{SocketAddr,IpAddr};
//use common::{errcode};
#[cfg(unix)] 
use std::io::{Error};

pub mod common;
pub mod rsm;
pub mod alg;
pub mod net_ext;

struct sample_app_t {
    _count:u64,
    log:Option<rsm::xlog::xlogger_t>,
}

impl rsm::Runnable for sample_app_t {
    fn on_init(&mut self,cid:&rsm::rsm_component_t) {
        println!("recv Init msg,self_cid={:?}\n",cid);
        rsm_timer::set_timer(2000, 0, 10);
        self.log=Some(rsm::new_xlog("sample"));
    }

    fn on_timer(&mut self,cid:&rsm::rsm_component_t,timer_id:rsm::rsm_timer_id_t,timer_data:usize) {
        println!("Recv Timer Event,timer_id={},data={},time={}\n",
            timer_id,timer_data,common::format_datetime(&std::time::SystemTime::now()));
        static mut data:u64=2048;
        let msg= rsm::rsm_message_t::new::<u64>(10015,unsafe {&data}).unwrap();
        if cid.get_inst_id()==1 {
            let dst=rsm::rsm_component_t::new(cid.get_cid(),1,2);
            let ret = rsm::send_asyn_msg(&dst, msg);
            if ret!=errcode::RESULT_SUCCESS {
                println!("Send message failed,ret={}",ret);
            }
        }
        unsafe { data+=2 };
    }

    fn on_message(&mut self,cid:&rsm::rsm_component_t,msg_id:rsm::rsm_message_id_t,msg:&rsm::rsm_message_t) {

        let self_cid=rsm::get_self_cid();
        let sender= rsm::get_sender_cid();

        println!("recv msg,msg_id={},content={:?},sender={:?},self={:?}\n",msg_id,msg,sender,self_cid);
        if let Some(log) = &mut self.log {
            log.Errorf("sampleapp", 0, &format!("self_id={},recv message id={},v={:?}",cid,msg_id,msg));
        }
        
    }

    fn on_close(&mut self,cid:&rsm::rsm_component_t) {

    }
}
static mut sampleApp:[sample_app_t;2]=[sample_app_t{_count:0,log:None},sample_app_t{_count:1,log:None}];

fn new_sample(cid:&rsm::rsm_component_t)->&'static mut dyn rsm::Runnable {
    let apps = unsafe {&mut sampleApp};
    let idx = cid.get_inst_id();
    return &mut apps[idx-1];
}
fn main() {
    let conf = parse_agrs();
    
    println!("Staring RSM framework,config={:?}", conf);
    println!("Copyright by russell Wang , {}","2022.7");
    
    rsm::rsm_init(&conf.cfg);
    registerApp(1);
    rsm::start_rsm();
    let mut content:u64=1;
    let mut msg=rsm::rsm_message_t::new::<u64>(10014, &content).unwrap();
    thread::sleep(Duration::from_millis(500));
    loop {      
        let dst = rsm::rsm_component_t::new(1,1,1);        
       
        rsm::send_asyn_msg(&dst, msg);
        content+=1;
        msg=rsm::rsm_message_t::new::<u64>(10014, &content).unwrap();
        thread::sleep(Duration::from_millis(3000));
    
    }
    
}

fn registerApp(cid:u32) {
    let attrs = rsm::component_attrs_t {
        cid:cid,    
        name:"sample".to_string(),
        inst_num:2, //实例数量
        qlen:100,
        priority:rsm::E_RSM_TASK_PRIORITY::THREAD_PRI_REALTIME_HIGHEST,
        need_init_ack:true,
    };
     rsm::registry_component(cid, &attrs, new_sample);
    
    
}


//解析参数，执行相关初始化
fn parse_agrs()->rsm_cfg_t {
    let def_log_addr=SocketAddr::new(IpAddr::from([127,0,0,1]),rsm::config::RSM_DEF_LOG_SERVER_PORT);
    let def_oam_addr=SocketAddr::new(IpAddr::from([127,0,0,1]),rsm::config::RSM_OAM_SERVER_PORT);
    let mut conf = rsm_cfg_t::new(1, Some(def_log_addr),
        Some(def_oam_addr),None);
    let mut args = env::args();
    println!("os param={:?}",args);
    args.nth(0);
    loop {
        let p = match args.nth(0) {
            None=> break,
            Some(s)=>s,
        };
        
        match p.as_str() {
            "--cfg"=> {
                if let Some(p1) = args.nth(0) {
                    conf.path=p1;         
                }
            },
            _=>(),
        }
    }
    //println!("config={:?}",conf);
    return conf;
}

//读取配置，并添加相应的配置数据
fn read_config(fpath:&String) {
    let rsm_conf = match config::load_rsm_cfg(fpath) {
        None=>{
            gen_empty_config(fpath);
            return;
        },
        Some(c)=>c,
    };
    
}

///生成一个空配置
fn gen_empty_config(fpath:&String) {
    let def_log_addr=SocketAddr::new(IpAddr::from([127,0,0,1]),rsm::config::RSM_DEF_LOG_SERVER_PORT);
    let def_oam_addr=SocketAddr::new(IpAddr::from([127,0,0,1]),rsm::config::RSM_OAM_SERVER_PORT);
    let conf = rsm_init_cfg_t::new(1, Some(def_log_addr),
        Some(def_oam_addr),None);
    config::save_rsm_cfg(fpath, &conf);
}
