/* spf.rs
日期：2022-9-3

最短路径算法，给定一个开销矩阵，计算任意两点之间最大的路径长度
目前最大支持1024个节点，入参的每个节点用整形的ID确定

已经支持：
1、增删节点
2、增删链路
3、更新链路Metric值
4、基于Priority Queue优化的Dijkstra算法
5、按源节点计算最短路径
6、计算所有的最短路径
7、一对节点间多链路，以支持设备多接口的场景；包括MANET网络结构下的路由

尚未支持的：
1、ECMP
2、PRC路由计算
3、路径变化通知
*/
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::common::errcode;
use crate::common::TsIdAllocator;
//use crate::common::TsHashMap;
use std::collections::HashMap;
use std::mem;
use std::alloc;
use super::*;

type T_NODE_INDEX=u16;


const INVALID_NODE_ID:u16 = u16::MAX;
pub const METRIC_INFINITY:u16 = u16::MAX;
const SPF_MAX_LINK_PER_PAIR:u16 = 4; //每对Node之间最大的链路数量

/*一对节点间允许多个链路的情况下，每个链路都有自己的Metric*/
#[derive(Clone)]
pub struct port_pair_t {
	src_port_id:u32, 
    dst_port_id:u32,
	metric:u32,
}

///path edge,represent one edge of the path
#[derive(Clone)]
pub struct path_edge_t {
    dst_node_id:u32,
    src_port_id:u32,
    dst_port_id:u32,
	metric:u32,
}

///path_node,represent one none in path, include multiple edges
#[derive(Clone)]
pub struct path_node_t {
	src_node_id:u32,
    edges:Vec<path_edge_t>,
}

/*一条边的信息,dst_node_id和src_node_id都按ID编号*/

#[derive(Clone)]
pub struct edge_t {
    src_node_id:T_NODE_INDEX,
    dst_node_id:T_NODE_INDEX,
    link_count:u16,
	metric:u32,
	best_link:u16,
	links:[port_pair_t;SPF_MAX_LINK_PER_PAIR as usize],
}

#[derive(Clone)]
pub struct edge_info_t {
	edge:edge_t,
	changed:bool,
}

type path_t=Vec<path_node_t>;

/*用于计算的开销矩阵*/
pub struct spf_matrix_t {
	node_count:usize,
	max_node_idx:i32,
	idx_alloc:TsIdAllocator,
	idx2id:HashMap<i32,u32>, //节点的索引序号和ID号的对应关系
	id2idx:HashMap<u32,i32>, //节点的ID号到序号的映射关系
	cost_matrix:* mut [[edge_info_t;SPF_MAX_NODE_NUM];SPF_MAX_NODE_NUM],
    //path vector, store the shortest distance predecessor;
	path_vector: * mut [[Vec<T_NODE_INDEX>;SPF_MAX_NODE_NUM];SPF_MAX_NODE_NUM],
}

