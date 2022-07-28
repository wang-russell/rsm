#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

//rsm scheduler, manager task control block, schedule task according to the message
use super::{*, task::task_stats_t, oam::E_RSM_OAM_OP};
use common::{tsmap::TsHashMap,errcode};
use task::task_t;
#[cfg(windows)]
use windows_sys::Win32::System::Threading;
#[cfg(unix)]
use libc;
use common::sched;

struct component_registry_t {
    cattr:component_attrs_t,
    new_task:rsm_new_task,
}

const RSM_SCHED_TASK_URL:&str="/task";
const RSM_SCHED_COMPONENT_URL:&str="/component";

static mut gComponentRegistry:Option<TsHashMap<u32,component_registry_t>>=None;

static mut gTaskRegistry:Option<TsHashMap<rsm_component_t,task_t>>=None;
static mut gTaskIdMap:Option<TsHashMap<sched::os_task_id_t,rsm_component_t>>=None;

///initialize the scheduler
pub fn init_scheduler(max_component:usize) {
    unsafe {
        gComponentRegistry = Some(TsHashMap::new(max_component));
        gTaskRegistry = Some(TsHashMap::new(max_component*4));
        gTaskIdMap = Some(TsHashMap::new(max_component*4));
    }
    let urls = [RSM_SCHED_TASK_URL.to_string(),RSM_SCHED_COMPONENT_URL.to_string()];
    oam::RegisterOamModule(&urls, process_sched_oam);

}

fn register_task(attrs:&component_attrs_t,callback:rsm_new_task)->errcode::RESULT {
    let ptMap = match unsafe { &mut gTaskRegistry } {
        None=>return errcode::ERROR_NOT_INITIALIZED,
        Some(m)=>m,
    };

    for i in 0..attrs.inst_num {
        let tid = rsm_component_t::new(attrs.cid,1,i+1);
        let cb_inst = (callback)(&tid);
        let task = task_t::new(&tid, attrs.qlen, attrs.priority,cb_inst);
        ptMap.insert(tid,task);
    }
    errcode::RESULT_SUCCESS
}

///register one component to scheduler
pub fn registry_component(cid:u32,attrs:&component_attrs_t,callback:rsm_new_task)->errcode::RESULT {
    let pcMap = match unsafe { &mut gComponentRegistry } {
        None=>return errcode::ERROR_NOT_INITIALIZED,
        Some(m)=>m,
    };
    
    let r = component_registry_t {
        cattr:attrs.clone(),
        new_task:callback,
    };

    let res = pcMap.insert(cid,r);
    if res!=errcode::RESULT_SUCCESS {
        return res;
    }
    register_task(attrs, callback);
    errcode::RESULT_SUCCESS
}



///run the scheduler
pub fn run() {
    let tEntries = match unsafe {&mut gTaskRegistry} {
        None=>return,
        Some(e)=>e,
    };

    for (t,_) in tEntries.iter() {
        println!("Spawn a task,id={},inst={}",t.cid,t.inst_id);
        std::thread::spawn(|| schedule_task(t.clone()));
    }
    tEntries.end_iter();
}

/// schedule task for each task
fn schedule_task(task_id:rsm_component_t) {
    let gTask = match unsafe {&mut gTaskRegistry} {
        None=>return,
        Some(e)=>e,
    };
    let t = match gTask.get_mut(&task_id) {
        None=> {
            println!("Running a task,id={},inst={} error, not found in task registry",task_id.cid,task_id.inst_id);
            return 
        },
        Some(v)=>v,
    };
    println!("Running a task,id={},inst={}",task_id.cid,task_id.inst_id);
    let os_tid = sched::get_self_os_task_id();
    if let Some(tm) = unsafe { & mut gTaskIdMap} {
        tm.insert(os_tid,task_id.clone());
    }

    t.run();
}

///get self component id, get None if not under the rsm thread context
pub(crate) fn get_self_cid()->Option<rsm_component_t> {
    let os_tid = sched::get_self_os_task_id();

    if let Some(tm) = unsafe {&mut gTaskIdMap} {
        return match tm.get(&os_tid) {
            None=> None,
            Some(tid)=>Some(tid.clone()),
        };
    }   
    None
}

fn get_task_inst(tid:&rsm_component_t)->Option<&mut task_t> {
  let tm = match unsafe {&mut gTaskRegistry} {
    None=>return None,
    Some(t)=>t,
  };
  return tm.get_mut(&tid);
}

pub(crate) fn get_sender_cid()->Option<rsm_component_t> {
    let self_cid = match get_self_cid() {
        None=>return None,
        Some(c)=>c,
    };
    match get_task_inst(&self_cid) {
        None=>return None,
        Some(t)=>return match t.get_sender_cid() {
            None=>None,
            Some(c)=>Some(c.clone()),
        },
    }
}


pub(crate) fn power_on_ack() {

}

pub(crate) fn send_asyn_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT {
    let task = match unsafe {&mut gTaskRegistry} {
        None=>return errcode::ERROR_NOT_INITIALIZED,
        Some(t)=> match t.get_mut(dst) {
            None=>return errcode::ERROR_NOT_FOUND,
            Some(tk)=>tk,
        },
    };

    return task.send_asyn_msg(msg);
}

///send one high priority message to specific component
pub(crate) fn send_asyn_priority_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT {
    let task = match unsafe {&mut gTaskRegistry} {
        None=>return errcode::ERROR_NOT_INITIALIZED,
        Some(t)=> match t.get_mut(dst) {
            None=>return errcode::ERROR_NOT_FOUND,
            Some(tk)=>tk,
        },
    };

    return task.send_asyn_priority_msg(msg);
}

