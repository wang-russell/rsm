//提供一个有序、有界的环，每个元素都有唯一的序号，按序号进行存取
//主要场景：可靠传输的报文发送队列，确认、重传和查重
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
use super::*;

#[derive(Copy,Clone,PartialEq,Debug)]
pub enum EItemState {
    ITEM_STATE_IDLE = 0,
    ITEM_STATE_USED = 1,
    ITEM_STATE_DELETED = 2,
}
pub const INVALID_SEQ: u64 = 2 ^ 64 - 1;
pub const INVALID_INDEX: usize = 0xFFFFFFFF;
pub struct data_item<T> {
    seq_no: u64,
    state: EItemState,
    item: Option<T>,
}
impl <T>data_item<T> {
    pub fn new()->Self{
        return Self{
            seq_no:0,
            state:EItemState::ITEM_STATE_IDLE,
            item:None,
        };
    }
}
pub struct index_ring_t<T> {
    data: Vec<data_item<T>>,
    head: usize,
    tail: usize,
    min_seq: u64,
    max_seq: u64,
    capacity: usize,
    expect_item:u64,
    recv_item:u64,
}

impl<T> index_ring_t<T> {
    pub fn new(ring_capacity: usize, start_seq: u64) -> Self {
            let mut vItem:Vec<data_item<T>>=Vec::with_capacity(ring_capacity);
            for _ in 0..ring_capacity {
                vItem.push(data_item::new())
            }
                       
            return Self {
                data: vItem,
                head: INVALID_INDEX,
                tail: INVALID_INDEX,
                min_seq: start_seq,
                max_seq: start_seq,
                capacity: ring_capacity,
                expect_item:0,
                recv_item:0,
            };
        }

    fn is_valid_index(&self,idx:usize)->bool {
        if idx>=self.capacity {
            return false;
        }
        if self.head<=self.tail {
            return idx>=self.head && idx<=self.tail
        } else {
            return (idx>=self.head && idx<self.capacity) || idx<=self.tail
        }
        
       
    }
    fn set_item(&mut self, seq: u64, idx: usize, item: T) -> errcode::RESULT {
        
        if self.data[idx].state == EItemState::ITEM_STATE_USED && self.data[idx].seq_no == seq {
            return errcode::ERROR_ALREADY_EXIST;
        }
        self.data[idx].state = EItemState::ITEM_STATE_USED;
        self.data[idx].seq_no = seq;
        self.data[idx].item = Some(item);
        self.recv_item+=1;
        errcode::RESULT_SUCCESS
    }
    //添加一条记录,如果序号超出当前范围，往前移动；但是如果序号超出当前最大值+capacity，则认为是非法序号，丢弃
    pub fn add_item(&mut self, seq: u64, item: T) -> errcode::RESULT {
        //序号小于min_seq，或者比max_seq大capacity，认为是非法序号
        if seq<self.min_seq || seq>=self.max_seq+self.capacity as u64{
            return errcode::ERROR_INVALID_INDEX;
        }
        let distance = if seq<=self.max_seq {0} else {seq-self.max_seq};
        if self.get_ring_len() == 0 {
            self.head = 0;
            self.tail = distance as usize;
            //self.min_seq = seq;
            self.max_seq = seq;
            self.expect_item+=1+distance;
            return self.set_item(seq, self.tail, item);
        }

        if seq >= self.min_seq && seq <= self.max_seq {
            let idx = self.get_index_by_seq(seq);
            if idx < self.capacity {                
                return self.set_item(seq, idx, item);
            }
        } else if seq > self.max_seq {            
            self.expect_item+=distance;
            self.max_seq = seq;
            if self.get_ring_len() + distance as usize > self.capacity {
                //self.min_seq = self.min_seq + distance;
                self.remove_item_before_seq(self.min_seq + distance-1, true);
                //self.head = (self.head + distance as usize) % self.capacity;
            }
            self.tail = (self.tail + distance as usize) % self.capacity;
            return self.set_item(seq, self.tail, item);
        }

        return  errcode::ERROR_INVALID_INDEX;

        
    }

    fn incr_hdr_index(&mut self) {
        match self.get_ring_len() {
            0 => (),
            1 => {
                self.head = INVALID_INDEX;
                self.tail = INVALID_INDEX;
                self.min_seq += 1;
                self.max_seq=self.min_seq;
            }
            _ => {
                self.min_seq += 1;
                self.head = (self.head + 1) % self.capacity;
            }
        }
    }

    fn incr_tail_index(&mut self) {
        match self.get_ring_len() {
            0 => {
                self.head = 0;
                self.tail = 0;
            }
            _ => {
                self.tail = (self.tail + 1) % self.capacity;
                self.max_seq += 1;
            }
        }
    }

