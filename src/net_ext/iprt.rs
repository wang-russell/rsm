#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

///IP route table implementation
use super::*;
use std::net::IpAddr;
use crate::common::{errcode,tsmap::TsHashMap};
use super::ipnetwork::IpNetwork;

///Route Source, can be extend to any value
pub const ROUTE_SRC_STATIC:u8=0; //static route
pub const ROUTE_SRC_CONTROLLER:u8=1;//route entry from sdn controller
pub const ROUTE_SRC_OSPF:u8=2;
pub const ROUTE_SRC_BGP:u8=3;
pub const ROUTE_SRC_ISIS:u8=4;
pub const ROUTE_SRC_OTHER_IGP:u8=5;

/// 
#[derive(Clone,Hash,PartialEq,Eq)]
pub struct ip_route_key_t {
    pub vrf:i32,
    pub subnet:IpAddr,
    pub mask_len:u8,
}
impl ip_route_key_t {
    pub fn new(vrf:i32,prefix:&IpAddr,mask_len:u8)->Self {
        let subnet = ipnetwork::IpNetwork::get_ip_subnet(prefix, mask_len);
        return Self {
            vrf,
            subnet,
            mask_len,
        }
    }
}

///路由结果，T为用户定义的路由结果,priority越大表示优先级越高
#[derive(Clone,PartialEq)]
pub struct ip_route_result_t<T> {
    pub rt_key:ip_route_key_t,
    pub priority:u16,
    pub route_src:u16,
    pub user_result:T,
}

impl <T> ip_route_result_t<T> 
where T: Clone+Eq {
    pub fn new(key:&ip_route_key_t, priority:u16,route_src:u16,route_result:&T)->Self {
        return Self {
            rt_key:key.clone(),
            priority:priority,
            route_src:route_src,
            user_result:(*route_result).clone(),
        }
    }
}

///路由结果集，T为用户定义的路由结果,priority越大表示优先级越高
#[derive(Clone,PartialEq)]
pub struct ip_route_result_set_t<T> {
    pub result_set:Vec<ip_route_result_t<T>>,
    pub valid_index:Vec<usize>,
}

impl <T> ip_route_result_set_t<T> 
where T: Clone+Eq {
    pub fn new()->Self {
        return Self {
            result_set:Vec::new(),
            valid_index:Vec::new(),
        }
    }

    pub fn len(&self)->usize {
        self.result_set.len()
    }

    ///判断某个路由结果是否已经存在
    pub fn is_result_exist(&self,key:&ip_route_key_t, priority:u16,route_src:u16,route_result:&T)->bool {
        for item in &self.result_set {
            if item.rt_key.eq(key) && item.priority==priority && item.route_src==route_src && item.user_result.eq(route_result){
                return true
            }
        }
        false
    }
    ///添加一个route result
    pub fn add_route_result(&mut self,key:&ip_route_key_t, priority:u16,route_src:u16,route_result:&T)->errcode::RESULT {
        if self.is_result_exist(key,priority,route_src,route_result) {
            return errcode::ERROR_ALREADY_EXIST;
        }
        let result = ip_route_result_t::new(key,priority,route_src,route_result);
        self.result_set.push(result);
        self.set_max_route_priority_index();
        errcode::RESULT_SUCCESS
    }

    pub fn delete_route_result(&mut self,key:&ip_route_key_t, priority:u16,route_src:u16,route_result:&T)->errcode::RESULT {
        let mut idx:usize = usize::MAX;
        for i in 0..self.result_set.len() {
            let pitem = &self.result_set[i];
            if pitem.rt_key.eq(key) && pitem.priority==priority && pitem.route_src==route_src && pitem.user_result.eq(route_result) {
                idx=i;
                break;
            }
        }
        if idx< self.result_set.len() {
            self.result_set.remove(idx);
            self.set_max_route_priority_index();
            return  errcode::RESULT_SUCCESS
        }
        return errcode::ERROR_NOT_FOUND
    }

    ///获取Result Count
    pub fn get_result_count(&self)->usize {
        self.result_set.len()
    }
    ///根据priority 求最高优先级的route条目
    pub fn set_max_route_priority_index(&mut self) {
        self.valid_index.clear();
        if self.result_set.len()==0 {
            return;
        }
        let mut max_prio =0;
        for i in 0..self.result_set.len() {
            if self.result_set[i].priority >=max_prio {
                max_prio = self.result_set[i].priority;
            }
        }

        for i in 0..self.result_set.len() {
            if self.result_set[i].priority ==max_prio {
                self.valid_index.push(i);
            }
        }
    }
    
}


const MAX_ROUTE_PREFIX_LEN:usize = IPV6_ADDR_LEN*8;
const IPV4_ROUTE_PREFIX_LEN:usize = IPV4_ADDR_LEN*8;

