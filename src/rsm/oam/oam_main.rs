use crate::common::{errcode,tsmap::TsHashMap};
use crate::rsm::xlog;
use super::*;
use std::net::{SocketAddr,IpAddr};
use crate::net_ext::restserver;
use serde_json;

#[derive(Clone)]
enum ERPCMethod {
    RPC_INVALID_OP = 0,
    RPC_ADD = 1,
    RPC_GET = 2,
    RPC_UPDATE = 3,
    RPC_DELETE = 4,
}

impl ERPCMethod {
    pub fn from_http_method(method: &restserver::Method) -> ERPCMethod {
        match method {
            restserver::Method::Post => ERPCMethod::RPC_ADD,
            restserver::Method::Get => ERPCMethod::RPC_GET,
            restserver::Method::Put => ERPCMethod::RPC_UPDATE,
            restserver::Method::Delete => ERPCMethod::RPC_DELETE,
            _ => ERPCMethod::RPC_INVALID_OP,
        }
    }
    pub fn to_string(&self)->String {
        match self {
            ERPCMethod::RPC_ADD=>String::from("Post") ,
            ERPCMethod::RPC_GET=> String::from("Get"),
            ERPCMethod::RPC_UPDATE=>  String::from("Put"),
            ERPCMethod::RPC_DELETE=> String::from("Delete"),
            ERPCMethod::RPC_INVALID_OP => String::from("NULL"),
            }
    }
}

type ProcessRestCall =
    fn(method: ERPCMethod, path: &str, body: &String) -> Result<(String,restserver::E_CONTENT_TYPE), errcode::RESULT>;
//RestServer主要路径常量
const PATH_RSM_CMD_REQ: &str = "/rsm";
const PATH_RSM_OAM: &str = "/oam";
const PATH_RSM_CMD_HELP:&str = "/help";

const SUCCESS_RESPONSE_STR:&str="Success!\r\n";
const MIN_KEEP_ALIVE_TIMER_MSEC:u32 = 50;//最小保活时间

//RestServer 的CallBack处理
fn call_back(
    method: &restserver::Method,
    path: &str,
    body: &String,
) -> Result<(String,restserver::E_CONTENT_TYPE), errcode::RESULT> {

    println!("[rsm_oam]recv restcall,method={},url={},body={}",
            method,path,body);
    if !path.starts_with(PATH_RSM_CMD_REQ) {
        return Err(errcode::ERROR_NOT_FOUND)
    }


    let inst =match unsafe {&mut gOamInst} {
         None=>return Err(errcode::ERROR_NOT_INITIALIZED),
         Some(o)=>o,
    };

    let (url,id) = match get_app_path(path.to_lowercase().as_str()) {
        None=>return Err(errcode::ERROR_INVALID_MSG),
        Some((u,i))=>(u,i),
    };
    println!("path parse result,url={},id={}",url,id);
    if url.eq(PATH_RSM_CMD_HELP) {
        let tResp = getOamHelp(String::default());
        let resp = serde_json::to_string_pretty::<oam_cmd_resp_t>(&tResp).unwrap();
        return Ok((resp,restserver::E_CONTENT_TYPE::e_application_json));
    }
    let oam_op = E_RSM_OAM_OP::from_http_method(method);

    return inst.invoke_call_back(oam_op,&url,&id);
    
}


struct oam_instance_t {
    self_addr:SocketAddr,
    registry:TsHashMap<String,OamReqCallBack>,
    recv_req:u64,
    sent_ok_resp:u64,
    sent_err_resp:u64,
    log:xlog::xlogger_t,
    rest_server:Option<restserver::RestServer>,
}

impl oam_instance_t {
    pub fn register_oam_module(&mut self,urls:&[String], callback:OamReqCallBack)->errcode::RESULT {
        for s in urls {
            let s1 =s.trim().to_lowercase();
            let ret = self.registry.insert(s1, callback);
            self.log.Errorf("[oam register module]",ret, 
                &format!("Register OamModule, subject={},ret={}", s, ret));
            if ret != errcode::RESULT_SUCCESS {
                return ret
            }
        }
        return errcode::RESULT_SUCCESS
    }

    pub fn invoke_call_back(&mut self,op:E_RSM_OAM_OP,url:&String,id:&String)->Result<(String,restserver::E_CONTENT_TYPE), errcode::RESULT> {
        let callback = match self.registry.get_mut(url) {
            None=>return Err(errcode::ERROR_NOT_FOUND),
            Some(v)=>v,
        };
        let ret = (*callback)(op,url,id);
        self.recv_req+=1;
        if ret.RetCode!=errcode::RESULT_SUCCESS {
            self.sent_err_resp+=1;
            return Err(ret.RetCode)
        }

        if let Ok(resp)=serde_json::to_string_pretty::<oam_cmd_resp_t>(&ret) {
            self.sent_ok_resp+=1;
            return Ok((resp,restserver::E_CONTENT_TYPE::e_application_json))
        } else {
            self.sent_err_resp+=1;
            Err(errcode::ERROR_COMMON)
        }
        
    }

