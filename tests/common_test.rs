#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
use std::io::Write;
use std::time::Duration;
use std::{self, net, thread};
use rust_rsm::common::{self, errcode,rawstring,tsmap::TsHashMap,tsqueue::TsDequeue};
use std::collections::{HashMap,VecDeque};
use rust_rsm::alg::hash_alg;
use rust_rsm::net_ext::{self,restserver};

static mut count: i32 = 0;
fn call_back(
    method: &restserver::Method,
    url: &str,
    body: &String,
) -> Result<(String,restserver::E_CONTENT_TYPE), errcode::RESULT> {
    println!("url={},body={}", url, body);
    //Ok(String::from("123456"))

    unsafe {
        count += 1;
        match count % 10 {
            0 => Ok(("Hello World".to_string(),restserver::E_CONTENT_TYPE::e_content_text)),
            1 => Err(errcode::ERROR_NOT_FOUND),
            2 => Err(errcode::ERROR_INVALID_INDEX),
            3 => Err(errcode::ERROR_DECODE_MSG),
            _ => Err(errcode::ERROR_COMMON),
        }
    }
}

#[test]
fn test_get_path() {
    let url = "https://www.example.com/aaa/ccc";
    let res = restserver::get_url_path(url);
    let path = match res {
        Ok(s) => s,
        Err(r) => panic!("Error get path from url {}", url),
    };
    assert_eq!(path, "/aaa/ccc");
    println!("return path is {}", path);
}

#[test]
fn test_rest() {
    let url1 = "GET 10.1.2.3/rest/woc/wot/1";
    let path1 = restserver::get_url_path(&url1).unwrap();
    assert_eq!(path1,"/rest/woc/wot/1");

    let url2 = "GET https://10.1.2.3/rest/woc/wot/1";
    let path2 = restserver::get_url_path(&url2).unwrap();
    assert_eq!(path2,"/rest/woc/wot/1");
    println!("{},{}",path1,path2);

    let ip = net::IpAddr::from([127, 0, 0, 1]);
    let s = restserver::RestServer::new(ip, restserver::DEF_SERVER_PORT, call_back);
    println!("begin test Rest\n");

    let s = match s {
        Ok(server) => server,
        Err(code) => {
            return;
        }
    };
    s.run();

    let rstr =
        "GET /api/assets HTTP/1.1\r\n User-Agent: Test \r\ncontent-length: 8 \r\n\r\n Hello\r\n"
            .as_bytes();
    let mut sck =
        net::TcpStream::connect(net::SocketAddr::new(ip, restserver::DEF_SERVER_PORT)).unwrap();
    let _ = sck.set_write_timeout(Some(Duration::from_millis(1000)));

    for i in 0..11 {
        println!("Send {} Packet", i);
        let len = sck.write(rstr).unwrap();

        assert_eq!(len, rstr.len());
    }

    thread::sleep(Duration::from_millis(1000));
}

#[test]
fn test_func() {
    let now = common::get_now_usec64();
    thread::sleep(Duration::from_millis(1));
    println!(
        "using Unix Epoch:Time Elapse {} us",
        common::get_now_usec64() - now
    );

    let now1 = std::time::Instant::now();
    thread::sleep(Duration::from_millis(1));
    println!(
        "using Instant:Time Elapse {} us",
        now1.elapsed().as_micros()
    );
    let buf = [0; 1024];
    let ptr_buf=&buf;
    println!("len of slice={}",ptr_buf.len());
}

#[test]
fn test_hash() {
    let user_name = "user-1";
    let passwd = "passwd1234";
    //let nonce = hash_alg::get_rand_128();
    let nonce:[u8;16] = [0xf5, 0x7e, 0x6f, 0xf1,0xec, 0xda ,0x04 ,0x17 ,0xcb, 0x53 ,0x8f ,0xb9 ,0xaa ,0x95 ,0x97 ,0xcd];
    let resp = hash_alg::hash_3value_128(user_name.as_bytes(),passwd.as_bytes(),&nonce[..]);

    println!("hash({},{},[{}])=[{}]",user_name,passwd,rawstring::slice_to_hex_string(&nonce[..]),
    rawstring::slice_to_hex_string(&resp[..]));

    for i in 0..10 {
        let rnd = hash_alg::get_rand_128();
        println!("{}th rand_num=[{}]",i,rawstring::slice_to_hex_string(&rnd[0..16]));

    }
}

#[test]
fn test_tsmap() {
    let mut map:TsHashMap<u64,u64> = TsHashMap::new(1024);

    for i in 1..1024 {
        map.insert(i,i);
    }
    println!("map len={},capacity={}",map.len(),map.capacity());
    map.remove(&1);
    println!("map len={},capacity={}",map.len(),map.capacity());
    
}

