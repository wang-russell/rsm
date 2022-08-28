#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

//rsm scheduler, manager task control block, schedule task according to the message
use super::*;
use common::{errcode,atomicqueue::AtomicDequeue};
use common::sched;

#[derive(Default,Clone,Serialize)]
pub(crate) struct task_stats_t {
    recv_msg:u64,
    recv_prio_msg:u64,
    drop_msg:u64,
    drop_prio_msg:u64,
    timer_ev_count:u64,    
    cur_msg_id:u32,
    last_run_at:u64,    
    last_run_usec:u64,
    cur_run_usec:u64,
}

pub(crate) struct task_t{
    tid:rsm_component_t,
    os_tid:sched::os_task_id_t,
    recv_q:Option<AtomicDequeue<rsm_message_t>>,
    priority:E_RSM_TASK_PRIORITY,
    stats:task_stats_t,
    sender:Option<rsm_component_t>,
    terminated:bool,
    task_obj:&'static mut dyn Runnable,
}

impl task_t {
    pub(crate)  fn new(tid:&rsm_component_t,q_len:usize,prio:E_RSM_TASK_PRIORITY,task_obj:&'static mut dyn Runnable)->Self {
        return Self {
            tid:tid.clone(),
            os_tid:0,
            recv_q:Some(AtomicDequeue::new(q_len)),
            priority:prio,
            stats:task_stats_t::default(),
            sender:None,   
            terminated:false,
            task_obj:task_obj,
        }
    }

   ///get self component id, get None if not under the rsm thread context
    pub(crate) fn get_self_cid(&self)->Option<&rsm_component_t> {
        Some(&self.tid)
    }

    pub(crate)  fn send_asyn_msg(&mut self,msg:rsm_message_t)->errcode::RESULT {
        if let Some(q)=&mut self.recv_q {
            self.sender = Some(msg.sender.clone());
            let res = q.push_back(msg);
            if res==errcode::RESULT_SUCCESS {
                self.stats.recv_msg+=1;
                q.notify();                
            } else {
                self.stats.drop_msg+=1;
            }
            res
        } else {
            self.stats.drop_msg+=1;
            errcode::ERROR_NOT_INITIALIZED
        }        
    }

    ///send one high priority message to specific component
    pub(crate) fn send_asyn_priority_msg(&mut self,msg:rsm_message_t)->errcode::RESULT {
         if let Some(q)=&mut self.recv_q {
            self.sender = Some(msg.sender.clone());
            let res =  q.push_front(msg);
            if res==errcode::RESULT_SUCCESS {
                q.notify();
                self.stats.recv_prio_msg+=1;           
            } else {
                self.stats.drop_prio_msg+=1;
            }
            res
        } else {
            self.stats.drop_prio_msg+=1;
            errcode::ERROR_NOT_INITIALIZED
        }
    }

    pub(crate) fn get_task_priority(&self)->E_RSM_TASK_PRIORITY {
        self.priority
    }
    /// return current recv queue len
    pub fn get_qlen(&self)->usize {
        if let Some(q) = &self.recv_q {
            q.len()
        } else {
            0
        }
    }

    /// return current msg id proccessed
    pub(crate)  fn cur_msg_id(&self)->u32 {
        self.stats.cur_msg_id
    }

    pub(crate)  fn set_os_task_attr(&mut self) {
        self.os_tid = sched::get_self_os_task_id();
        let (policy,prio) = rsm_sched::map_os_priority(self.priority);
        sched::set_self_priority(policy,prio);
    }

    ///Running the Task
    pub(crate)  fn run(&mut self) {
        self.set_os_task_attr();

        let rq=match &mut self.recv_q {
            None=>return,
            Some(q)=>q,
        };
        self.task_obj.on_init(&self.tid);
        loop {
            rq.wait();
            loop {
                let msg = match rq.pop_front() {
                    None=>break,
                    Some(msg)=>msg,
                };
                self.stats.cur_msg_id = msg.msg_id;
                self.stats.last_run_at = common::get_now_usec64();
                match msg.msg_id {
                    RSM_MSG_ID_TIMER=> {
                        self.stats.timer_ev_count+=1;
                        self.task_obj.on_timer(&self.tid,msg.timer_id,msg.timer_data);
                    },
                    RSM_MSG_ID_SOCKET=> {
                        if let Some(ev) = msg.decode() {
                            self.task_obj.on_socket_event(&self.tid, ev);
                        }
                        
                    },                   
                    RSM_MSG_ID_MASTER_POWER_ON..=RSM_MSG_ID_SLAVE_POWER_ON=>self.task_obj.on_init(&self.tid),

                    
                    RSM_MSG_ID_POWER_OFF=>{
                        self.task_obj.on_close(&self.tid);
                        self.terminated = true;
                        break;
                    },
                    _=> {
                        self.task_obj.on_message(&self.tid,msg.msg_id,&msg);                        
                    },               
                }
                self.stats.cur_msg_id = RSM_INVALID_MESSAGE_ID;
                self.stats.last_run_usec = common::get_now_usec64()-self.stats.last_run_at;
            }
            if self.terminated {
                break;
            }
        }
        self.task_obj.on_close(&self.tid);
    }

    pub fn get_task_stats(&self)->task_stats_t {
        let mut stats = self.stats.clone();
        if stats.cur_msg_id!=RSM_INVALID_MESSAGE_ID {
            stats.cur_run_usec = common::get_now_usec64()-self.stats.last_run_at;
        } else {
            stats.cur_run_usec=0;
        }
        return stats
    }
    pub fn get_sender_cid(&self)->Option<rsm_component_t> {
        match &self.sender {
            None=>return None,
            Some(s)=>Some(s.clone()),
        }
    }
    pub fn clear_task_stats(&mut self) {
        self.stats = task_stats_t::default();
    }

    pub fn to_string(&self)->String {
        let qlen = match &self.recv_q {
            None=>0,
            Some(q)=>q.len()
        };
        format!("cid={},inst={},node_id={},cur_msg={},qlen={},recv_normal_msg={},recv_priority_msg={},drop_msg={},drop_prio_msg={}",
        self.tid.cid,self.tid.inst_id,self.tid.node_id,self.stats.cur_msg_id,qlen,
        self.stats.recv_msg,self.stats.recv_prio_msg,self.stats.drop_msg,self.stats.drop_prio_msg)
    }

}