#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use rust_rsm::rsm::{os_timer};
use rust_rsm::common::{self,errcode};
use std::net::SocketAddr;
use std::time::Duration;
use std::thread;
use rust_rsm::rsm::{self,socket,rsm_timer,xlog};

const TEST_APP_ID:rsm::rsm_component_id_t = rsm::RSM_USER_CID_START+1;
struct test_app_t {
    _count:u64,
    log:Option<xlog::xlogger_t>,
    tcpListener:Option<socket::TcpListener>,
    udpsock:Option<socket::UdpSocket>,
}
static mut testApp:[test_app_t;2]=[test_app_t{_count:0,log:None,tcpListener:None,udpsock:None},
    test_app_t{_count:1,log:None,tcpListener:None,udpsock:None}];

fn init_rsm() {
    let log_addr=SocketAddr::new("127.0.0.1".parse().unwrap(),15000);
    let oam_addr=SocketAddr::new("127.0.0.1".parse().unwrap(),12000);

    let cfg = rsm::config::rsm_init_cfg_t::new(1, Some(log_addr), 
        Some(oam_addr), None);
    rsm::rsm_init(&cfg);
}

fn new_test_app(cid:&rsm::rsm_component_t)->&'static mut dyn rsm::Runnable {
    let apps = unsafe {&mut testApp};
    let idx = cid.get_inst_id();
    return &mut apps[idx-1];
}

#[test]
fn test_rsm_sched() {
   init_rsm();
   let attrs = rsm::component_attrs_t {
    cid:TEST_APP_ID,    
    name:"test".to_string(),
    inst_num:2, //实例数量
    qlen:100,
    priority:rsm::E_RSM_TASK_PRIORITY::THREAD_PRI_REALTIME_HIGHEST,
    need_init_ack:false,
};

   rsm::registry_component(TEST_APP_ID, &attrs, new_test_app);
   rsm::start_rsm();

   loop {
    thread::sleep(Duration::from_millis(1000));
   }
}

impl rsm::Runnable for test_app_t {
    fn on_init(&mut self,cid:&rsm::rsm_component_t) {
        println!("recv Init msg,self_cid={:?}\n",cid);
        rsm_timer::set_timer(5000, 0, 10);
        self.log=Some(rsm::new_xlog("test"));
        let port = 14000+cid.get_inst_id() as u16;
        let addr=SocketAddr::new("0.0.0.0".parse().unwrap(),port);
        let lis = match socket::TcpListener::new(&addr, 128, socket::SOCKET_LB_POLICY::SOCK_LB_ALL_INSTANCE) {
            Ok(l)=>l,
            Err(e)=> {
                println!("create tcp listener failed, err={},addr={}",e,addr);
                return;
            }
        };

        println!("create tcp listener success,socket_id={}",lis.get_sock_id());
        self.tcpListener=Some(lis);

        if cid.get_inst_id()==1 {
            let us=socket::UdpSocket::new(&addr).unwrap();
            self.udpsock=Some(us);
        }
    }

    fn on_timer(&mut self,cid:&rsm::rsm_component_t,timer_id:rsm::rsm_timer_id_t,timer_data:usize) {
        println!("Recv Timer Event,timer_id={},data={},time={}\n",
            timer_id,timer_data,common::format_datetime(&std::time::SystemTime::now()));
        static mut data:u64=2048;
        /*
        let msg= rsm::rsm_message_t::new::<u64>(20000,unsafe {&data}).unwrap();
        
        if cid.get_inst_id()==1 {
            let dst=rsm::rsm_component_t::new(cid.get_cid(),1,2);
            let ret = rsm::send_asyn_msg(&dst, msg);
            if ret!=errcode::RESULT_SUCCESS {
                println!("Send message failed,ret={}",ret);
            }
        }*/
        unsafe { data+=2 };
    }

    fn on_socket_event(&mut self,cid:&rsm::rsm_component_t,event:rsm::rsm_socket_event_t) {
        println!("[test app]recv socket event,cid={},event=0x{:x},socketid={}",cid,event.event as u32,event.socket_id);
        if event.event & rsm::SOCK_EVENT_READ==0 {
            return
        }
        if event.event & rsm::SOCK_EVENT_ERR!=0 {
            return
        }
        match event.sock_type {

        socket::SOCKET_TYPE::PROTO_STREAM=>{
        let mut sock=socket::TcpSocket::get_socket_by_id(event.socket_id);

        let mut buf=[0u8;2048];
        let res = match sock.recv(&mut buf[..]) {
            Ok(l)=> String::from_utf8_lossy(&buf[0..l]),
            Err(e)=> {
                println!("[testapp]recv socket msg err,ret={},os_err={},sock_id={},fd={}",
                e,std::io::Error::last_os_error(),sock.get_socket_id(),sock.get_os_socket());
                return
            },
        };
        println!("[test app]recv socket msg:{}",res);
        sock.send("HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length:13\r\n\r\nHello world\r\n".as_bytes());
        sock.close();
        },
        socket::SOCKET_TYPE::PROTO_DGRAM=>{
            let mut s=socket::UdpSocket::get_socket_by_id(event.socket_id);
            let mut buf=[0u8;2048];
            let (len,dst) = match s.recv_from(&mut buf[..]) {
                Ok((l,d))=> (l,d),
                Err(e)=> {
                    println!("recv socket msg err,ret={}",e);
                    return
                },
            };
            let c=String::from_utf8_lossy(&buf[0..len]);
            println!("recv udp message from {},content={}",dst,c);           
        },
        _=>(),

    }

        
       
    }

    fn on_message(&mut self,cid:&rsm::rsm_component_t,msg_id:rsm::rsm_message_id_t,msg:&rsm::rsm_message_t) {

        let self_cid=rsm::get_self_cid();
        let sender= rsm::get_sender_cid();

        println!("recv msg,msg_id={},content={:?},sender={:?},self={:?}\n",msg_id,msg,sender,self_cid);
        if let Some(log) = &mut self.log {
            log.Errorf("test_app", 0, &format!("self_id={},recv message id={},v={:?}",cid,msg_id,msg));
        }
        
    }

    fn on_close(&mut self,cid:&rsm::rsm_component_t) {
        if let Some(log) = &mut self.log {
            log.Errorf("test_app", 0, &format!("self_id={},recv close call back",cid));
        }
    }
}
