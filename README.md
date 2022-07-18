RSM: Rust Realtime System Middleware
=====
Introduction
===
Realtime system is defined as a system that can response the external request in certain deterministic time. To achieve this goal in generic computer systems, we must adopt a realtime shcedule policy on the software system, and keep from some time-consuming operation such as synchronous I/O operation, memory garbage collection and lock.

RSM is a lightweight realtime middleware implementation written in rust, support event-driven, message oriented lock-free programming principle. in RSM, every software module is a **component**, and each component can be instantiated to several tasks, and each task mapped to a dedicated **OS thread** and has its own message queue.

Developer can set the task's schedule priority and their message queue length respectively,usually based on the service model and performance & latency requirements.

RSM is suitable for the following applications:
----
- network device control plane, e.g. routing protocol, service control
- embedded system application
- remote control system
- realtime telemetry and instrumentation

Programming
===

Concept
---

each RSM component must implement the **rsm::Runnable** trait and provides a task creation Callback function.

the code in *main.rs* is a sample RSM application implementation.

*pub trait Runnable {*

    fn on_init(&mut self,cid:&rsm_component_t);

    fn on_timer(&mut self,cid:&rsm_component_t,timer_id:rsm_timer_id_t,timer_data:usize);

    fn on_message(&mut self,cid:&rsm_component_t,msg_id:rsm_message_id_t,msg:&rsm_message_t);

    fn on_close(&mut self,cid:&rsm_component_t);

}

*type rsm_new_task=fn(cid:&rsm_component_t)->&'static mut dyn Runnable*


Initialize the RSM
---
using *rsm_init* function to init the rsm system, then the applicaition can register their components to RSM.

rsm_init_cfg_t is the RSM's configuration file, which is in json format.
rsm_init(conf:&config::rsm_init_cfg_t)->errcode::RESULT

*pub fn registry_component(cid:u32,attrs:&component_attrs_t,callback:rsm_new_task)->errcode::RESULT*

After the component registration is finished, the *start_rsm()* function should be called to running the system.

Runtime
---
every running task can be identified uniquely by **rsm_component_t**

task can send message to each other, with normal message or a high priority message
*pub fn send_asyn_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESUL*

*pub fn send_asyn_priority_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT*

for the receiver side, the application use msg.decode::<T>(v) to restore the message to application defined type

RSM also provides a timer service, application can set timer simply by calling **set_timer** function, once the timer is set and expired, rsm task will receive a on_timer event, which is defined in the Runnable trait.

*pub fn set_timer(dur_msec:u64,loop_count:u64,timer_data:usize)->Option<rsm_timer_id_t>*
*pub fn kill_timer_by_id(timer_id:rsm_timer_id_t)->errcode::RESULT*

Diagnostic
===
Developer and user can use rest api get running status and statistics

Built in api
---
help,*curl http://127.0.0.1:12000/rsm/help*
get task running status, *curl http://127.0.0.1:12000/rsm/task?1:2*
get component configuration,*curl http://127.0.0.1:12000/rsm/component?1*

Application defined OAM API
---
application Module must implement *OamReqCallBack* function, and invoke *RegisterOamModule* to register self
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

if you have any suggestion, please send email to me: <wang_russell@hotmail.com>