pub struct ip_route_table_t<T> 
where T: Clone+Eq  {
    routes_count:[usize;MAX_ROUTE_PREFIX_LEN+1],
    routes:TsHashMap<ip_route_key_t,ip_route_result_set_t<T>>
}


impl <T> ip_route_table_t<T> 
    where T: Clone+Eq {
    pub fn new(capacity:usize)->Self {
        let rt_table = Self{
            routes_count:[0;MAX_ROUTE_PREFIX_LEN+1],
            routes:TsHashMap::new(capacity)
        };
        return rt_table
    }

    ///添加一条路由条目,priority是优先级，数值越大表示优先级越高
    pub fn add_ip_route(&mut self,vrf:i32,prefix:&IpAddr,mask_len:u8,priority:u16,route_src:u16,route_result:&T)->errcode::RESULT {
        if !IpNetwork::is_valid_ipmask(prefix,mask_len) {
            return errcode::ERROR_INVALID_PARAM
        }

        let rt_key = ip_route_key_t::new(vrf, prefix, mask_len);
        match self.routes.get_mut(&rt_key) {
            None=> {
                let mut rs_set = ip_route_result_set_t::new();
                rs_set.add_route_result(&rt_key,priority,route_src,route_result);
                let res = self.routes.insert(rt_key,rs_set);
                if res==errcode::RESULT_SUCCESS {
                    self.routes_count[mask_len as usize]+=1;
                }
                return res;
                
            },
            Some(r)=> {
                return r.add_route_result(&rt_key,priority,route_src,route_result);
            },
        }
    }

    pub fn delete_ip_route(&mut self,vrf:i32,prefix:&IpAddr,mask_len:u8,priority:u16,route_src:u16,user_result:&T)->errcode::RESULT {
        if !IpNetwork::is_valid_ipmask(prefix,mask_len) {
            return errcode::ERROR_INVALID_PARAM
        }
        let rt_key = ip_route_key_t::new(vrf, prefix, mask_len);
        let (res,rset) = match self.routes.get_mut(&rt_key) {
            None=>return errcode::ERROR_NOT_FOUND,
            Some(r)=> {
                let res = r.delete_route_result(&rt_key,priority,route_src,user_result);
                (res,r)                
            },
        };
        if res==errcode::RESULT_SUCCESS && rset.len()==0{
            self.routes_count[mask_len as usize]-=1;
            self.routes.remove(&rt_key);
        }
        return res
       
    }

    pub fn lookup_ip_route(&self,vrf:i32,dst_ip:&IpAddr)->Option<&ip_route_result_set_t<T>> {
        let prefix_len = if dst_ip.is_ipv6() {MAX_ROUTE_PREFIX_LEN} else {IPV4_ROUTE_PREFIX_LEN};

        for i in 0..prefix_len+1 {
            let mask_len = prefix_len-i;
            if self.routes_count[mask_len]==0 {
                continue
            }
            let rt_key = ip_route_key_t::new(vrf,dst_ip,mask_len as u8);
            if let Some(res) = self.routes.get(&rt_key) {
                return Some(&res)
            }
        } 
        None
    }

        pub fn lookup_ip_one_route(&self,vrf:i32,dst_ip:&IpAddr)->Option<ip_route_result_t<T>> {
            let prefix_len = if dst_ip.is_ipv6() {MAX_ROUTE_PREFIX_LEN} else {IPV4_ROUTE_PREFIX_LEN};
    
            for i in 0..prefix_len+1 {
                let mask_len = prefix_len-i;
                if self.routes_count[mask_len]==0 {
                    continue
                }
                let rt_key = ip_route_key_t::new(vrf,dst_ip,mask_len as u8);
                if let Some(res) = self.routes.get(&rt_key) {
                    if res.valid_index.len()>0 {
                        return Some(res.result_set[res.valid_index[0]].clone())
                    } else {
                        return None;
                    }
                   
                }
            } 
            None
        }
    
    pub fn len(&self)->usize {
        self.routes.len()
    }

    pub fn capacity(&self)->usize {
        self.routes.capacity()
    }

    pub fn clear(&mut self) {
        self.routes.clear();
        self.routes_count.fill(0);
    }

    pub fn print_stats(&self) {
        println!("Ip route table: capacity={},used={}",self.routes.capacity(),self.routes.len());
        let mut sub_total=0;
        for i in 0..MAX_ROUTE_PREFIX_LEN+1 {
            if self.routes_count[i]>0 {
                println!("prefix_len={},route entries={}",i,self.routes_count[i]);
                sub_total+=self.routes_count[i];
            }            
        }

        println!("Ip route table sub entries total={}",sub_total);
    }
}