    pub fn get_oam_stats(&self)->oam_cmd_resp_t {
        let mut tResp=oam_cmd_resp_t::new(errcode::RESULT_SUCCESS,&"oam module stats".to_string());
        let hdrs=vec!["registered callback count".to_string(),"recv request".to_string(),
            "sent_ok_count".to_string(),"sent_err_count".to_string()];
        tResp.set_row_hdr(hdrs);

        let row=vec![self.registry.len().to_string(),self.recv_req.to_string(),
            self.sent_ok_resp.to_string(),self.sent_err_resp.to_string()];
        tResp.add_row(&row);
        return tResp

    }
    pub fn clear_stats(&mut self) {
        self.sent_ok_resp=0;
        self.sent_err_resp=0;
        self.recv_req = 0;
    }
    pub fn start_rest_server(&mut self) {        
	    let server = match restserver::RestServer::new(self.self_addr.ip().clone(), 
        self.self_addr.port(), call_back) {
            Ok(s)=>s,
            Err(e)=> {
                println!("Start rest server failed,ret={},addr={}",e,self.self_addr);
                return
            },
        };
        server.run();
        //self.rest_server=Some(server);

    }
}

static mut gOamInst:Option<oam_instance_t>=None;

pub fn init_oam(server_addr:&SocketAddr,log_server:&SocketAddr)->errcode::RESULT {
    unsafe {
        if gOamInst.is_some() {
            return errcode::ERROR_ALREADY_EXIST;
        }
    }
    println!("[rsm oam]:begin init oam,server_addr={},log_server_addr={}",server_addr,log_server);
    init_map();

    let inst=oam_instance_t {
        self_addr:server_addr.clone(),
        registry:TsHashMap::new(256),
        recv_req:0,
        sent_ok_resp:0,
        sent_err_resp:0,
        log:xlog::xlogger::new_xlogger(OAM_MODULE_NAME, &IpAddr::from([127,0,0,1]), 0, 
                &log_server.ip(), log_server.port()),
        rest_server:None,
    };

    unsafe {
        gOamInst=Some(inst);
    }
    let urls =[PATH_RSM_OAM.to_string()];
    RegisterOamModule(&urls, process_oam_self_stats);
    std::thread::spawn(|| run_oam_server());
    errcode::RESULT_SUCCESS
    
}

/*初始化OAM子系统*/
//对于不同的ProcessId，其OAM Server的TCP端口地址为Base port+ProcessId
fn run_oam_server() {
    let inst =match unsafe {&mut gOamInst} {
        None=>return,
        Some(o)=>o,
    };
    inst.start_rest_server();
}


/*注册OAM实现接口，每个希望通过OAM进行管理的模块调用注册OAM接口实现；每个对象都不能重复
后续调用回调时，会根据操作的Subject查找Map表，进行回调*/
pub(crate) fn RegisterOamModule(urls:&[String], callback:OamReqCallBack)->errcode::RESULT {
    let inst =match unsafe {&mut gOamInst} {
        None=>return errcode::ERROR_INIT_FAILED,
        Some(o)=>o,
    };

	return inst.register_oam_module(urls, callback)
}

const help_str:&str = "Show|Add|Del|Set Subject [:param_name=param_value][,:param_name=param_value]*\n Subject = Component | Task | Timer | Lock";
/*构建一个OAM帮助信息*/
fn getOamHelp(_subject:String)->oam_cmd_resp_t {
	let tResp = oam_cmd_resp_t{RetCode: errcode::RESULT_SUCCESS,
        Description:help_str.to_string(),
        RespTableHeader:Vec::new(),
        RespRows:Vec::new(),
    };

	return tResp
}

///get app path and id from rest call URL, strip the /rsm prefix
fn get_app_path(path:&str)->Option<(String,String)> {
    if !path.starts_with(PATH_RSM_CMD_REQ) {
        return None
    }
    let url=match path.strip_prefix(PATH_RSM_CMD_REQ) {
        None=>return None,
        Some(s)=>s.to_string(),
    };
    let idx=match url.rfind("?") {
        None=>0,
        Some(idx)=>idx,
    };
    if idx==0 {
        return Some((url,String::default()))
    }
    let url_new = url[0..idx].to_string();
    let id = url[idx+1..].to_string();
    return Some((url_new,id))
}

///get_oam_self_stats, the url must be /rsm/oam 
fn process_oam_self_stats(op:E_RSM_OAM_OP,url:&String,_param:&String)->oam_cmd_resp_t {    
    let mut tResp = oam_cmd_resp_t::new(errcode::ERROR_NOT_FOUND,&"".to_string());
    if url.ne("/oam") {
        return tResp;
    }
    let inst =match unsafe {&mut gOamInst} {
        None=>return tResp,
        Some(o)=>o,
    };

    match op {
        E_RSM_OAM_OP::CLI_OP_SHOW=>return inst.get_oam_stats(),
        E_RSM_OAM_OP::CLI_OP_DEL=> {
            inst.clear_stats();
            tResp.RetCode=errcode::RESULT_SUCCESS;
            return tResp;
        },
        _=>{
            tResp.RetCode=errcode::ERROR_NOT_SUPPORT;
            return tResp
        },
    }
	
}