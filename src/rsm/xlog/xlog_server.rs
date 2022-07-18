#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use super::*;
use crate::common::{errcode,tsmap::TsHashMap};
use std::net::{SocketAddr,UdpSocket,IpAddr};
use std::time::{self,Duration,SystemTime};
use std::fs::{self,OpenOptions};
use std::collections::{VecDeque};
use std::io::{self,Read,Write};
use std::thread;

const  MAX_LOG_QUEUE_LEN:usize    = 2048;
const  MAX_LOG_MSG_LEN:usize      = 65000;
const  MAX_OOB_MSG_LEN:usize      = 4096;
const  MAX_UNSYNCED_MSG:i32     = 256;
const  LOG_SYNC_DISK_PERIOD:u64 = 2; //刷新到磁盘的周期
const MAX_LOG_FILE:usize = 365;

impl log_service_t {
    pub fn new(conf:&log_service_config_t)->Option<log_service_t> {
        let sck=match UdpSocket::bind(conf.self_addr) {
            Err(_)=>return None,
            Ok(s)=>s,
        };

        let mut service = Self {
            service_conf:conf.clone(),
            sck:sck,
            curLogFile:None,
            unSyncedMsg:0,      //已经写入文件，但是没有存盘的消息计数
            lastWriteTime:common::get_datetime_from_std(&SystemTime::now()),
            queue:VecDeque::with_capacity(MAX_LOG_QUEUE_LEN),
            logMuduleControl:TsHashMap::new(128), //以名称索引到日志的客户模块信息
            logModuleIndex:TsHashMap::new(128),   //以IP:端口索引到名称
            logListener:TsHashMap::new(128),
            logPackets:0, 
            LogBytes:0,
            logSeq:1,
			stdout:io::stdout(),
			sys_client:None,
        };
		if let Some(syslog_addr) = conf.syslog_server {
			if let Ok(mut syslog)=syslog::sys_log_client_t::new(&conf.self_addr) {
				syslog.set_server_addr(&syslog_addr);
				service.sys_client=Some(syslog);
			}

		}

        return Some(service)
    }
    /*根据自动生成的文件名，打开日志文件*/
    pub fn openLogFile(&mut self)->errcode::RESULT {    
        let tm = common::get_datetime_from_std(&SystemTime::now());
    
        let fileName = format!("{}{}",self.service_conf.logFilePath, self.getLogFileName(&tm));
    
        let fd = match OpenOptions::new().create(true).write(true).append(true).open(fileName.clone()) {
            Ok(f)=>f,
            Err(e)=>{
				println!("open file {} error,err={}",fileName,e);
				return errcode::ERROR_OPEN_FILE;
			},
        };
        self.curLogFile=Some(fd);
        return errcode::RESULT_SUCCESS
    }

    /*给定一个时间，返回一个规整的日志文件名称，一般为prefix+"_"+YYYYMMDD+".log"*/
    fn getLogFileName(&self,tm:&common::datetime_t)->String {
	    let fileName = format!("{}_{:#04}{:#02}{:#02}.log", self.service_conf.logFilePrefix, 
			tm.get_year(), tm.get_mon_in_year(), tm.get_day_in_mon());
	    return fileName
    }

    /*Log Service的内部数据获取*/
    pub fn GetLogFilePath(&self)->String {
	    return self.service_conf.logFilePath.clone()
    }

    ///init log service
    pub fn init(&mut self) {

    }
    /*设置全局日志级别,分为存盘级别和控制台输出级别*/
    fn SetGlobalPersitentLogLevel(&mut self,newLevel:LOG_LEVEL) {
	    self.service_conf.persistentLevel = newLevel;

    }

    fn SetGlobalConsoleLogLevel(&mut self,newLevel:LOG_LEVEL) {
	    self.service_conf.consoleLevel = newLevel;
    }

/*设置模块级的持久化日志级别*/
    fn SetModulePersitentLogLevel(&mut self,name:&String, newLevel:LOG_LEVEL)->errcode::RESULT {
		if let Some(module) = self.logMuduleControl.get_mut(name) {
			module.persistent_log_level = newLevel;
		} else {
			let module = log_client_t{
				name: name.clone(),
				persistent_log_level:newLevel,
				console_log_level: LOG_DEF_CONSOLE_OUTPUT_LEVEL,
				addr:SocketAddr::new(IpAddr::from([127,0,0,1]),0),
				logBytes:0,
				logPackets:0,
			};
			self.logMuduleControl.insert(name.clone(),module);
		}
	    return errcode::RESULT_SUCCESS

    }

/*设置模块级的控制台输出日志级别*/
    fn SetModuleConsoleLogLevel(&mut self,name:&String, newLevel:LOG_LEVEL)->errcode::RESULT {
		if let Some(module) = self.logMuduleControl.get_mut(name) {
			module.console_log_level = newLevel;
		} else {
			let module = log_client_t{
				name: name.clone(),
				persistent_log_level:LOG_DEF_PERSISTENT_LEVEL,
				console_log_level: newLevel,
				addr:SocketAddr::new(IpAddr::from([127,0,0,1]),0),
				logBytes:0,
				logPackets:0,
			};
			self.logMuduleControl.insert(name.clone(),module);
		}

	    return errcode::RESULT_SUCCESS
    }

