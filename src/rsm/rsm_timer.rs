#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

///rsm timer manager
/// Application call set_timer to activate a timer for specific duration and loop_count, and get rsm_timer_id_t 
/// set_timer(dur_msec:u64,loop_count:u64,timer_data:usize)->Option<rsm_timer_id_t>
/// later application can kill the timer by invoke kill_timer_by_id
/// Once the timer is set, application can get a timer event on the Runnable Trait called by RSM
/// 

use super::*;
use crate::common::{self,errcode,tsidallocator::TsIdAllocator,tsmap::TsHashMap};
use os_timer;
const MAX_TIMER_COUNT:usize = 192*1024;
const MAX_TIMER_PER_CAT:usize = 64*1024;
const INVALID_TIMER_ID:rsm_timer_id_t = TsIdAllocator::INVALID_ID;

const TIMER_CAT_1MS:u8=1;
const TIMER_CAT_100MS:u8=2;
const TIMER_CAT_1S:u8=3;

#[derive(Clone,Default)]
pub struct timer_stats_t {
    total:usize,
    timer_count_1ms:usize,
    timer_count_100ms:usize,
    timer_count_1s:usize,
}

struct timer_desc_t {
    id:rsm_timer_id_t,
    duration_msec:u64,
    loop_count:u64,
    timer_data:usize,
    last_fired:u64,
    expired_count:u64,
    cid:rsm_component_t,
}

type timer_hash_map = TsHashMap<rsm_timer_id_t,timer_desc_t>;
static mut gTimerIdAlloc:Option<TsIdAllocator>=None;
static mut gTimerCatMap:[u8;MAX_TIMER_COUNT+1]=[0;MAX_TIMER_COUNT+1];
static mut gTimer1ms:Option<timer_hash_map>=None;
static mut gTimer100ms:Option<timer_hash_map>=None;
static mut gTimer1s:Option<timer_hash_map>=None;

///initialize timer
pub(crate) fn init_timer() {
    unsafe {
        if gTimerIdAlloc.is_some() {
            return
        }
        gTimerIdAlloc = Some(TsIdAllocator::new(1, MAX_TIMER_COUNT as i32));
        gTimer1ms = Some(TsHashMap::new(MAX_TIMER_PER_CAT));
        gTimer100ms = Some(TsHashMap::new(MAX_TIMER_PER_CAT));
        gTimer1s = Some(TsHashMap::new(MAX_TIMER_PER_CAT));
    }

    println!("RSM Init Timer finished");
}

fn get_timer_cb_by_duration(dur_msec:u64)->Option<&'static mut timer_hash_map> {
    if dur_msec<10 {
        return unsafe {(&mut gTimer1ms).as_mut()}
    } else if dur_msec<100{
        return unsafe {(&mut gTimer100ms).as_mut()}
    }
    return unsafe {(&mut gTimer1s).as_mut()}
}

fn get_timer_cb_by_cat(timer_cat:u8)->Option<&'static mut timer_hash_map> {
    match timer_cat {
        TIMER_CAT_1MS=> return unsafe {(&mut gTimer1ms).as_mut()},
        TIMER_CAT_100MS=> return unsafe {(&mut gTimer100ms).as_mut()},
        TIMER_CAT_1S=> return unsafe {(&mut gTimer1s).as_mut()},
        _=>None, 
    }    
}

fn get_timer_cat(dur_msec:u64)->u8 {
    if dur_msec<10 {
        return TIMER_CAT_1MS
    } else if dur_msec<100{
        return TIMER_CAT_100MS
    }
    return TIMER_CAT_1S 
}

fn set_timer_cat_map(id:rsm_timer_id_t,cat:u8)->errcode::RESULT {
    if cat<TIMER_CAT_1MS || cat>TIMER_CAT_1S || id as usize>MAX_TIMER_COUNT{
        return errcode::ERROR_INVALID_PARAM
    }
    unsafe {
        gTimerCatMap[id as usize] = cat;
    }

    errcode::RESULT_SUCCESS
}
///set a timer, loop_count=1 indicate a one time timer, 0-loop forever
pub fn set_timer(dur_msec:u64,loop_count:u64,timer_data:usize)->Option<rsm_timer_id_t> {
    let ida = match unsafe { &mut gTimerIdAlloc} {
        None=>return None,
        Some(a)=> a,
    };
    let id = ida.allocate_id();
    if id==INVALID_TIMER_ID {
        return None;
    }
    let caller = match rsm_sched::get_self_cid() {
        None=>return None,
        Some(c)=>c,
    };
    let timer = timer_desc_t {
        id:id,
        duration_msec:dur_msec,
        loop_count:loop_count,
        timer_data:timer_data,
        last_fired:get_inner_time_stamp(),
        cid:caller,
        expired_count:0,
    };

    let timer_cat = get_timer_cat(dur_msec);
    let timer_map = match get_timer_cb_by_cat(timer_cat) {
        None=>return None,
        Some(m)=>m,
    };
    if timer_map.insert(id,timer)!=errcode::RESULT_SUCCESS {
        ida.release_id(id);
        return None        
    }
    set_timer_cat_map(id, timer_cat);
    return Some(id)
}

