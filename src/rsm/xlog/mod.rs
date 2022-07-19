//! #xlog
//! xlog is comprise of a xlog server and multiple xlog clients, which can be configured with different log filter level
//! application's log message can be send to the server only when the log level higher than the log filter level
//! each application module can create their own xlog client instance, then calling xlog client method to process log,
//! xlog client using UDP socket to send json formatted message to xlog server, 
//! then xlog server filter the log by configured filter level after receiving client's log message, the write to the disk periodically
//! 

use self::syslog::sys_log_client_t;
use super::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr,SocketAddr,UdpSocket};
use crate::common::{self,tsmap::TsHashMap};
use std::collections::{VecDeque};
use std::io::{self,Read,Write};
use std::fs;
use libdeflater;

pub mod syslog;
pub mod xlogger;
pub mod xlog_server;

pub type LOG_LEVEL = i32;

pub const LOG_LEVEL_EMERGENCY: LOG_LEVEL = 0;
pub const LOG_LEVEL_ALERT: LOG_LEVEL = 1;
pub const LOG_LEVEL_CRITICAL: LOG_LEVEL = 2;
pub const LOG_LEVEL_ERROR: LOG_LEVEL = 3;
pub const LOG_LEVEL_WARNING: LOG_LEVEL = 4;
pub const LOG_LEVEL_NOTICE: LOG_LEVEL = 5;
pub const LOG_LEVEL_INFO: LOG_LEVEL = 6;
pub const LOG_LEVEL_DEBUG: LOG_LEVEL = 7;
pub const LOG_LEVEL_MAX: LOG_LEVEL = 7;

pub const LOG_TYPE_SYSTEM: i32 = 0;
pub const LOG_TYPE_OPERATION: i32 = 1;
pub const LOG_TYPE_SECURITY: i32 = 2;

pub const LOG_DEF_PERSISTENT_LEVEL: LOG_LEVEL    = LOG_LEVEL_WARNING; //日志默认存盘的级别
pub const LOG_DEF_CONSOLE_OUTPUT_LEVEL: LOG_LEVEL  = LOG_LEVEL_INFO;

pub const LOG_DEF_SERVICE_PORT: u16 = 61000;
pub const SYSLOG_DEF_UDP_PORT:u16=512;

/*内部传递的Log数据结构*/
#[derive(Clone, Serialize, Deserialize)]
pub struct InnerLogMsg {
    ModuleName: String,
	#[serde(with = "time::serde::rfc3339")]
    OccureTime: common::rsm_time_t,
    LogType: i32,
    LogLevel: LOG_LEVEL,
    Position: String,
    ErrCode: errcode::RESULT,
    LogDesc: String,
    Context: String,
}

///Log listener, for application want to implement specific log storage & report function
pub trait LogListener {
	fn NotifyLog(&mut self,msg:&InnerLogMsg, msgSeq:u64)->bool;
}

pub (crate) fn LogFormat(msgStru:&InnerLogMsg, msgId:u64,sender:&SocketAddr)->String {
	/*SYSLOG-MSG = HEADER SP STRUCTURED-DATA [SP MSG]
	  HEADER = PRI VERSION SP TIMESTAMP SP HOSTNAME
	  SP APP-NAME SP PROCID SP MSGID */
	let strHdr = format!("<{}> 1 {} {}:{} {} {} {}", 1*8+msgStru.LogLevel, 
        msgStru.OccureTime.to_string(),
		sender.ip(), sender.port(), msgStru.ModuleName, msgStru.Position, msgId);

	let strMsg = format!("{} ErrorCode=\"{}\" {} {}\n", strHdr,msgStru.ErrCode, msgStru.LogDesc, msgStru.Context);
	return strMsg
}

#[derive(Clone,Debug)]
pub struct log_client_t {
	persistent_log_level:LOG_LEVEL,
	console_log_level:LOG_LEVEL,
	name:String, //客户模块的名字
	addr:SocketAddr,
	logPackets:u64,
    logBytes:u64,  //发送Log的条数和
}

