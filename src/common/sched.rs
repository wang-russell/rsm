#[cfg(unix)]
use libc;
#[cfg(unix)]
use std::mem;
#[cfg(unix)]
use libc::{cpu_set_t};
#[cfg(windows)]
use windows_sys::Win32::System::Threading;
use super::errcode;

#[derive(Debug)]
pub struct sys_info_t {
    pub uptime: i64,
    pub loads: [u64; 3],
    pub totalram: u64,
    pub freeram: u64,
    pub sharedram: u64,
    pub bufferram: u64,
    pub totalswap: u64,
    pub freeswap: u64,
    pub procs: u16,
    pub totalhigh: u64,
    pub freehigh: u64,
    pub mem_unit: u32,
}

#[cfg(unix)]
impl sys_info_t {
    pub fn from_sys_info(info:&libc::sysinfo)->Self {
        return Self {
            uptime: info.uptime,
            loads: info.loads,
            totalram: info.totalram,
            freeram: info.freeram,
            sharedram: info.sharedram,
            bufferram: info.bufferram,
            totalswap: info.totalswap,
            freeswap: info.freeswap,
            procs: info.procs,
            totalhigh: info.totalhigh,
            freehigh: info.freehigh,
            mem_unit: info.mem_unit,
        }
    }
}

#[cfg(unix)]
pub fn get_sys_info()->sys_info_t {
    unsafe {
        let mut info = mem::zeroed::<libc::sysinfo>();
        libc::sysinfo(&mut info as * mut libc::sysinfo);
        return sys_info_t::from_sys_info(&info)
    }
}

#[cfg(unix)]pub fn get_sys_cpu_num()->u16 {

        let count = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
        return count as u16;
}


#[cfg(windows)]
pub fn get_sys_cpu_num()->usize {
    0
}


///设置指定线程的CPU亲和性
#[cfg(unix)]
pub fn set_thread_cpu_affinity(threadId:u64,bitmask:u64)->i32 {
    unsafe {
        let cpus = build_cpu_set(bitmask);
        return libc::pthread_setaffinity_np(threadId,std::mem::size_of::<cpu_set_t>(),&cpus as *const cpu_set_t);

    }
}

#[cfg(windows)]
pub fn get_sys_info()->sys_info_t {
    unsafe {
        return std::mem::zeroed::<sys_info_t>();
    }
}

///设置指定线程的CPU亲和性
#[cfg(windows)]
pub fn set_thread_cpu_affinity(_threadId:u64,_bitmask:u64)->i32  {
    0

}

///设置自身的CPU亲和性
#[cfg(unix)]
pub fn set_self_cpu_affinity(bitmask:u64) {
    unsafe {
        let threadId = libc::pthread_self();
        set_thread_cpu_affinity(threadId,bitmask);
    }
}

///设置自身的CPU亲和性
#[cfg(windows)]
pub fn set_self_cpu_affinity(_bitmask:u64) {

}

#[cfg(unix)]
fn build_cpu_set(bitmask:u64)->cpu_set_t {
    let mut cpu_set = unsafe { std::mem::zeroed::<cpu_set_t>()};

    for i in 0..64 {
        if bitmask & (1u64<<i) != 0{
            unsafe { libc::CPU_SET(i,&mut cpu_set) };
        }
    }
    return cpu_set;

}


#[cfg(unix)]
pub fn get_self_threadId()->usize {
    unsafe {
        libc::pthread_self() as usize
    }
}

#[cfg(windows)]
pub fn get_self_threadId()->usize {
    0
}

#[cfg(unix)]
pub fn set_self_priority(policy:i32,priority:i32)->errcode::RESULT {

    let mut sparam = unsafe { mem::zeroed::<libc::sched_param>() };
    sparam.sched_priority=priority;

    let ret = unsafe { libc::sched_setscheduler(0,policy,&sparam) };
    if ret!=0 {
        return errcode::ERROR_OS_CALL_FAILED
    }
    return errcode::RESULT_SUCCESS;

}

/*设置自身线程的优先级*/
#[cfg(windows)]
pub fn set_self_priority(_policy:i32,priority:i32)->errcode::RESULT {
	let h = unsafe { Threading::GetCurrentThread() };
	let res = unsafe { Threading::SetPriorityClass(h, priority as u32) };
	if res == 0 {
		return errcode::ERROR_OS_CALL_FAILED
	}

	return errcode::RESULT_SUCCESS

}

#[cfg(windows)]
pub type os_task_id_t = isize;
#[cfg(unix)]
pub type os_task_id_t = libc::pid_t;


#[cfg(unix)]
pub fn get_self_os_task_id()->os_task_id_t {
    unsafe { 
        return libc::gettid()
    }
}
#[cfg(windows)]
pub fn get_self_os_task_id()->os_task_id_t {
    unsafe { 
    return Threading::GetCurrentThread()
    }
}