	/*设置日志存储的参数，0表示不修改*/
    fn SetLogPersitentParam(&mut self,maxDiskSize:u64, maxPeriod:i32) {
	    self.service_conf.maxStorageSize = maxDiskSize;

	    if maxPeriod >= 0 {
		    self.service_conf.maxStoragePeriod = maxPeriod;
	    }
    }

	pub fn set_syslog_addr(&mut self,addr:&SocketAddr)->errcode::RESULT {
		if let Some(log)=&mut self.sys_client {
			log.set_server_addr(addr);
			return errcode::RESULT_SUCCESS;
		}
		return errcode::ERROR_NOT_INITIALIZED;
	}

	/*处理监听器的回调，被调用者返回false，则表示流程结束*/
	fn RegisterLogModule<'a>(&'a mut self,name:&String, addr:&SocketAddr)->Option<&'a mut log_client_t> {
		if self.logMuduleControl.contains_key(name) {
			 match self.logMuduleControl.get_mut(name) {
				None=>return None,
				Some(m)=> {
				if m.addr.port() == 0 {				
					self.logModuleIndex.insert(addr.clone(),name.clone());
					m.addr=addr.clone();
				}
				return Some(m);
			},
			}
		}

		self.logModuleIndex.insert(addr.clone(),name.clone());
		let module=log_client_t{
				persistent_log_level: LOG_LEVEL_WARNING, console_log_level: LOG_LEVEL_INFO,
				name: name.clone(), addr:addr.clone(),
				logBytes:0,
				logPackets:0,};
		self.logMuduleControl.insert(name.clone(),module);
		return self.logMuduleControl.get_mut(name);
		
	}
	/*处理监听器的回调，被调用者返回false，则表示流程结束*/
	fn processListener(&mut self,msgStru:&InnerLogMsg, msgSeq:u64)->bool {
		for (_, v) in self.logListener.iter_mut() {
			let callback = unsafe {&mut *(*v)};
			let ret = callback.NotifyLog(msgStru, msgSeq);
			if !ret {
				return false;
			}
		}
		self.logListener.end_iter();
		return true
	}

	/*日志接收处理,JSON格式
首先进行回调处理，然后进行存盘处理;最后发送给SysLog Server*/
	pub fn recvLog(&mut self) {
		println!("LogServer: Begin Receiving Log Message");
		let _ = self.sck.set_read_timeout(Some(Duration::from_millis(50)));
		let mut recv_buf=[0u8;MAX_LOG_MSG_LEN];
		loop {
			let (len,addr)=match self.sck.recv_from(&mut recv_buf[..]) {
				Err(_)=>continue,
				Ok((l,a))=>(l,a),
			};
			//println!("log server recv message from {},len={},msg = {}",addr,len,String::from_utf8_lossy(&recv_buf[0..len]));
			self.logPackets+=1;
			self.LogBytes+=len as u64;
		let logStru = match serde_json::from_slice::<InnerLogMsg>(&recv_buf[0..len]) {
			Err(_e)=> {
				println!("log server decode message err {},len={}",_e,len);
				continue;
			},
			Ok(d)=>d,
		};
		self.processLog(&logStru, &recv_buf[0..len], &addr);

		}

	}

	/*内部汇总输出日志的函数*/
	fn innerOutputLog(&mut self,msg:&InnerLogMsg, m:&mut log_client_t,formated_msg:&String) {		
		self.persistentLog(msg,m, formated_msg); //首先进行持久化处理
		self.consoleOutputLog(msg, m, formated_msg);    //然后进行控制台处理
	}

	/*真正的处理Log日志的任务，初始化日志实例时创建线程任务运行*/
	fn processLog(&mut self,msg:&InnerLogMsg,origin_msg:&[u8],sender:&SocketAddr) {
			self.logSeq+=1;
			let ret = self.processListener(msg, self.logSeq);
			if !ret {
				return
			}
			let seq = self.logSeq;
			/*每次都调用RegisterLogModule是为解决先在服务端初始化模块级日志，后收到日志的问题*/
			let mut client = match self.RegisterLogModule(&msg.ModuleName, sender) {
				None=>return,
				Some(m)=> m,
			};
	
			client.logPackets+=1;
			client.logBytes += origin_msg.len() as u64;

			/*输出Log文件*/
			let c= unsafe {&mut *(client as *mut log_client_t)};
			let strMsg = LogFormat(msg,seq,sender);
			self.innerOutputLog(msg, c,&strMsg);
	}



	/*周期性日志存盘的操作*/
	fn flushLogFile(&mut self) {
	
		if self.unSyncedMsg <= 0 {
			return
		}
		let cur = common::get_now_usec64();

		if self.unSyncedMsg >= MAX_UNSYNCED_MSG ||
			cur>=self.lastWriteTime.to_usecs()+1000*1000*LOG_SYNC_DISK_PERIOD {
			self.forceSyncLogFile();
		}

	}

