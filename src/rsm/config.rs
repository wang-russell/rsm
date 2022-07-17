use super::*;
use serde::{Deserialize,Serialize};
use serde_json;
use crate::common::errcode;
use std::fs::{self,File};
use std::io::{Error,Read,Write};
use std::net::{IpAddr,SocketAddr};

pub const RSM_DEF_LOG_SERVER_PORT:u16=61000;
pub const RSM_OAM_SERVER_PORT:u16=12000;

#[derive(Deserialize,Serialize,Clone,Debug)]
pub struct rsm_init_cfg_t {
    pub node_id:u32,
    pub max_component_num:usize,
    pub oam_server_addr:SocketAddr, 
    pub log_config:xlog::log_service_config_t,
     
}

impl rsm_init_cfg_t {
    pub fn new(node_id:u32,log_server_addr:Option<SocketAddr>,oam_server_addr:Option<SocketAddr>,
        syslog_server_addr:Option<SocketAddr>)->Self{
        let oam_addr = match oam_server_addr {
            None=>SocketAddr::new(IpAddr::from([127,0,0,1]),RSM_OAM_SERVER_PORT),
            Some(a)=>a,
        };

        let mut cfg = Self{
            node_id:node_id,
            max_component_num:MAX_COMPONENT_NUM,
            log_config:xlog::log_service_config_t::new_default(),
            oam_server_addr:oam_addr,
        };
        if let Some(log_addr) = log_server_addr {
            cfg.log_config.self_addr=log_addr;
        }
        if let Some(syslog_addr) = syslog_server_addr {
            cfg.log_config.syslog_server=Some(syslog_addr);
        }
        

        
        return cfg
    }
}

#[derive(Debug)]
pub struct rsm_cfg_t {
    pub path:String,
    pub cfg:rsm_init_cfg_t,
}

impl rsm_cfg_t {
    pub fn save_cfg(&self)->errcode::RESULT {
        return save_rsm_cfg(&self.path, &self.cfg)
    }

    pub fn new(node_id:u32,log_server_addr:Option<SocketAddr>,oam_server_addr:Option<SocketAddr>,
        syslog_server_addr:Option<SocketAddr>)->Self{
        let init_cfg= rsm_init_cfg_t::new(
            node_id,log_server_addr,oam_server_addr,syslog_server_addr);
        return Self {
            path:String::default(),
            cfg:init_cfg,
        };
    }
}

pub fn save_rsm_cfg(path:&String,cfg:&rsm_init_cfg_t)->errcode::RESULT {
    let mut fp:File = match fs::OpenOptions::new().read(true).write(true).create(true).open(path) {
        Ok(f)=>f,
        Err(e)=> {
            println!("Error Open File,e={},os_err={}",e,Error::last_os_error());
            return errcode::ERROR_OPEN_FILE;
        },          
    };

    let rstr = match serde_json::to_string_pretty(cfg) {
        Ok(s)=>s,
        Err(_)=>return errcode::ERROR_ENCODE_MSG,
    };

    match fp.write(rstr.as_bytes()) {
        Ok(_)=> errcode::RESULT_SUCCESS,
        Err(_)=>errcode::ERROR_COMMON,
    }
   
}

pub fn load_rsm_cfg(path:&String)->Option<rsm_cfg_t> {
    let mut buf = Vec::new();
    let mut fp:File = match fs::OpenOptions::new().read(true).open(path) {
        Ok(f)=>f,
        Err(e)=> {
            println!("Error Open File,e={},os_err={}",e,Error::last_os_error());
            return None
        },
    };
    buf.resize(32768,0u8);
    let len = match fp.read(buf.as_mut_slice()) {
        Ok(l)=>l,
        Err(_)=> {
            return None
        },
    };
    let cfg = match serde_json::from_slice::<rsm_init_cfg_t>(&buf[0..len]) {
        Ok(c)=>c,
        Err(_)=>return None,
    };
    return Some(rsm_cfg_t {
        path:path.clone(),
        cfg:cfg,
    })
}