pub fn kill_timer_by_id(timer_id:rsm_timer_id_t)->errcode::RESULT {
    if timer_id as usize> MAX_TIMER_COUNT {
        return errcode::ERROR_INVALID_PARAM
    }

    let cid=match rsm_sched::get_self_cid() {
        None=>return errcode::ERROR_INVALID_STATE,
        Some(c)=>c,
    };
    let cat = unsafe {gTimerCatMap[timer_id as usize]};
    let pMap = match get_timer_cb_by_cat(cat) {
        None=>return errcode::ERROR_NOT_FOUND,
        Some(m)=>m
    };
    
    if let Some(timer) = pMap.get(&timer_id) {
        if timer.cid !=cid {
            return errcode::ERROR_INVALID_STATE;
        }
    } else {
        return errcode::ERROR_NOT_FOUND
    }
    pMap.remove(&timer_id);

    match unsafe { &mut gTimerIdAlloc} {
        None=>(),
        Some(a)=>{
            a.release_id(timer_id);
        },
    }
    errcode::RESULT_SUCCESS
}

///run timer schedule task,scan the allocated timer map, send message to correspondant task
pub(crate) fn start_timer_thread() {

     init_timer();
   
    std::thread::spawn(move || {
        timer_loop()
    });
}
static mut cur_time_stamp:u64=0;
fn incr_inner_time_stamp() {
    //most os use ticks as min timer unit
    unsafe {
        cur_time_stamp=common::get_now_usec64()/1000;
    }
}
fn get_inner_time_stamp()->u64 {
    unsafe {
        return cur_time_stamp;
    }
}

const TIMER_TASK_INNER:usize = 1;
const TIMER_TASK_1MS:usize = 2;
const TIMER_TASK_100MS:usize = 3;
const TIMER_TASK_1S:usize = 4;
fn timer_loop() {
    os_timer::init_os_timer();
    let _tm = match os_timer::os_timer_t::new(1,TIMER_TASK_INNER,scan_timer_call_back) {
        None=> {
            println!("set time failed");
            return
        },
        Some(t)=>t,
    };
    os_timer::os_timer_t::new(1,TIMER_TASK_1MS,scan_timer_call_back);
    os_timer::os_timer_t::new(1,TIMER_TASK_100MS,scan_timer_call_back);
    os_timer::os_timer_t::new(1,TIMER_TASK_1S,scan_timer_call_back);
    loop {
       std::thread::sleep(std::time::Duration::from_millis(1000));       
    }
}

///send timer message
fn send_timer_msg(cid:&rsm_component_t,tid:rsm_timer_id_t,timer_data:usize) {
    let timer_msg = match rsm_message_t::new_timer_msg(tid, timer_data) {
        None=>return,
        Some(m)=>m,
    };
    rsm_sched::send_asyn_priority_msg(cid, timer_msg);
}

///os timer callback
fn scan_timer_call_back(_timerId:i32,timer_data:usize) {
    match timer_data {
        TIMER_TASK_INNER=>incr_inner_time_stamp(),
        TIMER_TASK_1MS=> {            
            scan_timer_1ms();
        },
        TIMER_TASK_100MS=>scan_timer_100ms(),
        TIMER_TASK_1S=>scan_timer_1s(),
        _=>(),
    }
}
///scan 1ms timer, and send timer message to specific component
fn scan_timer_1ms() {
    let tMap = match unsafe { &mut gTimer1ms} {
        None=>return,
        Some(m)=>m,
    };
    scan_timer_proc(tMap);
}

fn scan_timer_100ms() {
    let tMap = match unsafe { &mut gTimer100ms} {
        None=>return,
        Some(m)=>m,
    };
    scan_timer_proc(tMap);   
}

fn scan_timer_1s() {
    let tMap = match unsafe { &mut gTimer1s} {
        None=>return,
        Some(m)=>m,
    };
    scan_timer_proc(tMap);   
}

fn scan_timer_proc(timer_map:&mut timer_hash_map) {
    let cur = get_inner_time_stamp();//common::get_now_usec64();
    let mut to_delete=Vec::new();
    for (id,t) in timer_map.iter_mut() {
        if cur>=t.last_fired+t.duration_msec {
            send_timer_msg(&t.cid, *id, t.timer_data);
            t.last_fired=cur;
            t.expired_count+=1;
            if t.loop_count>0 && t.expired_count>=t.loop_count {
                to_delete.push(t.id);
            }
            
        }
    }
    timer_map.end_iter();
    // if expiry count large than expected, remove the timer
    for i in to_delete {
        timer_map.remove(&i);
    }
}

pub fn get_timer_stats()->timer_stats_t {
    let mut stats = timer_stats_t::default();
    unsafe {
        stats.total =  match &gTimerIdAlloc {
            None=>0,
            Some(ids)=>ids.used_count() as usize,
        };

        stats.timer_count_1ms = match &gTimer1ms {
            None=>0,
            Some(tm)=>tm.len(),
        };

        stats.timer_count_100ms = match &gTimer100ms {
            None=>0,
            Some(tm)=>tm.len(),
        };
        stats.timer_count_1s = match &gTimer1s {
            None=>0,
            Some(tm)=>tm.len(),
        };
    }
    

    return stats
}