/*处理日志持久化流程，首先判断级别是否够*/
fn persistentLog(&mut self,msgStru:&InnerLogMsg, m:&log_client_t, formated_msg:&String)->errcode::RESULT {
	if msgStru.LogLevel > self.service_conf.persistentLevel || msgStru.LogLevel > m.persistent_log_level {
		return errcode::ERROR_NO_OP;
	}

	let cur =common::get_datetime_from_std(&time::SystemTime::now());
	let last = self.lastWriteTime.clone();
	/*已经是另外一天，需要打开新的日志文件*/
	if (cur.get_day_in_year() != last.get_day_in_year()) || (cur.get_year()!= last.get_year()) {
		self.forceSyncLogFile();

		/*自动根据当前日期计算一个新的日志文件*/
		let err = self.openLogFile();
		if err != errcode::RESULT_SUCCESS {
			return err
		}
	}

	self.write_to_file(formated_msg.as_bytes());
	self.unSyncedMsg+=1;
	//println!("[log server]write to disk,seq={},unsynced={},msg_len={}",self.logSeq,self.unSyncedMsg,formated_msg.len());
	self.flushLogFile();
	self.lastWriteTime = common::get_datetime_from_std(&time::SystemTime::now());
	return errcode::RESULT_SUCCESS
}

	fn write_to_file(&mut self,buf:&[u8])->errcode::RESULT {
		if let Some(ref mut f) = &mut self.curLogFile {
			let _ = f.write(buf);
				return errcode::RESULT_SUCCESS
		} else {
			return errcode::ERROR_OPEN_FILE;
		}
	}
/*强制刷新到磁盘文件，将未同步的日志文件，刷新到磁盘*/
fn forceSyncLogFile(&mut self) {
	/*假如当前日志超过了存盘的最大尺寸限制，则截断日志大小*/
	let file=match &mut self.curLogFile {
		None=>return,
		Some(f)=>f,
	};
	if let Ok(stats) = file.metadata() {
		if stats.len()>self.service_conf.maxStorageSize {
			let _ = file.set_len(self.service_conf.maxStorageSize);
					
		}
	}
	let _ = file.sync_all();
	self.unSyncedMsg = 0;

}

	/*处理日志控制台输出流程，首先判断日志输出级别是否可以输出*/
   fn consoleOutputLog(&mut self,msgStru:&InnerLogMsg, m:&log_client_t, formated_msg:&String) {
		if msgStru.LogLevel > self.service_conf.consoleLevel || msgStru.LogLevel > m.console_log_level {
			return
		}
		let _ = self.stdout.write(formated_msg.as_bytes());	
		self.send_to_syslog_server(msgStru, formated_msg);
	}

	fn send_to_syslog_server(&mut self,msgStru:&InnerLogMsg, formated_msg:&String) {
		//to-do
		if let Some(syslog)=&mut self.sys_client {
			syslog.send_encoded_msg(formated_msg);
		}
	}
	/*日志监听器注册，LogService将所有的日志均发送给监听器*/
	pub fn RegisterListener(&mut self,name:&String, listener:&'static mut dyn LogListener)->errcode::RESULT {

		if self.logListener.contains_key(name) {
			return errcode::ERROR_ALREADY_EXIST;
		}
		self.logListener.insert(name.clone(),listener as *mut dyn LogListener);
		return errcode::RESULT_SUCCESS
	}

	pub fn DeregisterListener(&mut self,name:&String)->errcode::RESULT {
		if !self.logListener.contains_key(name) {
			return errcode::ERROR_NOT_FOUND;
		}
		self.logListener.remove(name);
		return errcode::RESULT_SUCCESS
	}

	
	fn do_clean(&mut self) {
		let cur = time::SystemTime::now();
		let mut toBeClean=false;
		let mut total_file_size =0u64;
		println!("[LogClean Task]Begin Log Clean Task");
		for i in 1..MAX_LOG_FILE {

			let tm = common::get_datetime_from_std(&cur.checked_sub(Duration::from_secs(3600*24*i as u64)).unwrap());
			let f1 = self.getLogFileName(&tm);
			let f2 = format!("{}{}",self.GetLogFilePath(),f1);
			let ziped = format!("{}{}",f2, ".zip");
			if toBeClean {
				let _ = fs::remove_file(ziped);
				let _ = fs::remove_file(f2);
				continue
			}
			
			if errcode::RESULT_SUCCESS == compressFile(&f2, &ziped) {
					let _ = fs::remove_file(f2);
			};
			

		match fs::metadata(ziped) {
			Err(_)=>continue,
			Ok(m)=>{
				total_file_size+=m.len();
				if total_file_size>self.service_conf.maxStorageSize || i>self.service_conf.maxStoragePeriod as usize {
					toBeClean = true;
				}
			},
		}
		}
	}

    pub fn PrintLogServiceStats(&self) {
        println!("LogService: Ip={},port={},SysLogServer={:?}\n",
            self.service_conf.self_addr.ip(), self.service_conf.self_addr.port(), 
			self.service_conf.syslog_server);
    
			println!("LogService: PersitentLevel={}, ConsoleLevel={}, max_disk_size={}Bytes,max_Period={} Days\n", 
			self.service_conf.persistentLevel,
            self.service_conf.consoleLevel, self.service_conf.maxStorageSize, self.service_conf.maxStoragePeriod);
    
        println!("LogService: Recv Log Packets={}, bytes={},queue_cap={},len={}\n",
            self.logPackets, self.LogBytes,self.queue.capacity(),self.queue.len());
    
		println!("--------------Log Module----------------");
        for (_, v) in self.logMuduleControl.iter() {
            println!("ModuleName={}, \taddr={}:{}, PersitentLevel={}, ConsoleLevel={},recv_packets={},bytes={}\n", 
				v.name,v.addr.ip(), v.addr.port(), v.persistent_log_level,
				v.console_log_level, v.logPackets, v.logBytes);
        }
		self.logMuduleControl.end_iter();
    }
    

}

