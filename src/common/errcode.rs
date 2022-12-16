#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
use std::collections::HashMap;
pub type RESULT = i32;
//通用错误
pub const RESULT_SUCCESS: RESULT = 0;
pub const ERROR_COMMON: RESULT = 1;
pub const ERROR_NULL_POINTER: RESULT = 2;
pub const ERROR_INVALID_INDEX: RESULT = 3; //下标越界
pub const ERROR_INVALID_PARAM: RESULT = 4;
pub const ERROR_OUTOF_SCOPE: RESULT = 5;
pub const ERROR_NOT_FOUND: RESULT = 6;
pub const ERROR_ALREADY_EXIST: RESULT = 7;
pub const ERROR_OUTOF_MEM: RESULT = 8; //内存用完
pub const ERROR_NO_PERMISSION: RESULT = 9; //没有权限
pub const ERROR_MSG_TOO_LONG: RESULT = 10; //消息、报文太长
pub const ERROR_ENCODE_MSG: RESULT = 11; //编码失败
pub const ERROR_DECODE_MSG: RESULT = 12; //解码失败
pub const ERROR_COLLISION: RESULT = 13; //数据、逻辑冲突
pub const ERROR_AUTH_FAILED: RESULT = 14; //认证失败
pub const ERROR_THRESHOLD_EXCEED: RESULT = 15; //超过门限
pub const ERROR_TIME_OUT: RESULT = 16; //超时
pub const ERROR_NOT_INITIALIZED: RESULT = 17; //没有初始化
pub const ERROR_INVALID_MSG: RESULT = 18;
pub const ERROR_INVALID_STATE: RESULT = 19; //无效状态
pub const ERROR_LOCK_FAILED: RESULT = 20;
pub const ERROR_MSG_TOO_SHORT: RESULT = 21;
pub const ERROR_NOT_SUPPORT: RESULT = 22; //不支持的功能
pub const ERROR_OS_CALL_FAILED: RESULT = 23; //OS调用失败
pub const ERROR_NO_OP: RESULT = 24; //没有发生实际动作
pub const ERROR_LINK_BROKEN: RESULT = 25; //链路中断


pub const ERROR_BUFFER_TOO_SMALL: RESULT = 26; //缓冲区太小
pub const ERROR_INIT_FAILED: RESULT = 27; //初始化失败
pub const ERROR_NO_DATA: RESULT = 28; //没有数据

pub const ERROR_INVALID_IPADDR: RESULT = 50; //Invalid Ip Addr
pub const ERROR_INVALID_MAC_ADDR: RESULT = 51;
pub const ERROR_DEVICE_NOT_EXIST:RESULT=52;
//I/O类错误
pub const ERROR_OPEN_FILE: RESULT = 100;
pub const ERROR_FILE_NOT_FOUND: RESULT = 101;
pub const ERROR_DISK_FULL: RESULT = 102;
pub const ERROR_SEND_MSG: RESULT = 103;
pub const ERROR_RECV_MSG: RESULT = 104;
pub const ERROR_BIND_SOCKET: RESULT = 105;
pub const ERROR_CONNECTION: RESULT = 106;
pub const ERROR_RPC_FAILED: RESULT = 107;
pub const ERROR_FILE_EXISTS: RESULT = 108;
pub const ERROR_WRITE_FILE_FAILED: RESULT = 109;
//应用特定错误

//HTTP协议常见错误
pub const HTTP_SUCCESS: i16 = 200;
pub const HTTP_MOVED: i16 = 300;

pub const HTTP_BAD_REQUEST: i16 = 400;
pub const HTTP_UNAUTHORIZED: i16 = 401;
pub const HTTP_PAYMENT_REQUIRED: i16 = 402;
pub const HTTP_FORBIDDEN: i16 = 403;
pub const HTTP_NOT_FOUND: i16 = 404;
pub const HTTP_METHOD_NOT_ALLOWED: i16 = 405;
pub const HTTP_REQUEST_TIMEOUT: i16 = 408;

pub const HTTP_INTERNAL_ERROR: i16 = 500;
pub const HTTP_SERVER_NOT_IMPLEMENT: i16 = 501;
pub const HTTP_SERVER_NOT_AVAILABLE: i16 = 503;

static mut  ErrorNameMap:Option<HashMap<i32,&str>>=None;

fn init_error_map() {
    match unsafe {&ErrorNameMap} {
        Some(_)=>return,
        None=>(),
    }

    let erm = HashMap::from([
        (RESULT_SUCCESS,"Success"),
        (ERROR_COMMON,"General failure"),
        (ERROR_NULL_POINTER,"Null Pointer"),
        (ERROR_INVALID_INDEX,"Invalid Index",),
        (ERROR_INVALID_PARAM,"Invalid parameter"),
        (ERROR_OUTOF_SCOPE,"Out of Scope"),
        (ERROR_NOT_FOUND,"Not Found"),
    (ERROR_ALREADY_EXIST,"Already Exist"),
    (ERROR_OUTOF_MEM,"Out of memory"), 
    (ERROR_NO_PERMISSION,"No Permission"),
    (ERROR_MSG_TOO_LONG,"Msg too long"),
    (ERROR_ENCODE_MSG,"Encode Message failed"),
    (ERROR_DECODE_MSG,"Decode Message failed"),
    (ERROR_COLLISION,"Collision Occured"),
    (ERROR_AUTH_FAILED,"Authentication Failed"),
    (ERROR_THRESHOLD_EXCEED,"Threshhold Exceed"),
    (ERROR_TIME_OUT,"Time Out"),
    (ERROR_NOT_INITIALIZED,"Not Initialized"),
    (ERROR_INVALID_MSG,"Invalid Message"), 
    (ERROR_INVALID_STATE,"Invalid State"),
    (ERROR_LOCK_FAILED,"lock Failed"), 
    (ERROR_MSG_TOO_SHORT,"Message Too Short"),
    (ERROR_NOT_SUPPORT,"Not Support"),
    (ERROR_OS_CALL_FAILED,"Os Call Failed"), 
    (ERROR_NO_OP,"No Operation"), 

    (ERROR_LINK_BROKEN,"Link Broken"), 
    (ERROR_OPEN_FILE,"Failed to Open File"), 
    (ERROR_FILE_NOT_FOUND,"File Not Found"), 
    (ERROR_DISK_FULL,"Disk is Full"),
    (ERROR_SEND_MSG,"Send Message Failed"),
    (ERROR_RECV_MSG,"Recv message Failed"),
    (ERROR_BIND_SOCKET,"Bind Socket Failed"), 
    (ERROR_CONNECTION,"Connection Error"),

    ]);
        unsafe {
            ErrorNameMap = Some(erm);
        }
    
}

pub fn errcode_to_string(code:RESULT)->&'static str {
    unsafe {
    if ErrorNameMap.is_none() {
        init_error_map();        
    }
    }
    if let Some(errm)=unsafe {&ErrorNameMap} {
        match errm.get(&code) {
            None=>return "Unknown Error",
            Some(e)=>return e,
        }
    } else {
        return "Unknown Error"
    }


}