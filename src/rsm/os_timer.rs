#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use super::*;
use core::ffi::c_void;
use crate::common::{self,tsidallocator::TsIdAllocator,tsmap::TsHashMap};
use std::mem;
#[cfg(windows)]
use windows_sys::Win32::Foundation::FILETIME;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{self,TP_CALLBACK_INSTANCE,TP_TIMER};

#[cfg(unix)]
use libc;

pub type timer_call_back = fn(timer_id:i32,timer_data:usize);

#[cfg(windows)]
type os_callback_fn = unsafe extern "system" fn(instance: *mut TP_CALLBACK_INSTANCE, context: *mut c_void, timer: *mut TP_TIMER);
#[cfg(windows)]
#[no_mangle]
unsafe extern "system" fn os_timer_callback(_instance: *mut TP_CALLBACK_INSTANCE, 
        context: *mut c_void, _timer: *mut TP_TIMER) {
    if context.is_null() {
        return
    }
    
    let tid = *(context as *const i32);
    //println!("recv timer,os_timer={:?},data={:?},timer_id={},current={}", timer,
    //    context,tid,common::format_datetime(&std::time::SystemTime::now()));    
    let t= match get_timer_by_id(tid) {
        None=>return,
        Some(t)=>t,
    };
    (t.app_callback)(t.timer_id,t.timer_data);
}

#[cfg(unix)]
type os_callback_fn = unsafe extern fn(ev:libc::sigval);
#[cfg(unix)]
#[no_mangle]
unsafe extern "C" fn os_timer_callback(ev:libc::sigval) {
    let tev = &ev;
    if tev.sival_ptr.is_null() {
        return
    }
    let tid = *(tev.sival_ptr as *const i32);
    //println!("recv timer,timer id={:},current={}", tid,
    //    common::format_datetime(&std::time::SystemTime::now()));    
    let t= match get_timer_by_id(tid) {
        None=>return,
        Some(t)=>t,
    };
    (t.app_callback)(t.timer_id,t.timer_data);
}

#[derive(Clone,Debug)]
pub struct os_timer_t {
    dur_msec:u64,    
    timer_data:usize,
    timer_id:i32,
    pub(crate) app_callback:timer_call_back,
    #[cfg(unix)]
    os_timer_id:libc::timer_t,
    #[cfg(windows)]
    os_timer_id:*mut TP_TIMER,
}


impl os_timer_t {
    pub fn new(dur_msec:u64,timer_data:usize,callbackfn:timer_call_back)->Option<Self> {
        let tid = match unsafe { &mut gOsTimerAlloc} {
            None=>return None,
            Some(a)=>{
                let id = a.allocate_id();
                if id==TsIdAllocator::INVALID_ID {
                    return None;
                }
                id
            },
        };
        unsafe {
            gTimerData[tid as usize]=tid;
        }
        let os_tid = match unsafe {
            set_os_timer(dur_msec, std::ptr::addr_of!(gTimerData[tid as usize]) as *mut u8, os_timer_callback)
        } {
            None=>return None,
            Some(t)=>t,
        };

        let tm = Self {
            dur_msec:dur_msec,    
            timer_data:timer_data,
            timer_id:tid,
            os_timer_id:os_tid,
            app_callback:callbackfn,
        };
        if insert_timer(tid, tm.clone()) != errcode::RESULT_SUCCESS {
            return None
        }
        return Some(tm);
    }

    pub fn close(&mut self) {

    }
}

const MAX_OS_TIMER_COUNT:usize = 1024;
static mut gOsTimerAlloc:Option<TsIdAllocator> = None;
static mut gOsTimerMap:Option<TsHashMap<i32,os_timer_t>> = None;
static mut gTimerData:[i32;MAX_OS_TIMER_COUNT+1]=[0;MAX_OS_TIMER_COUNT+1];

pub fn init_os_timer() {
    unsafe {
        if gOsTimerAlloc.is_none() {
            gOsTimerAlloc = Some(TsIdAllocator::new(1, MAX_OS_TIMER_COUNT as i32));
        }
        if gOsTimerMap.is_none() {
            gOsTimerMap = Some(TsHashMap::new(MAX_OS_TIMER_COUNT));
        }
    }    
}

fn insert_timer(tid:i32,timer:os_timer_t)->errcode::RESULT {
    let tm = match unsafe {&mut gOsTimerMap} {
        None=>return errcode::ERROR_NOT_INITIALIZED,
        Some(m)=>m,
    };
    return tm.insert(tid,timer);
}
fn get_timer_by_id(tid:i32)->Option<&'static mut os_timer_t> {
    let tm = match unsafe {&mut gOsTimerMap} {
        None=>return None,
        Some(m)=>m,
    };
    let t = tm.get_mut(&tid);
    return t;
}

#[cfg(unix)]
#[link(name = "os_linux", kind = "static")]
extern "C"  {
    fn c_create_timer(ptr_data:*mut u8,call_back:os_callback_fn,timer_id:*mut libc::timer_t)->i32;
}

#[cfg(unix)]
unsafe fn set_os_timer(dur_msec:u64,data_ptr:*mut u8,call_back:os_callback_fn)->Option<libc::timer_t> {
 
    let mut timer_id=mem::zeroed::<libc::timer_t>();
    /* create timer */
    let res = c_create_timer(data_ptr, call_back,&mut timer_id as *mut libc::timer_t);
    if res<0 {
        println!("error create time,ret={}",res);
        return None;
    }

    let tv_sec = dur_msec as i64 / 1000;
    let tv_nsec=(dur_msec as i64 % 1000)*1000*1000;
    let mut its = libc::itimerspec  {
        it_interval:libc::timespec {
                tv_sec:tv_sec,
                tv_nsec:tv_nsec,
            },
            it_value:libc::timespec {
                tv_sec:tv_sec,
                tv_nsec:tv_nsec,
            },
    };
    let res = libc::timer_settime(timer_id,0,&its as *const libc::itimerspec,std::ptr::null_mut());
    if res<0 {
        println!("error create time,ret={}",res);
    }
    
    return Some(timer_id);

}


#[cfg(windows)]
unsafe fn set_os_timer(dur_msec:u64,pdata:*mut u8,call_back:os_callback_fn)->Option<*mut TP_TIMER> {
    let timer = Threading::CreateThreadpoolTimer(Some(call_back), pdata as *mut c_void,std::ptr::null());
    if timer.is_null() {
        println!("windows set timer failed,err={}",std::io::Error::last_os_error());
        return None;
    }

    println!("windows set timer success,timer={:?}",timer);
    let ftime = FILETIME {
        dwLowDateTime:0xFFFFFFFF,
        dwHighDateTime:0xFFFFFFFF,
    };
    Threading::SetThreadpoolTimer(timer,std::ptr::addr_of!(ftime),dur_msec as u32,0);
    return Some(timer);
}