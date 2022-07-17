#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use rsm::common::{indexring,errcode};
use std::thread;
use std::time::Duration;
use std::sync::Arc;

type RingType = indexring::index_ring_t<Arc<u64>>;
#[test]
fn test_ring_op() {
    //test_option_ptr();
    let mut ring = RingType::new(100, 1);
    

    for i in 1..1024 {
        let a = Arc::new(i as u64);
        ring.add_item(i,a);
        ring.remove_item(i);      
    }
    assert_eq!(ring.get_ring_len(),0);
    ring.add_item(1,Arc::new(5));
    ring.add_item(2,Arc::new(5));
    ring.remove_item(1);
    assert_eq!(ring.get_ring_len(),1);
    show_ring_info(&ring);
    ring.clear();
    for i in 1..1025 {
        if i % 2==0 {
            let a = Arc::new(i as u64);
            ring.add_item(i,a);
        }
        
    }
    ring.remove_item_before_seq(974,true);
    show_ring_info(&ring);
    assert_eq!(ring.get_ring_capacity(), 100);
    assert_eq!(ring.get_ring_len(), 50);

    thread::sleep(Duration::from_secs(1));
    for i in 984..994 {
        ring.remove_item(i);
    }
    show_ring_info(&ring);
    assert_eq!(ring.get_ring_len(), 50);

    for i in 974..985 {
        ring.remove_item(i);
    }
    show_ring_info(&ring);
    assert_eq!(ring.get_ring_len(), 40);
    println!("Seq=1000,index={}",ring.get_index_by_seq(1000));
    ring.remove_item_before_seq(1023,false);
    show_ring_info(&ring);

    ring.remove_item_before_seq(1023,true);    
    show_ring_info(&ring);
    let a = Arc::new(1);
    ring.add_item(1025,a);
    show_ring_info(&ring);
    test_ring_rollover();
}

fn show_ring_info(r: &RingType) {
    println!(
        "capacity={},len={},head_idx={},tail_idx={},head_seq={},tail_seq={}",
        r.get_ring_capacity(),
        r.get_ring_len(),
        r.get_head_index(),
        r.get_tail_index(),
        r.get_head_seq(),
        r.get_tail_seq()
    );
}

fn test_ring_rollover() {
    let mut ring = indexring::index_ring_t::<u64>::new(1024,1);
    for i in 1..2048 {
        let r = ring.add_item(i,i);
        assert_eq!(r,errcode::RESULT_SUCCESS);        
    }

    assert_eq!(ring.get_ring_len(),1024);
    let r = ring.add_item(1048,1048);
    assert_eq!(r,errcode::ERROR_ALREADY_EXIST);
    ring.remove_item(1024);
    assert_eq!(ring.get_ring_len(),1023);
    println!("ring info {}",ring.to_string());
    let seq = ring.get_head_seq();
    let len = ring.get_ring_len() as u64;
    let item = ring.get_head_item().unwrap();
  
    assert_eq!(*item,seq);
    
    //show_ring_info(&ring);
    /*
    for i in seq..seq+len {
        match ring.get_item_by_seq(i) {
            None=> {
                println!("No. {} item is None",i);
            },
            Some(d)=> {
                println!("No. {} item={}",i,d);
            },
        }
    }*/
}

fn test_option_ptr () {
    let mut a:Option<Box<i32>>=None;
    println!("Init Option a");
    {
        let b=Box::new(5);
        a=Some(b);
    }
    match a {
        None=>println!("a is none"),
        Some(s)=>println!("{}",s),
    }
    //thread::sleep(Duration::from_secs(1));
    println!("release Option a");
    a=None;
    match a {
        None=>println!("a is none"),
        Some(s)=>println!("{}",s),
    }
}