#[test]
fn test_tsqueue() {
    let mut queue:TsDequeue<u64> = TsDequeue::new(1024);

    for i in 1..1024 {
       queue.push_back(i)
    }
    println!("queue len={},capacity={}",queue.len(),queue.capacity());
    queue.pop_front();
    println!("queue len={},capacity={}",queue.len(),queue.capacity());
    
}

#[test]
fn test_time() {
    use time;
    
    let now=std::time::SystemTime::now();
    
    println!("time={},std_time={},golang_time={}",common::format_datetime2(&now),common::format_datetime(&now),
    common::format_datetime_golang(&now));
    let local = time::UtcOffset::current_local_offset().unwrap();
    println!("UTC Offset={}", local.whole_seconds());
}

#[test]
fn test_rawstring() {
    let mut astr:[u8;10]=[0x41,0x42,0x43,0x44,0x45,0x46,0,0,0,0];
    println!("raw string len={}",common::rawstring::raw_strlen(&mut astr[0] as *mut u8,10));

    let new_str=common::rawstring::array_to_string(&mut astr[0] as *mut u8,10);
    println!("{}",new_str);
} 

const TEST_CAPACITY:usize = 100000;
#[test]
fn test_map_performance() {
    use std::sync::Mutex;
    use rust_rsm::common::tsmap;
    let mut map1:Mutex<HashMap<usize,usize>> = Mutex::new(HashMap::with_capacity(TEST_CAPACITY));
    let mut map2:TsHashMap<usize,usize> = TsHashMap::new(TEST_CAPACITY);
    let mut map3:tsmap::TsHashMap<usize,usize> = tsmap::TsHashMap::new(TEST_CAPACITY);
    let mut cur = common::get_now_usec64();
    for i in 0..TEST_CAPACITY {
        map1.lock().unwrap().insert(i, i);
    }
    println!("insert {} item into HashMap,spend {} us\n",TEST_CAPACITY,common::get_now_usec64()-cur);


    cur = common::get_now_usec64();
    for (a,b) in map1.lock().unwrap().iter(){
        let (k,v)=(a,b);
    }

    println!("iterate {} item into HashMap,spend {} us\n",TEST_CAPACITY,common::get_now_usec64()-cur);

    cur = common::get_now_usec64();
    for i in 0..TEST_CAPACITY {
        map1.lock().unwrap().get(&i);
    }
    println!("find {} times from HashMap,spend {} us\n",TEST_CAPACITY,common::get_now_usec64()-cur);



     cur = common::get_now_usec64();
    for i in 0..TEST_CAPACITY {
        map2.insert(i, i);
    }
    println!("insert {} item into TsHashMap,spend {} us\n",TEST_CAPACITY,common::get_now_usec64()-cur);

    cur = common::get_now_usec64();
    for i in 0..TEST_CAPACITY {
        map2.get(&i);
    }
    println!("find {} times from TsHashMap,spend {} us\n",TEST_CAPACITY,common::get_now_usec64()-cur);


    cur = common::get_now_usec64();
    for i in 0..TEST_CAPACITY {
        map3.insert(i, i);
    }
    println!("insert {} item into old TsHashMap,spend {} us\n",TEST_CAPACITY,common::get_now_usec64()-cur);

    cur = common::get_now_usec64();
    for i in 0..TEST_CAPACITY {
        map3.get(&i);
    }
    println!("find {} times from old TsHashMap,spend {} us\n",TEST_CAPACITY,common::get_now_usec64()-cur);

    cur = common::get_now_usec64();
    for (a,b) in map3.iter(){
        let (k,v)=(a,b);
    }
    println!("iterate {} item into old TsHashMap,spend {} us\n",TEST_CAPACITY,common::get_now_usec64()-cur);

}


