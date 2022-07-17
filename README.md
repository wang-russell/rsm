RSM: Rust Realtime System Middleware
=====
Introduction
===
    Realtime system is defined as a system that can response the external request in certain deterministic time. To acheive this goal in generic computer systems, we must adopt a realtime shcedule policy on the software system, and keep from some time-consuming operation such as synchronous I/O operation, memory garbage collection and lock.

    RSM is a lightweight realtime middleware implementation written in rust, support event-driven, message oriented lock-free programming style. in RSM, every software module is a **component**, and each component can be instantiated to several tasks, and each task mapped to a dedicated **OS thread** and has its own message queue.
    developer can set task's schedule priority and their message queue len respectively,usually based on the service model and performance & latency requirement.

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

each RSM component must implement the **rsm::Runnable** trait and provide a task create Callback function.

the code in *main.rs* is a sample RSM application implementation.

*pub trait Runnable {*

    fn on_init(&mut self,cid:&rsm_component_t);

    fn on_timer(&mut self,cid:&rsm_component_t,timer_id:rsm_timer_id_t,timer_data:usize);

    fn on_message(&mut self,cid:&rsm_component_t,msg_id:rsm_message_id_t,msg:&rsm_message_t);

    fn on_close(&mut self,cid:&rsm_component_t);

}

*type rsm_new_task=fn(cid:&rsm_component_t)->&'static mut dyn Runnable*


initialize the RSM
---
using rsm_init to init the rsm system, and applcaition can register their component to RSM
rsm_init_cfg_t is the RSM's configuration file, which is json format.
rsm_init(conf:&config::rsm_init_cfg_t)->errcode::RESULT

*pub fn registry_component(cid:u32,attrs:&component_attrs_t,callback:rsm_new_task)->errcode::RESULT*

after the component registration is finished, the start_rsm() should be called to running the system.

runtime
---
every running task can be identified uniquely by **rsm_component_t**

task can send message each other, normal message or a high priority message
*pub fn send_asyn_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESUL*
*pub fn send_asyn_priority_msg(dst:&rsm_component_t,msg:rsm_message_t)->errcode::RESULT*

RSM also provide a timer service, application can set timer simply by call **set_timer** function, once the timer is set and expired, rsm task will receive a on_message event, which is defined in the Runnable trait.

*pub fn set_timer(dur_msec:u64,loop_count:u64,timer_data:usize)->Option<rsm_timer_id_t>*
*pub fn kill_timer_by_id(timer_id:rsm_timer_id_t)->errcode::RESULT*

Other service& lib function
===
xlog service
---
xlog service is based on client/server architecture, keeping from write disk under the application's context, which is very important for the realtime application.

    *let log = rsm::new_xlog(module_name:&str)->xlog::xlogger_t;*
    log.Errorf(postion, err, logDesc);

thread safe algorithm and data structure
---
+ spin_lock_t, Atomic operation based lock.
+ AtomicQueue, based on spin_lock
+ TsIdAllocator, thread safe Id allocator
+ bitmap
+ several network function and object wrapper