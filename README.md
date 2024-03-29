RSM: Rust Realtime System Middleware
=====
Introduction
===
Realtime system is defined as a system that can response the external request in certain deterministic time. To achieve this goal in a generic computer systems, we must adopt a realtime shcedule policy on the software system, and keep from some time-consuming operation such as synchronous I/O operation, memory garbage collection and lock.

RSM is a lightweight realtime middleware implementation written in rust, support event-driven, message oriented lock-free programming principle. in RSM, every software module is a **component**, which is normally a Finite State Machine, mainly proccess event loop. Each component can be instantiated to several tasks, and each task mapped to a dedicated **OS thread** and has its own message queue.

Developer can set the task's schedule priority and their message queue length respectively,usually based on the service model and performance & latency requirements.

RSM is suitable for the following applications:
----
- network device control plane, e.g. routing protocol, service control
- embedded system applications
- remote control systems
- realtime telemetry and instrumentation

Programming
===

Concepts
---

each RSM component must implement the **rsm::Runnable** trait and provides a task creation Callback function.

the code in *main.rs* is a sample RSM application implementation.

*pub trait Runnable {*

    fn on_init(&mut self,cid:&rsm_component_t);

    fn on_timer(&mut self,cid:&rsm_component_t,timer_id:rsm_timer_id_t,timer_data:usize);

    fn on_socket_event(&mut self,cid:&rsm_component_t,event:rsm_socket_event_t);

    fn on_message(&mut self,cid:&rsm_component_t,msg_id:rsm_message_id_t,msg:&rsm_message_t);

    fn on_close(&mut self,cid:&rsm_component_t);

}

*type rsm_new_task=fn(cid:&rsm_component_t)->&'static mut dyn Runnable*


Initialize RSM
---
using *rsm_init* function to init the rsm system, then the applicaition can register their components to RSM.

rsm_init_cfg_t is the RSM's configuration file, which is in json format.
rsm_init(conf:&config::rsm_init_cfg_t)->errcode::RESULT

*pub fn registry_component(cid:u32,attrs:&component_attrs_t,callback:rsm_new_task)->errcode::RESULT*

After the component registration is finished, the *start_rsm()* function should be called to start the system.

Runtime
---
every running task can be identified uniquely by **rsm_component_t**

task can send message to each other, with normal message or a high priority message
*pub fn send_asyn_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESUL*

*pub fn send_asyn_priority_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT*

for the receiver side, the application must use msg.decode::<T>(v) to restore the message to application defined type

RSM also provides a timer service, application can set timer simply by calling **set_timer** function, once the timer is set and expired, rsm task will receive a on_timer event, which is defined in the Runnable trait.

*pub fn set_timer(dur_msec:u64,loop_count:u64,timer_data:usize)->Option<rsm_timer_id_t>*
*pub fn kill_timer_by_id(timer_id:rsm_timer_id_t)->errcode::RESULT*

Schedule priority
---
RSM support several predefined task priorities, they are mapped to the underlying OS thread schedule policies or priorities.

for the Linux OS, realtime priority map to the schedule_policy=SCHED_RR, non-realtime ones map to policy=SCHED_OTHER.

for Windows, RSM use THREAD_PRIORITY_TIME_CRITICAL const represents realtime priority.

pub enum E_RSM_TASK_PRIORITY {
    THREAD_PRI_LOW = 0,
	THREAD_PRI_NORMAL = 1,
	THREAD_PRI_HIGH = 2,    
	THREAD_PRI_REALTIME = 3,
	THREAD_PRI_REALTIME_HIGH = 4,
    THREAD_PRI_REALTIME_HIGHEST = 5,
}

Asynchronous Socket API
===
Since 0.3.0, a socket event method is added to RSM component Runnable Trait

fn on_socket_event(&mut self,cid:&rsm_component_t,event:rsm_socket_event_t);

and RSM provide several API object to implement asynchronous socket programming. The application can init a socket, and then process the socket event(new tcp socket or the socket is readable) sent by RSM

TCPListener
---
application can start a TCPListener, set the loadbalance policy.

if the tcplistener's initiator component has 4 tasks, RSM will dispatch the client connection to the 4 task by socket id using hash algorithm, and ensure one client connection is proccessed by only one task.

this is the default behavior, application can change this by set different LB_POLICY

pub enum SOCKET_LB_POLICY {
    ///dispatch tcp client connections to all the component instance by hash result
    SOCK_LB_ALL_INSTANCE=0,
    ///tcp connections only handled by the caller instance
    SOCK_LB_CALLER_INSTANCE=1,
    ///tcp connections dispatch to the component instances except the caller 
    SOCK_LB_EXCLUDE_CALLER_INSTANCE=2,
}

TCPSocket
---
A Tcp client socket API object, does not hold any socket runtime state

for the RSM message interface, a socket_id is used by default, application can conver this id to a Socket API object, e.g.:

 let mut sock=socket::TcpSocket::get_socket_by_id(event.socket_id);

UDPSocket
---
A UDP Socket, connectionless Socket API wrapper, the remainning part is similar to the TCPSocket.

You can use following method to get a socket instance from a socket_id.

let mut sock=socket::UdpSocket::get_socket_by_id(event.socket_id);

Diagnostic
===
Developer and user can use rest api to get running status and statistics of RSM.

Built in api
---
help,*curl http://127.0.0.1:12000/rsm/help*
get task running status, *curl http://127.0.0.1:12000/rsm/task?1:2*
get component configuration,*curl http://127.0.0.1:12000/rsm/component?1*

Application defined OAM API
---
application Module must implement *OamReqCallBack* function, and invoke *RegisterOamModule* to register itself
*OamReqCallBack=fn(op:E_RSM_OAM_OP,url:&String,param:&String)->oam_cmd_resp_t*

///register a module callback, urls is a list of rest api url, the prefix /rsm and id following a "?" are not included
*RegisterOamModule(urls:&[String], callback:OamReqCallBack)*

Other service& lib function
===
xlog service
---
xlog service is based on client/server architecture, the client side simple send log message to the server which responsible for log file manipulation, keeping from write disk under the application's context, which is very important for the realtime application.

*let log = rsm::new_xlog(module_name:&str)->xlog::xlogger_t;*

*log.Errorf(postion, err, logDesc);*

Other thread safe algorithm and data structure
---
+ spin_lock_t, Atomic operation based lock.
+ AtomicQueue, based on spin_lock
+ TsIdAllocator, thread safe Id allocator
+ bitmap
+ ethernet packet parser
+ Ip routing table
+ several other network function and object wrapper

if you have any question, please send email to: <wang_russell@hotmail.com>