/*创建一个开销矩阵，并进行初始化;Node Index从0开始编号*/
impl spf_matrix_t {

pub fn new()->Self {

    let cm = unsafe {
        alloc::alloc(alloc::Layout::from_size_align_unchecked(SPF_MAX_NODE_NUM*SPF_MAX_NODE_NUM*mem::size_of::<edge_info_t>(), 1)) 
		as * mut [[edge_info_t;SPF_MAX_NODE_NUM];SPF_MAX_NODE_NUM]    
    };
	let pv = unsafe {
		alloc::alloc(alloc::Layout::from_size_align_unchecked(SPF_MAX_NODE_NUM*SPF_MAX_NODE_NUM*mem::size_of::<path_t>(), 1)) 
		as * mut [[Vec<T_NODE_INDEX>;SPF_MAX_NODE_NUM];SPF_MAX_NODE_NUM]
	};

	let pcm=unsafe {&mut *cm};
    let ppv=unsafe {&mut *pv};

	for i in 0..SPF_MAX_NODE_NUM {
		for j in 0..SPF_MAX_NODE_NUM {
			pcm[i][j].changed = false;
			pcm[i][j].edge.src_node_id = i as u16;
			pcm[i][j].edge.dst_node_id = j as u16;
			if i == j {
				pcm[i][j].edge.metric = 0;
			} else {
				pcm[i][j].edge.metric = METRIC_INFINITY as u32;
			}
            let path=Vec::new();
            ppv[i][j]=path;

		}
	}

	return spf_matrix_t {
		node_count:0,
		idx_alloc:TsIdAllocator::new(0, SPF_MAX_NODE_NUM as i32),
		idx2id:HashMap::with_capacity(SPF_MAX_NODE_NUM),
		id2idx:HashMap::with_capacity(SPF_MAX_NODE_NUM),
		cost_matrix:cm,
		path_vector:pv,
		max_node_idx:-1,
	}
}

/*加入一个Node，用NodeId表示;内部用idx进行管理计算*/
pub fn add_node(&mut self,node_id:u32)->errcode::RESULT {

	if self.node_count >= SPF_MAX_NODE_NUM {
		return errcode::ERROR_OUTOF_MEM;
	}

	if self.id2idx.contains_key(&node_id) {
		return errcode::ERROR_ALREADY_EXIST
	}

	/*分配一个索引值*/
	let idx = self.idx_alloc.allocate_id();
	//fmt.Printf("Allocate NodeIdx,Id=%d,Index=%d\n", nodeId, idx)
	self.id2idx.insert(node_id.clone(), idx.clone());
	self.idx2id.insert(idx,node_id);
	self.node_count+=1;
	if idx>self.max_node_idx {
		self.max_node_idx = idx;
	}
	println!("add node success,node_id={},idx={}",node_id,idx);
	return errcode::RESULT_SUCCESS
}

/*删除一个节点*/
pub fn  delete_node(&mut self,node_id:u32)->errcode::RESULT {
	let idx = match self.id2idx.get(&node_id) {
        None=> {
            return errcode::ERROR_NOT_FOUND
        },
        Some(v)=>v.clone(),
    };
    self.idx_alloc.release_id(idx);
    self.node_count-=1;
	if idx==self.max_node_idx {
		self.max_node_idx-=1;
	}
	return errcode::RESULT_SUCCESS
}

/*根据NodeID号读取索引号*/
fn  getIndexById(&self,node_id:u32)->Option<i32> {
	match self.id2idx.get(&node_id) {
        None=> {
            return None
        },
        Some(v)=>Some(v.clone()),
    }
}

/*根据NodeID号读取索引号，读取不到返回INVALID_NODE_ID*/
fn  get_nodeid_by_idx(&self,idx:u16)->Option<u32> {
    let new_idx=idx as i32;
	match self.idx2id.get(&new_idx) {
        None=> {
            return None
        },
        Some(v)=>Some(v.clone()),
    }
}

/*从edge_t结构体中查找一个端口对是否存在，返回下标索引;否则返回INVALID_INDEX*/
fn find_link_from_node(&self,edge:&edge_t, src_port_id:u32, dst_port:u32)->u16 {
	for i in 0..edge.link_count as usize {
		if edge.links[i].src_port_id == src_port_id && edge.links[i].dst_port_id == dst_port {
			return i as u16
		}
	}

	return u16::MAX
}

/*给指定的节点对添加一条链路,返回是否有实质修改，并且自动更新EDGE中的最佳链路和Metric*/
fn addLinkForNode(&mut self,edge:&mut edge_t, srcPort:u32, dstPort:u32, metric:u16)->bool {
	let idx = self.find_link_from_node(edge, srcPort, dstPort);
	let uMetric=metric as u32;
	if edge.link_count >= SPF_MAX_LINK_PER_PAIR || idx != u16::MAX {
		return false
	}

	edge.links[edge.link_count as usize].src_port_id = srcPort;
	edge.links[edge.link_count as usize].dst_port_id = dstPort;
	edge.links[edge.link_count as usize].metric = uMetric;

	let mut ret = false;
	if uMetric < edge.metric {
		edge.metric = uMetric;
		edge.best_link = edge.link_count;
		ret = true
	}
	edge.link_count+=1;

	return ret

}

/*更新链路的最佳路径情况*/
fn update_best_link(&mut self,edge:&mut edge_t)->bool {

	let mut idx=u16::MAX;
	let mut m = METRIC_INFINITY as u32;
	for i in 0..edge.link_count {
		if edge.links[i as usize].metric < m {
			m = edge.links[i as usize].metric;
			idx = i
		}
	}

	if edge.metric == m {
		return false
	} 
	edge.best_link = idx;
	edge.metric = m;

	return true
}

/*给指定的节点对删除一条链路,返回是否形成实际修改，并且自动更新EDGE中的最佳链路和Metric*/
fn deleteLinkForNode(&mut self,edge:&mut edge_t, srcPort:u32, dstPort:u32)->bool {
	let idx = self.find_link_from_node(edge, srcPort, dstPort);

	if idx == u16::MAX {
		return false
	}

	/*删除链路的操作实际上是移动链路*/
	for i in idx..edge.link_count-1 {
		edge.links[i as usize] = edge.links[i as usize+1].clone();
	}

	edge.link_count-=1;

	return self.update_best_link(edge)
}

/*给指定的节点对更新一条链路,返回是否确实发生变化，并且自动更新EDGE中的最佳链路和Metric*/
fn updateLinkForNode(&mut self,edge:&mut edge_t, srcPort:u32, dstPort:u32, metric:u16)->bool {
	let idx = self.find_link_from_node(edge, srcPort, dstPort);

	if idx == u16::MAX {
		return false
	}

	if edge.links[idx as usize].metric == metric as u32 {
		return false
	}
	edge.links[idx as usize].metric = metric as u32;

	return self.update_best_link(edge)
}

/*增加一条边，以及metric,Metric越小表示链路质量越好，取值建议在0~65535之间*/
pub fn  AddEdge(&mut self,src_node_id:u32,srcPort:u32, dst_node_id:u32, dstPort:u32, metric:u16)->errcode::RESULT {

	let idxSrc = match self.getIndexById(src_node_id) {
        None=>return errcode::ERROR_NOT_FOUND,
        Some(v)=>v as usize,
    };
	let idxDst = match self.getIndexById(dst_node_id) {
        None=>return errcode::ERROR_NOT_FOUND,
        Some(v)=>v as usize,       
    };

	/*是否需要支持两个相同节点之间有多条不同的链路*/
    let cm=unsafe {&mut *self.cost_matrix};
	cm[idxSrc][idxDst].changed = self.addLinkForNode(&mut cm[idxSrc][idxDst].edge, srcPort, dstPort, metric);

	//fmt.Printf("Src=%d,dst=%d, orig_metric=%d,Metric=%d\n", idxSrc, idxDst, metric,
	//	self.costMatrix[idxSrc][idxDst].edge.metric)
	return errcode::RESULT_SUCCESS
}

/*删除一条边，实际上是将两个节点之间的Cost值设为无穷大*/
pub fn  DeleteEdge(&mut self,src_node_id:u32, srcPort:u32, dst_node_id:u32, dstPort:u32)->errcode::RESULT {

	let idxSrc = match self.getIndexById(src_node_id) {
        None=>return errcode::ERROR_NOT_FOUND,
        Some(v)=>v as usize,
    };
	let idxDst = match self.getIndexById(dst_node_id) {
        None=>return errcode::ERROR_NOT_FOUND,
        Some(v)=>v as usize,       
    };

    let cm=unsafe {&mut *self.cost_matrix};
	cm[idxSrc][idxDst].changed = self.deleteLinkForNode(&mut cm[idxSrc][idxDst].edge, srcPort, dstPort);	
	return errcode::RESULT_SUCCESS
}

/*更新一条边的metric*/
pub fn  UpdateEdge(&mut self,src_node_id:u32, srcPort:u32, dst_node_id:u32, dstPort:u32, metric:u16)->errcode::RESULT {
	let idxSrc = match self.getIndexById(src_node_id) {
        None=>return errcode::ERROR_NOT_FOUND,
        Some(v)=>v as usize,
    };
	let idxDst = match self.getIndexById(dst_node_id) {
        None=>return errcode::ERROR_NOT_FOUND,
        Some(v)=>v as usize,       
    };

    let cm=unsafe {&mut *self.cost_matrix};
    cm[idxSrc][idxDst].changed = self.updateLinkForNode(&mut cm[idxSrc][idxDst].edge, srcPort, dstPort, metric);
	

	return errcode::RESULT_SUCCESS
}

/*计算一个源节点到系统中所有其它节点的路径，如果要计算所有的节点，应该遍历系统中所有节点进行计算*/
pub fn  CalcOneSrcPath(&mut self,src_node_id:u32)->errcode::RESULT {
	
	let idxSrc = match self.getIndexById(src_node_id) {
        None=>return errcode::ERROR_NOT_FOUND,
        Some(v)=>v as usize,
    };
	//println!("[CalcOneSrcPath]begin formulate priority queue,idx_src_id={},index={}",src_node_id,idxSrc);
	let mut dist:Vec<u32>=Vec::with_capacity(SPF_MAX_NODE_NUM);
	let mut c=0u32;
    let mut alt=0u32;
	let mut pq=priority_queue_t::new();
		
	
    let cm=unsafe {&mut *self.cost_matrix};
	let max_node_idx=(self.max_node_idx+1) as usize;
	/*初始化开销向量,dist是src到每个节点的初始距离*/
	let prev = unsafe { &mut (&mut *self.path_vector)[idxSrc][0..max_node_idx]};
	for n in 0..max_node_idx {
		let k=n as usize;
		if k == idxSrc {
			c = 0;
		} else {
			c = METRIC_INFINITY as u32;//cm[idxSrc as usize][k as usize].edge.metric;
		}
        dist.push(c);
		prev[k].clear();
		prev[k].push(u16::MAX);

		/*插入PriorityQueue的记录，把从源节点到系统中所有其它节点的开销插入优先级队列
		后者会自动进行排序，排名最靠前的最先出队*/
		pq.push_nosort(k as usize,cm[idxSrc][k].edge.metric);
	}
	pq.sort();
	// /self.id2idx.end_iter();
	
	//计算src到所有节点的最短路径，u始终是min<distance(src,vertex)>的节点
	//let mut first=true;
	while pq.len()>0 {
		let u= match pq.pop_min() { //弹出的实际上是当前和Src距离最近的一个节点
            None=>break,
            Some(v)=>v,
        };

		for idx in 0..pq.len() {
			let v = match pq.get_item_by_index(idx) {
				None=>continue,
				Some(v)=>v,
			};

			let metric = unsafe { cm.get_unchecked(u.node_idx).get_unchecked(v.node_idx).edge.metric};
			// /cm[u.node_idx as usize][v.node_idx as usize].edge.metric
			if  metric >= METRIC_INFINITY as u32 {
				continue
			}
			alt = unsafe { dist.get_unchecked(u.node_idx as usize)} + metric;
			if alt>=METRIC_INFINITY as u32 {
				continue
			}
			if alt < dist[v.node_idx as usize] {
				dist[v.node_idx as usize] = alt;
				prev[v.node_idx as usize][0] = u.node_idx as u16;
				pq.decrease_priority(idx, alt);
			}
		}

	}

	return errcode::RESULT_SUCCESS
}

/*读取一条SPF路径，在调用CalcOne(All)SpfPath路径计算后，就可以使用本函数获得计算后的一条路径*/
pub fn  get_spf_path(&mut self,src_node_id:u32, dst_node_id:u32)->Option<path_t> {
	let idxSrc = match self.getIndexById(src_node_id) {
        None=>return None,
        Some(v)=>v as u16,
    };
	let idxDst = match self.getIndexById(dst_node_id) {
        None=>return None,
        Some(v)=>v as u16,       
    };

	//let pv = unsafe {&mut *self.path_vector};
	let tPath = self.constructPath(idxSrc as u16, idxDst);

	return tPath
}

/*进行全部路径的计算，后续考虑增加的功能：
1、部分路径计算
2、路径变化主动通知功能*/
pub fn  calc_all_path(&mut self)->errcode::RESULT {
	let mut ids=Vec::new();
	for (node_id,_) in self.id2idx.iter() {
		ids.push(node_id.clone());
	}
	//self.id2idx.end_iter();

	for id in ids {		
		self.CalcOneSrcPath(id);
	}

	return errcode::RESULT_SUCCESS
}

#[inline(always)]
fn get_edge_metric(&self,src_node_idx:u16,dst_node_idx:u16)->u32 {
	if src_node_idx>=SPF_MAX_NODE_NUM as u16 || dst_node_idx>=SPF_MAX_NODE_NUM as u16 {
		return METRIC_INFINITY as u32
	}
	let cm=unsafe {&mut *self.cost_matrix};
	return cm[src_node_idx as usize][dst_node_idx as usize].edge.metric;
}

/*根据路径的前驱节点表，构造一条Path*/
fn  constructPath(&mut self, src:u16, dst:u16)->Option<path_t> {

	if src==dst || src>=SPF_MAX_NODE_NUM as u16 || dst>=SPF_MAX_NODE_NUM as u16 {
		return None;
	}
	let max_node_idx=(self.max_node_idx+1) as usize;
	let prev=&mut unsafe {&mut *self.path_vector}[src as usize][0..max_node_idx];
	if prev.len()==0 {
		return None;
	}
	//println!("path {}-{},len={},content={:?}",src,dst,prev.len(),prev);
	let mut rPath=path_t::new();

	let mut cur = dst;
	let mut next=dst;
	let cm=unsafe {&mut *self.cost_matrix};
	while cur != u16::MAX  {
		let dst_node = match self.get_nodeid_by_idx(cur) {
			None=> {
				next=cur;
				cur = prev[cur as usize][0];
				continue
			},
			Some(i)=>i,
		};
		let src_node = match self.get_nodeid_by_idx(prev[cur as usize][0]) {
			None=>{
				break;
			},
			Some(i)=>i,
		};
		let edge = vec![path_edge_t {
			dst_node_id:dst_node,
			src_port_id:0,
			dst_port_id:0,
			metric:self.get_edge_metric(cur,next),
		}];
		let mut e = path_node_t{src_node_id: src_node, edges:edge};

		if cur == src {
			break
		}

		if cur < max_node_idx as u16 && prev[cur as usize][0] != u16::MAX {
			let pEdge = &cm[prev[cur as usize][0] as usize][cur as usize].edge;
			e.edges[0].metric = pEdge.metric;
			e.edges[0].src_port_id = pEdge.links[pEdge.best_link as usize].src_port_id;
			e.edges[0].dst_port_id = pEdge.links[pEdge.best_link as usize].dst_port_id;
			rPath.push(e);
		} else {
			break
		}
		next=cur;
		cur = prev[cur as usize][0];

	}

	if rPath.len()==0 {
		return None;
	}
	rPath.reverse();

	return Some(rPath)

}

pub fn  print_stats(&self) {
	println!("Total Node={}, idToIdxMap={}, idxToIdMap={}\n",
		self.node_count, self.id2idx.len(), self.idx2id.len());
	//self.idxAlloc.PrintStats()
	for i in 0..self.node_count {
        let idx = i as i32;
		let id = match self.idx2id.get(&idx) {
            None=>u32::MAX,
            Some(v)=>*v,
        };

		println!("Idx={},IdxToId[{}]={}\n", i, idx, id);
	}
}


}

