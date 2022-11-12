#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::cmp::{Ord,PartialOrd,Eq};
use super::*;

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct node_cost_t {
    pub node_idx:usize,
    pub priority:u32,
}

impl Ord for node_cost_t {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority).then_with(|| self.node_idx.cmp(&other.node_idx))
    }
}
impl PartialOrd for node_cost_t {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
pub struct priority_queue_t {
    start_idx:usize,
    data:Vec<node_cost_t>,
}

impl priority_queue_t {

    pub fn new()->Self {
        return Self {
            start_idx:0,
            data:Vec::with_capacity(SPF_MAX_NODE_NUM*2),
        }
    }

    pub fn push(&mut self,node_id:usize, prio:u32) {
        let node = node_cost_t {
            node_idx:node_id,
            priority:prio,
        };
        self.data.push(node);
        self.data.as_mut_slice()[self.start_idx..].sort();
    }

    //add a data item , but app must call sort later
    pub fn push_nosort(&mut self,node_id:usize, prio:u32) {
        let node = node_cost_t {
            node_idx:node_id,
            priority:prio,
        };
        self.data.push(node);
    }

    pub fn sort(&mut self) {
        self.data.as_mut_slice()[self.start_idx..].sort();
    }

    //pop a minimum value item
    pub fn pop_min(&mut self)->Option<node_cost_t> {
        if self.start_idx>=self.data.len() {
            return None
        }
        let v = self.data[self.start_idx].clone(); 
        self.start_idx+=1;
        
        return Some(v)
    }

    /*降低一个元素的优先级*/
    #[inline(always)]
    pub fn decrease_priority(&mut self,node_idx:usize, prio:u32) {
        let real_idx=node_idx+self.start_idx;
        if real_idx>=self.data.len() || self.data[real_idx].priority==prio{
            return
        }
        self.data[real_idx].priority=prio;
        self.data.as_mut_slice()[self.start_idx..].sort();
        
    }

    #[inline(always)]
    pub fn get_item_by_index(&self,idx:usize)->Option<node_cost_t> {
        let real_idx=idx+self.start_idx;
        if real_idx>= self.data.len() {
            return None;
        }

        return Some(unsafe { self.data.get_unchecked(real_idx).clone()})
    }

    #[inline(always)]
    pub fn len(&self)->usize {
        self.data.len()-self.start_idx
    }
}
