#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::common::errcode;
use serde::{Deserialize, Serialize};
use crate::net_ext::restserver;
use std::{collections::HashMap, net::SocketAddr};

pub mod oam_main;

const OAM_MODULE_NAME:&str = "PlatOam";
#[derive(Deserialize,Serialize,Clone,Debug,PartialEq,Eq)]
pub enum E_RSM_OAM_OP {
    CLI_OP_INVALID = 0,
	CLI_OP_ADD = 1,
	CLI_OP_DEL = 2,
	CLI_OP_SET = 3,
	CLI_OP_SHOW = 4,
}

impl E_RSM_OAM_OP {
    pub fn from_http_method(method:&restserver::Method)->Self {
        match method {
            restserver::Method::Post => E_RSM_OAM_OP::CLI_OP_ADD,
            restserver::Method::Get => E_RSM_OAM_OP::CLI_OP_SHOW,
            restserver::Method::Put => E_RSM_OAM_OP::CLI_OP_SET,
            restserver::Method::Delete => E_RSM_OAM_OP::CLI_OP_DEL,
            _ => E_RSM_OAM_OP::CLI_OP_INVALID,
        }        
    }
}
const	CLI_OP_ADD_STRING  :&str = "ADD";
const	CLI_OP_DEL_STRING  :&str = "DEL";
const CLI_OP_SET_STRING  :&str = "SET";
const 	CLI_OP_SHOW_STRING :&str = "SHOW";
const 	CLI_OP_CLEAR_STRING:&str = "CLEAR";
const 	CLI_OP_HELP_STRING :&str = "HELP";


static mut gMapOpStr2Int:Option<HashMap<&str,E_RSM_OAM_OP>>= None;

fn init_map() {
    let mut map:HashMap<&str,E_RSM_OAM_OP>=HashMap::new();
	map.insert(CLI_OP_ADD_STRING,E_RSM_OAM_OP::CLI_OP_ADD);
	map.insert(CLI_OP_DEL_STRING, E_RSM_OAM_OP::CLI_OP_DEL);
    map.insert(CLI_OP_SET_STRING, E_RSM_OAM_OP::CLI_OP_SET);
    map.insert(CLI_OP_SHOW_STRING,E_RSM_OAM_OP::CLI_OP_SHOW);
    unsafe {
        gMapOpStr2Int = Some(map);
    }
}

//根据命令行字符串查找对应的命令操作码
pub fn get_cmd_by_name(op:&String)->E_RSM_OAM_OP{
    let map = match unsafe {&mut gMapOpStr2Int} {
        None=>return E_RSM_OAM_OP::CLI_OP_INVALID,
        Some(m)=>m,
    };
	
    return match map.get(op.as_str()) {
        None=>E_RSM_OAM_OP::CLI_OP_INVALID,
        Some(code)=>code.clone(),
    };
}

/*预置支持的OAM操作对象，其它应用可以进一步注册*/
const CLI_SUBJECT_COMPONENT:&str = "COMPONENT";
const 	CLI_SUBJECT_MSGQUEUE :&str = "MSGQUEUE";
const 	CLI_SUBJECT_LOCK     :&str = "LOCK";
const 	CLI_SUBJECT_TIMER    :&str = "TIMER";

const 	OAM_DEFAULT_IP_STRING:&str = "127.0.0.1";
const OAM_LOG_SERVICE_PORT:u16  = 10000;

const 	OAM_DEF_PORT:u16      = 12000;
const 	MAX_SOCKET_BUFFER:u16 = 32 * 1024;
const 	OAM_Welcome_String:&str = "\n RSM 1.0\n";

/*命令行解析结果*/
#[derive(Deserialize,Serialize,Clone)]
pub struct param_pair_t {
	pub Name :String,
	pub Value:String,
}

/*OAM请求字段,由OAM Server发送给各个应用*/
#[derive(Deserialize,Serialize)]
pub struct oam_cmd_req_t {
	pub Op     :E_RSM_OAM_OP,
	pub Subject:String,
	pub Params:Vec<param_pair_t>,
}

#[derive(Deserialize,Serialize,Clone)]
pub struct oam_resp_row_t {
	pub row:Vec<param_pair_t>,
}

/*OAM response*/
#[derive(Deserialize,Serialize)]
pub struct oam_cmd_resp_t {
	pub RetCode        :errcode::RESULT, //返回码，errcode.RESULT_SUCCESS表示成功
	pub Description    :String,         //描述，包括错误等
    #[serde(skip)]
	RespTableHeader:Vec<String>,
	pub RespRows        :Vec<oam_resp_row_t>, //返回结果，Key=Value的方式进行显示
}
impl oam_cmd_resp_t {
    pub fn new(ret_code:errcode::RESULT,desc:&String)->Self{
        return Self { RetCode: ret_code, Description: desc.clone(), 
            RespTableHeader:Vec::new(), RespRows: Vec::new() }
    }

    pub fn set_row_hdr(&mut self,hdr:Vec<String>) {
        self.RespTableHeader=hdr;
    }

    pub fn add_row(&mut self,row:&Vec<String>)->errcode::RESULT {
        if row.len()!=self.RespTableHeader.len() {
            return errcode::ERROR_INVALID_PARAM;
        }
        let mut data_row=oam_resp_row_t {
            row:Vec::new(),
        };
        for i in 0..row.len() {
            let pair = param_pair_t {
                Name:self.RespTableHeader[i].clone(),
                Value:row[i].clone(),
            };
            data_row.row.push(pair);
        }
        self.RespRows.push(data_row);
        return errcode::RESULT_SUCCESS
    }
    pub fn clear(&mut self) {
        self.RespTableHeader=Vec::new();
        self.RespRows=Vec::new();
    }
}


///Application Callback, register to OAM, and implement this callback function
pub type OamReqCallBack=fn(op:E_RSM_OAM_OP,url:&String,param:&String)->oam_cmd_resp_t;

///register a module callback, urls is a list of rest api url, the prefix /rsm and id following a "?" are not included
pub fn RegisterOamModule(urls:&[String], callback:OamReqCallBack)->errcode::RESULT{
    return oam_main::RegisterOamModule(urls, callback);
}

///init and start oam server, using configured socket addr
pub fn init_oam(server_addr:&SocketAddr,log_addr:&SocketAddr)->errcode::RESULT {
    return oam_main::init_oam(server_addr,log_addr);
}