    fn inner_delete_item(&mut self,idx:usize) {
        assert!(idx < self.capacity);
            self.data[idx].state = EItemState::ITEM_STATE_DELETED;
            self.data[idx].item = None;
            if idx == self.head {
                self.incr_hdr_index()
            }
    }
    //删除一个元素，如果数据不一致，需要进行补救
    pub fn remove_item(&mut self, seq: u64) -> errcode::RESULT {
        let idx = self.get_index_by_seq(seq);
        if idx < self.capacity {
            let state = self.data[idx].state;
            self.inner_delete_item(idx);
            if  state == EItemState::ITEM_STATE_USED {                
                errcode::RESULT_SUCCESS
            } else {
                return errcode::ERROR_NOT_FOUND;
            }
        } else {
            return errcode::ERROR_NOT_FOUND;
        }
    }

    //删除指定序号之前所有的数据，如果是强制删除，则无论序号状态如何均予以删除，否则判断状态是否是deleted
    pub fn remove_item_before_seq(&mut self, seq: u64,force:bool) -> errcode::RESULT {
        let idx = self.get_index_by_seq(seq);
        assert!(idx==INVALID_INDEX || idx<self.capacity);
        if idx >= self.capacity  {
            return errcode::ERROR_INVALID_INDEX;
        }
        
        let mut head=self.head;
        while head!=((idx+1) % self.capacity) {
            if self.data[head].state==EItemState::ITEM_STATE_DELETED || force {
                self.inner_delete_item(head);
            } else {
                return errcode::ERROR_INVALID_STATE;
            }
            head = (head+1) % self.capacity;
        }
        
        errcode::RESULT_SUCCESS
    }
    //根据序号获取下标
    pub fn get_index_by_seq(&self, seq: u64) -> usize {
        if seq < self.min_seq || seq > self.max_seq || self.head > self.capacity {
            return INVALID_INDEX;
        }
        return ((seq - self.min_seq) as usize + self.head) % self.capacity;
    }
    //根据序号获取实际的数据
    pub fn get_item_by_seq(&mut self, seq: u64) -> Option<&mut T> {
        let idx = self.get_index_by_seq(seq);
        if idx != INVALID_INDEX && self.data[idx].state == EItemState::ITEM_STATE_USED {
            match &mut self.data[idx].item {
                None => return None,
                Some(ref mut s) => return Some(s),
            }
        } else {
            return None;
        }
    }

    pub fn get_head_item(&mut self) -> Option<&mut T> {
        if self.get_ring_len() > 0 {
            match &mut self.data[self.head].item {
                None=>return None,
                Some(item)=>return Some(item),                
            }
            
        } else {
            return None;
        }
    }

    pub fn get_head_index(&self) -> usize {
        self.head
    }
    pub fn get_tail_index(&self) -> usize {
        self.tail
    }

    pub fn get_head_seq(&self) -> u64 {
        self.min_seq
    }
    pub fn get_tail_seq(&self) -> u64 {
        self.max_seq
    }

    pub fn get_ring_len(&self) -> usize {
        if self.head == INVALID_INDEX || self.tail == INVALID_INDEX {
            return 0;
        }
        if self.head <= self.tail {
            return self.tail - self.head + 1;
        } else {
            return self.tail + self.capacity - self.head + 1;
        }
    }

    pub fn get_ring_capacity(&self) -> usize {
        self.capacity
    }

    pub fn get_head_item_state(&self)->EItemState {
        if self.get_ring_len()==0 {
            return EItemState::ITEM_STATE_IDLE;
        } else {
            return self.data[self.head].state;
        }

    }

    pub fn get_item_state(&self,seq:u64)->EItemState {
        let idx = self.get_index_by_seq(seq);

        if idx==INVALID_INDEX {
            return EItemState::ITEM_STATE_IDLE;
        } else {
            return self.data[idx].state;
        }

    }
    ///返回收到的报文以及期望收到的报文数量，计算丢包率
    pub fn get_recv_item_stats(&self)->(u64,u64) {
        return (self.recv_item,self.expect_item);
    }
    //清除统计
    pub fn clear_recv_stats(&mut self) {
        self.recv_item = 0;
        self.expect_item = 0;
    }
    pub fn get_loss_count(&self)->u64 {
        return self.expect_item - self.recv_item;
    }
    pub fn get_loss_rate(&self)->f32 {
        if self.expect_item==0 {
            0.0
        } else {
            return ((self.expect_item-self.recv_item)*100) as f32 / self.expect_item as f32;
        }
    }
    pub fn clear(&mut self) {
        self.clear_recv_stats();
        self.head = INVALID_INDEX;
        self.tail = INVALID_INDEX;
        self.min_seq = 1;
        self.max_seq = 1;
        for i in &mut self.data {
            i.state = EItemState::ITEM_STATE_IDLE;
            i.item=None;
        }

    }
    pub fn to_string(&self)->String {
        format!("capacity:{},len:{},head:{},tail:{},seq:<{}-{},expect:{},recved={}>",
        self.capacity,self.get_ring_len(),self.head,self.tail,self.min_seq,self.max_seq,self.expect_item,self.recv_item)
    }
}
