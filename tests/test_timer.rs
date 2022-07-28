#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use rust_rsm::rsm::{os_timer};
use rust_rsm::common;
use std::time::Duration;
use std::thread;

fn test_timer_call_back(timer_id:i32,timer_data:usize) {
    println!("[test timer]:timer_id={},timer_data={},current={}",timer_id,timer_data,
    common::format_datetime(&std::time::SystemTime::now()));
}
#[test]
fn test_os_timer() {
    os_timer::init_os_timer();
    let tm = match os_timer::os_timer_t::new(1000,1000,test_timer_call_back) {
        None=> {
            println!("set time failed");
            return
        },
        Some(t)=>t,
    };
    thread::sleep(Duration::from_millis(10*1000));
}