fn get_component_registry(cid:u32)->Option<&'static component_registry_t> {
    let cm = match unsafe {& gComponentRegistry} {
        None=>return None,
        Some(c)=>c,
    };
    match cm.get(&cid) {
        None=>return None,
        Some(c)=>return Some(c),
    }
}
fn get_task_stats(tid:&rsm_component_t)->Option<task_stats_t> {
    let task = match unsafe {&mut gTaskRegistry} {
        None=>return None,
        Some(t)=> match t.get_mut(tid) {
            None=>return None,
            Some(tk)=>tk,
        },
    };
    return Some(task.get_task_stats())
}

#[cfg(windows)]
pub (crate) fn map_os_priority(priority:E_RSM_TASK_PRIORITY)->(i32,i32) {
	let  (policy,sys_pri) = match priority {
	    E_RSM_TASK_PRIORITY::THREAD_PRI_LOW=>(0,Threading::THREAD_PRIORITY_BELOW_NORMAL),
        E_RSM_TASK_PRIORITY::THREAD_PRI_NORMAL=>(0,Threading::THREAD_PRIORITY_NORMAL),
        E_RSM_TASK_PRIORITY::THREAD_PRI_HIGH=>(0,Threading::THREAD_PRIORITY_ABOVE_NORMAL),
        E_RSM_TASK_PRIORITY::THREAD_PRI_REALTIME=>(0,Threading::THREAD_PRIORITY_TIME_CRITICAL),
        E_RSM_TASK_PRIORITY::THREAD_PRI_REALTIME_HIGH=>(0,Threading::THREAD_PRIORITY_TIME_CRITICAL),
        E_RSM_TASK_PRIORITY::THREAD_PRI_REALTIME_HIGHEST=>(0,Threading::THREAD_PRIORITY_TIME_CRITICAL),
        _=>(0,Threading::THREAD_PRIORITY_NORMAL),
	};

	return (policy,sys_pri)
}

#[cfg(unix)]
pub (crate)fn map_os_priority(priority:E_RSM_TASK_PRIORITY)->(i32,i32) {
	let  (policy,sys_pri) = match priority {
	    E_RSM_TASK_PRIORITY::THREAD_PRI_LOW=>(libc::SCHED_OTHER,0),
        E_RSM_TASK_PRIORITY::THREAD_PRI_NORMAL=>(libc::SCHED_OTHER,50),
        E_RSM_TASK_PRIORITY::THREAD_PRI_HIGH=>(libc::SCHED_OTHER,80),
        E_RSM_TASK_PRIORITY::THREAD_PRI_REALTIME=>(libc::SCHED_RR,10),
        E_RSM_TASK_PRIORITY::THREAD_PRI_REALTIME_HIGH=>(libc::SCHED_RR,50),
        E_RSM_TASK_PRIORITY::THREAD_PRI_REALTIME_HIGHEST=>(libc::SCHED_RR,99),
        _=>(libc::SCHED_OTHER,0),
	};

	return (policy,sys_pri)
}

///Oam Request Process
fn process_sched_oam(op:oam::E_RSM_OAM_OP,url:&String,param:&String)->oam::oam_cmd_resp_t {
    let mut tResp = oam::oam_cmd_resp_t::new(errcode::ERROR_NOT_FOUND,&String::default());

    match url.as_str() {
        RSM_SCHED_TASK_URL=>{
            proccess_task_oam(op,param,&mut tResp);        

        },
        RSM_SCHED_COMPONENT_URL=>{
            proccess_component_oam(op,param,&mut tResp); 
        },
        _=>(),
    }

    return tResp;
}

fn proccess_task_oam(op:oam::E_RSM_OAM_OP,param:&String,tResp:&mut oam::oam_cmd_resp_t) {
    if op!=E_RSM_OAM_OP::CLI_OP_SHOW {
        tResp.RetCode = errcode::ERROR_NOT_SUPPORT;
        return;
    }
    tResp.RetCode=errcode::ERROR_NOT_FOUND;
    let idx = match param.find(":") {
        None=> {            
            return;
        },
        Some(i)=>i,
    };

    let cid = match u32::from_str_radix(&param.as_str()[0..idx],10) {
        Ok(d)=>d,
        Err(_)=>return,
    };
    
    let inst = match u32::from_str_radix(&param.as_str()[idx+1..],10) {
        Ok(d)=>d,
        Err(_)=>return,
    };
    let tid = &rsm_component_t::new(cid, 1, inst as usize);
    if let Some(stats) = get_task_stats(&tid) {
        tResp.RetCode=errcode::RESULT_SUCCESS;
        tResp.Description = serde_json::to_string_pretty::<task_stats_t>(&stats).unwrap();
    }

}

fn proccess_component_oam(op:oam::E_RSM_OAM_OP,param:&String,tResp:&mut oam::oam_cmd_resp_t) {
    
    if op!=E_RSM_OAM_OP::CLI_OP_SHOW {
        tResp.RetCode = errcode::ERROR_NOT_SUPPORT;
        return;
    }
    tResp.RetCode = errcode::ERROR_NOT_FOUND;
    let cid = match u32::from_str_radix(param.as_str(), 10) {
        Err(_)=>{            
            return;
        },
        Ok(v)=>v,
    };
    if let Some(c) = get_component_registry(cid) {
        tResp.RetCode=errcode::RESULT_SUCCESS;
        tResp.Description = serde_json::to_string_pretty::<component_attrs_t>(&c.cattr).unwrap();
    }
    
}