static mut gLogServer:Option<log_service_t>=None;
/*init log service,parameter is log_service_config_t*/
pub fn InitLogService(conf:&log_service_config_t) ->errcode::RESULT {
	unsafe {
    if gLogServer.is_none() {
            gLogServer=log_service_t::new(conf);
    }
	}
	let service=match unsafe {&mut gLogServer} {
        None=>{
            return errcode::ERROR_INIT_FAILED;
        },
        Some(s)=>s,
    };
    service.service_conf.logFilePath=formatLogPath(&service.service_conf.logFilePath);

	let ret = service.openLogFile();
	if ret != errcode::RESULT_SUCCESS {
		return ret
	}
	std::thread::spawn(||run_log_service());
	InitLogCleanTask(); //初始化清理任务，定期清理任务

	return errcode::RESULT_SUCCESS
}

fn run_log_service() {
	let service=match unsafe {&mut gLogServer} {
        None=>{
            return ;
        },
        Some(s)=>s,
    };

	loop {
		service.recvLog();
	}
}


/*日志监听器注册，LogService将所有的日志均发送给监听器*/
pub fn RegisterListener(name:&String, listener:&'static mut dyn LogListener)->errcode::RESULT {
	let service=match unsafe {&mut gLogServer} {
        None=>{
            return errcode::ERROR_INIT_FAILED;
        },
        Some(s)=>s,
    };
	
	return service.RegisterListener(name,listener);
}

/*监听器去注册*/
fn DeregisterListener(name:&String)->errcode::RESULT {
	let service=match unsafe {&mut gLogServer} {
        None=>{
            return errcode::ERROR_INIT_FAILED;
        },
        Some(s)=>s,
    };
	
	return service.DeregisterListener(name);
}

pub fn set_syslog_addr(addr:&SocketAddr)->errcode::RESULT {
	let service=match unsafe {&mut gLogServer} {
        None=>{
            return errcode::ERROR_INIT_FAILED;
        },
        Some(s)=>s,
    };
	
	return service.set_syslog_addr(addr);
}
/*格式化处理日志目录，包括处理空路径、添加遗失的"/"等操作*/
fn formatLogPath(filePath:&String)->String {
	let mut new_str = filePath.trim().to_string();

	if new_str.len() == 0 {
		new_str = "/var/log/".to_string();
	} else {
		if !new_str.ends_with("/") {
			new_str +="/";
		}
	}
	return new_str;
}

pub(crate) fn InitLogCleanTask() {
	std::thread::spawn(|| LogCleanTask()); //日志清理的任务
}

/*整理日志文件，仅保留最近30天并不超过总容量的日志文件，并且对每天的日志文件进行压缩处理
对于一天以及以前的文件压缩为.zip文件，并删除原始Log文件*/
fn LogCleanTask() {
	let log_serv = match unsafe{&mut gLogServer} {
		None=>return,
		Some(s)=>s,
	};
	loop {
		thread::sleep(time::Duration::from_secs(3600));
		log_serv.do_clean();
	}
}