const LOG_DEF_STORAGE_SIZE:u64=2*1024*1024;
const LOG_DEF_PATH:&str = "./";
const LOG_DEF_PREFIX:&str = "rsm_xlog";

///configuration for create a log service
#[derive(Clone,Debug,Serialize, Deserialize)]
pub struct log_service_config_t {
	pub persistentLevel:LOG_LEVEL,
	pub consoleLevel:LOG_LEVEL,
	///max log file disk size, unit is bytes, if the stored log file exceed this value, then the file will be tuncated
	pub maxStorageSize:u64, 
	pub maxStoragePeriod:i32,  
    pub logFilePath:String,
	pub logFilePrefix:String, 
    pub self_addr:SocketAddr,
	pub syslog_server:Option<SocketAddr>,
}
impl log_service_config_t {
	pub fn new_default()->Self {
		let def_addr=SocketAddr::new(IpAddr::from([127,0,0,1]),LOG_DEF_SERVICE_PORT);
		return Self { persistentLevel:LOG_LEVEL_ERROR, consoleLevel: LOG_LEVEL_ERROR, 
			maxStorageSize: LOG_DEF_STORAGE_SIZE, maxStoragePeriod: 2, 
			logFilePath:LOG_DEF_PATH.to_string(), logFilePrefix: LOG_DEF_PREFIX.to_string(), 
			self_addr:def_addr, syslog_server: None 
		}
	}
}

///log service should be create only once in any system
pub struct log_service_t  {
    service_conf:log_service_config_t,
	sck:UdpSocket,
	curLogFile:Option<fs::File>, 
	unSyncedMsg:i32,      
	lastWriteTime:common::datetime_t,
	queue:VecDeque<String>,
	logMuduleControl:TsHashMap<String,log_client_t>, 
	logModuleIndex:TsHashMap<SocketAddr,String>,  
	logListener:TsHashMap<String,*mut dyn LogListener>,
	logPackets:u64, 
    LogBytes:u64,
	logSeq:u64, //Log Msg序列号
	stdout:io::Stdout,
	sys_client:Option<sys_log_client_t>,
}

///Log client instance, should be created before using log function
pub struct xlogger_t {
    module_name: String,
    self_ip: IpAddr,
    self_port: u16,
    server_addr: SocketAddr,
    socket: Option<UdpSocket>,
    level: LOG_LEVEL,
    sentPackets: u64,
    sentbytes: u64,
}


pub(crate) fn compressFile(fileIn:&String, fileOut:&String)->errcode::RESULT {
	let mut fp1 = match fs::OpenOptions::new().read(true).open(fileIn) {
		Err(_)=>return errcode::ERROR_OPEN_FILE,
		Ok(f)=>f,
	};

	
	let mut fp2 = match fs::OpenOptions::new().create_new(true).write(true).open(fileOut) {
		Err(_)=>return errcode::ERROR_OPEN_FILE,
		Ok(f)=>f,
	};
	let stats = match fp1.metadata() {
		Err(_)=>return errcode::ERROR_OPEN_FILE,
		Ok(s)=>s,
	};
	let complvl=libdeflater::CompressionLvl::default();
	let mut comp =libdeflater::Compressor::new(complvl);
	comp.gzip_compress_bound(stats.len() as usize);
	let mut vec_buf_in = Vec::with_capacity(stats.len() as usize);
	let mut vec_buf_out = Vec::with_capacity(stats.len() as usize);
	unsafe {
		vec_buf_in.set_len(stats.len() as usize);
		vec_buf_out.set_len(stats.len() as usize);
	}
	let n_bytes = match fp1.read(vec_buf_in.as_mut_slice()) {
		Err(_)=>return errcode::ERROR_BUFFER_TOO_SMALL,
		Ok(l)=>l,
	};
	let comp_len = match comp.gzip_compress(&vec_buf_in.as_slice()[0..n_bytes], vec_buf_out.as_mut_slice()) {
		Err(_)=>return errcode::ERROR_BUFFER_TOO_SMALL,
		Ok(l)=>l,		
	};

	fp2.write(&vec_buf_out.as_slice()[0..comp_len]);
	fp2.flush();

	return errcode::RESULT_SUCCESS

}