impl Drop for spf_matrix_t {
	fn drop(&mut self) {
		unsafe { 
		alloc::dealloc(self.cost_matrix as * mut u8,
			alloc::Layout::from_size_align_unchecked(SPF_MAX_NODE_NUM*SPF_MAX_NODE_NUM*mem::size_of::<edge_info_t>(), 1));
		
		alloc::dealloc(self.path_vector as * mut _,
			alloc::Layout::from_size_align_unchecked(SPF_MAX_NODE_NUM*SPF_MAX_NODE_NUM*mem::size_of::<path_t>(), 1));
		}
	}
}

/*文本形式显示一个路径*/
pub fn print_path(tPath:&path_t) {

	let pl = tPath.len();
	let mut sPath = String::default();
	for i in 0..pl{
		sPath = format!("{}:[{}:{}-->{}:{},Metric={}]", sPath, tPath[i].src_node_id, tPath[i].edges[0].src_port_id,
			tPath[i].edges[0].dst_node_id, tPath[i].edges[0].dst_port_id, tPath[i].edges[0].metric)
	}
	if pl>0 {
		println!("Path_len={}, from {} to {}, path:{}\n", pl, tPath[0].src_node_id, tPath[pl-1].edges[0].dst_node_id, sPath);
	} else {
		println!("path length is zero!");
	}
	

}

