#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use time;
use std::time::{self as std_time, SystemTime};

pub mod errcode;
pub mod tsidallocator;
pub use tsidallocator::TsIdAllocator;

pub mod tsmap;
pub use tsmap::TsHashMap;
pub mod tsqueue;
pub use tsqueue::TsDequeue;

pub mod indexring;
pub mod rawstring;
pub mod atomicqueue;
pub use atomicqueue::AtomicDequeue;

pub mod spin_lock;
pub use spin_lock::spin_lock_t;

pub mod sched;
pub mod bitmap;
pub use bitmap::bitmap_t;

pub mod ringbuf;
pub use ringbuf::ring_buffer_t;
pub use ringbuf::ts_ring_buffer_t;

pub mod uuid;
pub use uuid::uuid_t;

pub type rsm_time_t = time::OffsetDateTime;

pub const UNIX_EPOCH_STRING:&str = "1970-1-1 00:00:00.000";
#[inline(always)]
pub fn get_now_usec64() -> u64 {
    match SystemTime::UNIX_EPOCH.elapsed() {
        Err(_) => 0,
        Ok(d) => d.as_micros() as u64,
    }
}
#[inline(always)]
pub fn get_time_from_usec(usec: u64) -> SystemTime {
    match SystemTime::UNIX_EPOCH.checked_add(std_time::Duration::from_micros(usec)) {
        None => std_time::SystemTime::UNIX_EPOCH,
        Some(t) => t,
    }
}
const INVALID_MONTH:u16=0xFFFF;

fn get_month(day_in_year:u16,leap_year:bool)->(u16,u16) {
    const days_per_mon1:[u16;12]=[31,28,31,30,31,30,31,31,30,31,30,31];
    const days_per_mon2:[u16;12]=[31,29,31,30,31,30,31,31,30,31,30,31];
    let days=if leap_year {&days_per_mon2} else {&days_per_mon1};
    let mut cuml = 0;
    for i in 0..12 {
        cuml+=days[i];
        if day_in_year<cuml {
            return ((i+1) as u16,day_in_year+1 -(cuml-days[i]));
        }
        
    }
    return (INVALID_MONTH,0);
}
pub fn is_leap_year(year:u64)->bool {
    if ((year % 4 ==0) && (year % 100!=0)) || (year % 400==0) {
        return true
    } else {
        return false;
    }
}
fn get_years_of_days(days:u64)->u64 {
    if days % 365 >58 {
        return days/365
    } else {
        days/365+1
    }
}
pub fn ceiling(a:u64,b:u64)->u64 {
    if a % b==0 {
        return a/b
    } else {
        return a/b+1
    }
}
//返回Linux Epoch 1970-1-1以来总共经过的年数，以及这些年所包含的实际天数
fn get_years_since_1970(days:u64)->(u64,u64) {
    let mut years = get_years_of_days(days);
    let leap_years = (years+2)/4 - (70+years)/100+(370+years)/400;

    years =  (days-leap_years)/365;
    return (years,years*365+leap_years);
}
const SECS_PER_DAY:u64=24*3600;
#[derive(Clone)]
pub struct datetime_t {
    systime:std_time::SystemTime,
    year:u32,
    mon:u8,
    day:u8,
    hour:u8,
    min:u8,
    sec:u8,
    msec:u16,
}

impl datetime_t {
    pub fn new()->Self {
        return datetime_t{
            systime:std_time::UNIX_EPOCH,
            year:1970,
            mon:1,
            day:1,
            hour:0,
            min:0,
            sec:0,
            msec:0,
        }   
    }

    pub fn get_year(&self)->u32 {
        self.year
    }
    pub fn get_mon_in_year(&self)->u8 {
        self.mon
    }
    pub fn get_day_in_mon(&self)->u8 {
        self.day
    }

    pub fn get_hour_in_day(&self)->u8 {
        self.hour
    }
    pub fn get_min_in_hour(&self)->u8 {
        self.min
    }
    pub fn get_secs_in_min(&self)->u8 {
        self.sec
    }
    pub fn get_day_in_year(&self)->u16 {
        const days_per_mon1:[u16;12]=[31,28,31,30,31,30,31,31,30,31,30,31];
        const days_per_mon2:[u16;12]=[31,29,31,30,31,30,31,31,30,31,30,31];
        let pCount = if is_leap_year(self.year as u64) {
            &days_per_mon2
        } else {
            &days_per_mon1
        };
        let mut days_in_year = 0;
        for i in 0..self.mon as usize {
            days_in_year+=pCount[i];
        }
        return days_in_year+self.day as u16;
    }

    pub fn prev_ndays(&self,days:u64)->Self {
        let prev_time=match self.systime.checked_sub(std_time::Duration::from_secs(SECS_PER_DAY*days)) {
            Some(d)=>d,
            None=>self.systime.clone(),
        };

        return get_datetime_from_std(&prev_time)
    }

    pub fn prev_secs(&self,secs:u64)->Self {
        let prev_time=match self.systime.checked_sub(std_time::Duration::from_secs(secs)) {
            Some(d)=>d,
            None=>self.systime.clone(),
        };

        return get_datetime_from_std(&prev_time)
    }

    pub fn to_usecs(&self)->u64 {
        match self.systime.duration_since(std_time::UNIX_EPOCH) {
            Err(_) => return 0,
            Ok(d) => d.as_micros() as u64,
        }
    }
}


pub fn get_datetime_from_std(dt:&std_time::SystemTime) ->datetime_t {
    let mut offset = 0;
    if let Ok(local) = time::UtcOffset::current_local_offset() {
        offset=local.whole_seconds();
    }

    let dur = (dt.duration_since(std_time::UNIX_EPOCH).unwrap().as_millis() as i64 + (offset as i64)*1000) as u64 ;
    if dur==0 {
        return datetime_t::new();
    }
    let total_days = dur/(SECS_PER_DAY*1000);

    let (years,days_elapsed)=get_years_since_1970(total_days);
    let days_in_year= total_days-days_elapsed;
    let leap_year = is_leap_year(1970+years);
    let (mon,day_in_mon) = get_month(days_in_year as u16, leap_year);
    let mut msec = dur % (SECS_PER_DAY*1000);
    let hour = msec /(3600*1000);
    msec-=hour*3600*1000;
    let min = msec / (60*1000);
    msec-=min*60*1000;
    let sec=msec/1000;
    msec = msec %1000;
    return datetime_t{
        systime:dt.clone(),
        year:(years+1970) as u32,
        mon:mon as u8,
        day:day_in_mon as u8,
        hour:hour as u8,
        min:min as u8,
        sec:sec as u8,
        msec:msec as u16,
    }
    

}

pub fn format_datetime(dt:&std_time::SystemTime) ->String {
    let t = get_datetime_from_std(dt);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.year,t.mon,t.day,t.hour,t.min,t.sec,t.msec)
}

pub fn format_datetime2(dt:&std_time::SystemTime) ->String {
    let tm = time::OffsetDateTime::from(*dt);
    format!("{}",tm)
}

pub fn format_datetime_golang(dt:&std_time::SystemTime) ->String {
    let t = get_datetime_from_std(dt);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        t.year,t.mon,t.day,t.hour,t.min,t.sec)
}
