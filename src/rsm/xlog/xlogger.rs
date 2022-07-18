/* xlog.rs
    作者：	汪军
    创建日期：2022-5-17
    最新修订日期：2022-5-17

   模块功能描述：Log客户端代码，由每个应用自己负责调用；Log客户端和服务端通过网络进行通信，控制级别
每个模块首先调用NewXLogger，传入本地的模块名、本地IP地址、端口，LogService的IP和端口；
如果不需要跨节点通信本地地址可以填写127.0.0.1,如不关心本地地址的可以填写0.0.0.0
*/
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
use crate::common::errcode;
use serde_json;
use std::net::{self, IpAddr, SocketAddr, UdpSocket};
use std::time;
use crate::common::{self,rsm_time_t};
use super::*;


const _DEBUG: bool = false;

/*初始化Log，每个模块使用Log前要初始化一个实例，用此实例输出日志,入参为自己的模块名称，自身IP和端口，LogService的IP和端口
支持LogClient和Service在不同的节点中部署*/
pub fn new_xlogger(
    moduleName: &str,
    selfIp: &IpAddr,
    self_port: u16,
    servIp: &net::IpAddr,
    servPort: u16,
) -> xlogger_t {
    let server_addr = SocketAddr::new(servIp.clone(), servPort);
    let mut socket: Option<UdpSocket> = None;
    if let Ok(conn) = UdpSocket::bind(SocketAddr::new(*selfIp, self_port)) {
        socket = Some(conn);
    }

    let logger = xlogger_t {
        module_name: String::from(moduleName),
        self_ip: *selfIp,
        server_addr: server_addr,
        socket: socket,
        self_port: self_port,
        level: LOG_LEVEL_ERROR,
        sentPackets: 0,
        sentbytes: 0,
    };

    return logger;
}

impl xlogger_t {
    /*系统日志输出*/
    pub fn Log(
        &mut self,
        level: LOG_LEVEL,
        position: &str,
        err: errcode::RESULT,
        logDesc: &String,
    ) {
        if _DEBUG && (level<=self.level){
            self.print_debug(level,err,position,logDesc);
           
        }

        if level > self.level {
            return;
        }
        let logMsg = InnerLogMsg {
            ModuleName: self.module_name.clone(),
            OccureTime: rsm_time_t::now_local().unwrap(),
            LogType: LOG_TYPE_SYSTEM,
            LogLevel: level,
            Position: String::from(position),
            ErrCode: err,
            LogDesc: logDesc.clone(),
            Context: String::from("Null"),
        };

        self.sentLog(&logMsg);
    }

    pub fn set_log_level(&mut self, new_level: LOG_LEVEL) {
        if new_level >= 0 && new_level <= LOG_LEVEL_MAX {
            self.level = new_level;
        }
    }

    fn print_debug(&self,level: LOG_LEVEL,ec:errcode::RESULT,position: &str,logDesc:&String) {
        if ec == 0 {
            println!(
                "time:{},position={},msg:{}\n",
                common::format_datetime(&time::SystemTime::now()),
                position, logDesc
            );
        } else {
            println!(
                "time:{},level={},errcode={},position={},err={}\n",
                common::format_datetime(&time::SystemTime::now()),
                level, ec, position, logDesc
            );
        }
    }
    /*发送日志给LogService*/
    fn sentLog(&mut self, msg: &InnerLogMsg) -> errcode::RESULT {
        let res = serde_json::to_string::<InnerLogMsg>(msg);
        let json_str = match res {
            Err(e) => {
                println!("LogClient: Sent Log error, err={}\n", e);
                return errcode::ERROR_ENCODE_MSG;
            }
            Ok(s) => s,
        };

        if let Some(ref c) = self.socket {
            if let Ok(len) = c.send_to(json_str.as_bytes(), self.server_addr) {
                self.sentbytes += len as u64;
                self.sentPackets += 1;
                return errcode::RESULT_SUCCESS;
            } else {
                return errcode::ERROR_SEND_MSG;
            }
        } else {
            println!(
                "LogClient: udp connection is not ready, ip={},port={}\n",
                self.self_ip, self.self_port
            );
            return errcode::ERROR_BIND_SOCKET;
        }
    }

    pub fn Alertf(&mut self, postion: &str, err: errcode::RESULT, logDesc: &String) {
        self.Log(LOG_LEVEL_ALERT, postion, err, logDesc)
    }

    pub fn Errorf(&mut self, postion: &str, err: errcode::RESULT, logDesc: &String) {
        self.Log(LOG_LEVEL_ERROR, postion, err, logDesc)
    }

    pub fn Warningf(&mut self, postion: &str, err: errcode::RESULT, logDesc: &String) {
        self.Log(LOG_LEVEL_WARNING, postion, err, logDesc);
    }

    pub fn Infof(&mut self, postion: &str, err: errcode::RESULT, logDesc: &String) {
        self.Log(LOG_LEVEL_INFO, postion, err, logDesc);
    }

    pub fn Debugf(&mut self, postion: &str, err: errcode::RESULT, logDesc: &String) {
        self.Log(LOG_LEVEL_DEBUG, postion, err, logDesc);
    }

    fn GetLogStats(&self) -> (u64, u64) {
        return (self.sentPackets, self.sentbytes);
    }

    pub fn PrintStats(&self) {
        println!(
            "LogClient: ClientIP={}:{}, Server={}:{}\n",
            self.self_ip,
            self.self_port,
            self.server_addr.ip(),
            self.server_addr.port()
        );
        println!(
            "LogClient: sent packets={}, bytes={}\n",
            self.sentPackets, self.sentbytes
        );
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => (xlog::_print($crate::format_args!($($arg)*)));
}