const TEST_QUEUE_CAPACITY:usize = 100000;
#[test]
fn test_queue_performance() {
    use std::sync::Mutex;
    use crate::common::atomicqueue::AtomicDequeue;
    let mut q:TsDequeue<usize>=TsDequeue::new(TEST_QUEUE_CAPACITY);
    let mut aq:AtomicDequeue<usize>=AtomicDequeue::new(TEST_QUEUE_CAPACITY);
    let mut cur = common::get_now_usec64();
    for i in 0..TEST_QUEUE_CAPACITY {
        q.push_back(i);
    }
    println!("insert {} item into TsQueue,spend {} us\n",q.len(),common::get_now_usec64()-cur);

    cur = common::get_now_usec64();
    for _ in 0..TEST_QUEUE_CAPACITY {
        let a=q.pop_front();
    }
    println!("Pop {} item from tsqueue,spend {} us, current_len={}\n",TEST_QUEUE_CAPACITY,common::get_now_usec64()-cur,q.len());

    cur = common::get_now_usec64();
    for i in 0..TEST_QUEUE_CAPACITY {
        aq.push_back(i);
    }
    println!("insert {} item into Atomic Queue,spend {} us,capacity={}\n",aq.len(),common::get_now_usec64()-cur,aq.capacity());

    cur = common::get_now_usec64();
    for _ in 0..TEST_QUEUE_CAPACITY {
        let aq = aq.pop_front();
    }
    println!("pop {} item from Atomic Queue,spend {} us,cur_len={}\n",TEST_QUEUE_CAPACITY,common::get_now_usec64()-cur,aq.len());

    let mut vq:VecDeque<usize> = VecDeque::with_capacity(TEST_QUEUE_CAPACITY);
     cur = common::get_now_usec64();
    for i in 0..TEST_QUEUE_CAPACITY {
        vq.push_back(i);
    }
   
    println!("insert {} item into oringin Queue,spend {} us\n",vq.len(),common::get_now_usec64()-cur);
    vq.clear();
    
    let mut vq1:VecDeque<usize> = VecDeque::with_capacity(TEST_QUEUE_CAPACITY);
    let lq = Mutex::new(vq1);

    cur = common::get_now_usec64();
   for i in 0..TEST_QUEUE_CAPACITY {
       lq.lock().unwrap().push_back(i);
   }
  
   println!("insert {} item into oringin locked Queue,spend {} us\n",lq.lock().unwrap().len(),common::get_now_usec64()-cur);

}

const USEC_PER_HOUR:u64 = 1000000*3600;
const USEC_PER_DAY:u64 = USEC_PER_HOUR*24;
#[test]
fn test_date_time() {
    println!("Unix Epoch = {}",common::format_datetime(&common::get_time_from_usec(0)));
    let mut cur = common::get_now_usec64();
    println!("Current time = {}",common::format_datetime(&common::get_time_from_usec(cur)));
    cur -= USEC_PER_DAY;
    println!("yestoday time = {}",common::format_datetime(&common::get_time_from_usec(cur)));
    cur -= USEC_PER_DAY*364;
    println!("one year ago time = {}",common::format_datetime(&common::get_time_from_usec(cur)));
}

#[test]
fn test_spin_lock() {
    use rust_rsm::common::spin_lock::spin_lock_t;

    let lock=spin_lock_t::new();

    lock.lock();
    println!("lock value={}",lock.value());

    lock.unlock();
    println!("unlock value={}",lock.value());

    lock.unlock();
    println!("unlock value={}",lock.value());

    lock.lock();
    println!("lock value={}",lock.value());

    lock.lock();
    println!("lock value={}",lock.value());



}

#[test]
fn test_os_sched() {
    use rust_rsm::common::sched;
    println!("Master thread self_pid={},cpu_nums={}",
        sched::get_self_threadId(),sched::get_sys_cpu_num());
        std::thread::spawn(an_th);
    std::thread::sleep_ms(1000);
}

fn an_th() {
    use rust_rsm::common::sched;
    println!("Spawn thread self_pid={},cpu_nums={}",
    sched::get_self_threadId(),sched::get_sys_cpu_num());
}

#[test]
fn test_bit_map() {
    use rust_rsm::common::bitmap::bitmap_t;

    let mut bitmap = bitmap_t::new(32768);

    for i in 0..32768 {
        if i % 2 == 0 {
            bitmap.set_bitmap(i);
        }
    }
    println!("bitmap status {}",bitmap.to_string());
    assert_eq!(bitmap.get_used_count(),32768/2);

    for i in 0..32768 {
        if i % 2 == 0 {
            if !bitmap.is_bit_set(i) {
                let (v,b)=bitmap.get_u64_by_index(i as usize);
                println!("error occured,i={},u64={:#0x},test_bits={:#0x},not_test:{:#0x}",i,*v,b,!b);
            }
            assert_eq!(bitmap.is_bit_set(i),true);
            bitmap.unset_bitmap(i);
        }
    }
    println!("bitmap status {}",bitmap.to_string());
    assert_eq!(bitmap.get_used_count(),0);

}

#[test]
fn test_tsidallocator() {
    use rust_rsm::common::tsidallocator::TsIdAllocator;
    let mut ids = TsIdAllocator::new(1,1024);

    for i in 1..1025 {
        let id=ids.allocate_id();
        println!("i={},allocated id={},",i,id);
        assert_eq!(id,i as i32);
    }

    for i in 1..1025 {
        ids.release_id(i);       
        assert_eq!(ids.used_count(),(1024-i) as i32);
    }

    for i in 1..1024 {
        let id1 = ids.allocate_id();
        println!("i={},allocated id={},",i,id1);
        ids.release_id(id1);
 
        assert_eq!(ids.used_count(),0);
    }
}