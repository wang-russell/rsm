#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

///FEC模块，实现多个报文计算形成一个FEC报文；同时提供多个报文(含FEC)恢复成一个原始报文
/// 设计假定报文都是有编号的，一组报文从起始编号开始N个报文编码形成一个FEC报文
/// 发端必须提供全部原始报文才能够编码形成一个FEC报文，收端在N个原始报文和1个FEC报文中只要收齐N个报文即结束
use super::*;
use crate::common::{self,errcode};
use std::mem;

const FEC_TYPE_ENCODER:u8=0;
const FEC_TYPE_DECODER:u8=1;

const MAX_FEC_RATIO:usize = 10;
const MAX_FEC_BUF_SIZE:usize = MAX_PKT_BUF_SIZE as usize;
///报文头部
#[derive(Clone,Debug)]
#[repr(C)]
struct fec_header_t {
    start_seq:u64,
    src_pkts_num:u8,
    alg_type:u8,
    max_pkt_len:u16,
    src_pkts_len:[u16;MAX_FEC_RATIO],
}
const FEC_HDR_SIZE:usize = mem::size_of::<fec_header_t>();

///FEC报文，实现对FEC的处理
#[derive(Clone,Debug)]
#[repr(C)]
pub struct fec_packet_t {
    hdr:fec_header_t,
    body:[u8;MAX_FEC_BUF_SIZE],
    real_len:u16,
    fec_type:u8,
    ///FEC的N:1比例，多少个报文计算一个FEC报文
    ratio:u8, 
    src_pkts_added:[bool;MAX_FEC_RATIO],
    fec_exist:bool,
}


///FEC处理，Encoder/Decoder同一套代码
impl fec_packet_t {
    pub fn new()->Self {
        return Self {
            hdr:unsafe { std::mem::zeroed::<fec_header_t>() },
            body:[0;MAX_FEC_BUF_SIZE],
            real_len:0,
            fec_type:FEC_TYPE_ENCODER,
            ratio:5,
            src_pkts_added:[false;MAX_FEC_RATIO],
            fec_exist:false,
        }
    }

    fn reset(&mut self,start_seq:u64,ratio:u8) {
        //let fec_type=self.fec_type;
        self.hdr = unsafe { std::mem::zeroed::<fec_header_t>() };
        self.hdr.start_seq = start_seq;
        self.hdr.src_pkts_num = ratio;
        self.real_len = 0;
        self.fec_exist = false;
        self.ratio = ratio;
        self.body.fill(0);
        self.src_pkts_added.fill(false);
    }
    //启动一次Encoder会话
    pub fn start_encoder(&mut self,start_seq:u64,ratio:u8) {
        self.fec_type = FEC_TYPE_ENCODER;
        self.reset(start_seq, ratio)
    }

    //启动一次Decoder会话
    pub fn start_decoder(&mut self,start_seq:u64,ratio:u8) {
        self.fec_type = FEC_TYPE_DECODER;
        self.reset(start_seq, ratio);
    }

    //接收端报文是否已经收齐
    fn is_packet_recv_finished(&self)->bool {
        for i in 0..self.ratio as usize {
            if !self.src_pkts_added[i] {
                return false;
            }
        }
        return true
    }
    //判断FEC计算过程是否已经结束
    pub fn is_fec_finished(&self)->bool {        
        if self.fec_type==FEC_TYPE_ENCODER {
            return self.is_packet_recv_finished();
        } else {
            //如果是Decoder，收齐N-1个原始报文和一个FEC报文可以恢复原始报文，但是如果收齐N个报文，FEC应该结束，等待进入下一次处理周期
            let mut expect = 0;
            for i in 0..self.ratio as usize {
                if self.src_pkts_added[i] {
                    expect+=1;
                }
            }
            if self.fec_exist {
                expect+=1;
            }
            //println!("ration={},expect={}",self.ratio,expect);
            if expect>=self.ratio {
                return true;
            } else {
                return false;
            }
        }
    }

    pub fn is_first_packet(&self)->bool {
        for i in 0..self.ratio as usize {
            if self.src_pkts_added[i] {
                return false
            }
        }
        if self.fec_exist {
            return false
        }

        return true
    }

    fn set_raw_packet_state(&mut self,idx:usize,len:u16) {
        self.src_pkts_added[idx] = true;
        self.hdr.src_pkts_len[idx]=len;
        if self.real_len< len {
            self.real_len = len;
            self.hdr.max_pkt_len = len;
        }

    }
    //加入一个RawPacket，如果FEC没有结束，返回ERROR_NO_OP，成功则应用需要读取相应的报文
    // 如果报文丢包同时且发生乱序，目前代码还需要优化
    pub fn add_raw_packet(&mut self,seq:u64,pkt:&[u8])->errcode::RESULT {
        if seq<self.hdr.start_seq || seq>=self.hdr.start_seq+self.ratio as u64 || pkt.len()>MAX_FEC_BUF_SIZE{
            return errcode::ERROR_INVALID_MSG
       }  
        if self.fec_type == FEC_TYPE_DECODER && self.is_fec_finished() {
            return errcode::ERROR_NO_OP;
        } else if self.is_packet_recv_finished() {
            return errcode::ERROR_NO_OP;
        }
       
       let idx=(seq-self.hdr.start_seq) as usize;
       if self.src_pkts_added[idx] {
        return errcode::ERROR_ALREADY_EXIST;
       }

       if self.is_first_packet() {
        self.set_raw_packet_state(idx,pkt.len() as u16);
        self.body[0..pkt.len()].copy_from_slice(pkt);
        return errcode::ERROR_NO_OP
       } 

       self.set_raw_packet_state(idx,pkt.len() as u16);
       self.calc_fec_packet(pkt);
       if self.is_fec_finished() {
            errcode::RESULT_SUCCESS
        } else {
            return errcode::ERROR_NO_OP
        }
    }

    ///FEC Encoder和Decoder进入下一个处理会话，Seq顺延
    pub fn next_session(&mut self,new_ratio:u8) {
        let seq = self.hdr.start_seq+(new_ratio as u64);
        self.reset(seq, new_ratio)
    }
    pub fn next_start_seq(&self)->u64 {
        return self.hdr.start_seq+(self.ratio as u64);
    }

    //加入一个FecPacket，如果FEC没有结束，返回ERROR_NO_OP，成功则返回编码的FEC，或者解码的原始报文   
    pub fn add_fec_packet(&mut self,seq:u64,pkt:&[u8])->errcode::RESULT {
        //let hdr_ptr = unsafe {&*(&pkt[0] as *const u8 as *const fec_header_t)};

        if seq<self.hdr.start_seq || seq>=self.hdr.start_seq+self.ratio as u64 || pkt.len()<=FEC_HDR_SIZE || pkt.len()>MAX_FEC_BUF_SIZE+FEC_HDR_SIZE{
            return errcode::ERROR_INVALID_MSG
       }
       
        if self.is_fec_finished() {
            self.fec_exist = true;
            return errcode::ERROR_NO_OP;
        }
        self.fec_exist = true;
        
        unsafe {
            std::ptr::copy_nonoverlapping(pkt.as_ptr(), &mut self.hdr as *mut _ as * mut u8,FEC_HDR_SIZE);
        }
        self.fec_exist = true;
        
        if self.is_first_packet() {
            self.body[0..pkt.len()-FEC_HDR_SIZE].copy_from_slice(&pkt[FEC_HDR_SIZE..]);
            return errcode::ERROR_NO_OP
        }

        self.calc_fec_packet(&pkt[FEC_HDR_SIZE..]);
        if self.is_fec_finished() {
            errcode::RESULT_SUCCESS
        } else {
            return errcode::ERROR_NO_OP
        }
        
    }

    //内部实际计算FEC Packet
    //FEC计算需要对报文进行填充，填充到最大MTU
    fn calc_fec_packet(&mut self,pkt:&[u8])->errcode::RESULT {

        let pkt_len = pkt.len();
        let u64_len:usize = pkt_len as usize/mem::size_of::<u64>();
        
        //let mut tmp_pkt = [0u8;MAX_PKT_BUF_SIZE as usize];
        

        //tmp_pkt[0..pkt_len].copy_from_slice(pkt);

        let u64_dst = self.body.as_mut_ptr();
        let u64_src = pkt.as_ptr();
        //let u64_dst = unsafe { &mut *(&mut self.body[0] as *mut _ as *mut [u64;u64_len]) };
        //let u64_src = unsafe { &*(tmp_pkt.as_ptr() as *const _ as *const [u64;u64_len]) };

        for i in 0..u64_len {
            unsafe {
                *((u64_dst as usize + i*mem::size_of::<u64>()) as *mut u64) ^= *((u64_src as usize + i*mem::size_of::<u64>()) as *const u64);
            }
        }

        for j in u64_len*mem::size_of::<u64>()..pkt_len as usize {
            self.body[j] ^=pkt[j];
        }

        for k in pkt_len..MAX_FEC_BUF_SIZE {
            self.body[k] ^=0u8;
        }

        errcode::RESULT_SUCCESS
    }

    ///获取丢失报文的序号
    fn get_lost_packet_seq(&self)->(u64,usize) {
        for i in 0..self.ratio as usize{
            if !self.src_pkts_added[i] {
                return (self.hdr.start_seq+(i as u64),i)
            }
        }
        return (0,0);
    }

    ///读取RawPacket，用于FEC计算恢复的原始报文;buf是应用传入的缓冲区，应该大于MTU长度
    /// 返回报文实际长度和恢复的rawPacket的序号，如果无丢包，则返回(0,0)
    pub fn get_raw_packet(&self,buf:&mut [u8])->(usize,u64) {
        if buf.len() < self.real_len as usize {
            return (0,0)
        }
        let (seq,idx)=self.get_lost_packet_seq();
        if seq==0 {
            return (0,0);
        }
        unsafe {
            std::ptr::copy_nonoverlapping(self.body.as_ptr(), buf.as_mut_ptr(), self.hdr.src_pkts_len[idx] as usize);
        }

        (self.hdr.src_pkts_len[idx] as usize,seq)
    }

    pub fn get_raw_packet_slice(&self)->Option<(&[u8],u64)> {
        let (seq,idx)=self.get_lost_packet_seq();
       
        if seq==0  || idx>=self.ratio as usize || self.hdr.src_pkts_len[idx]>=MAX_FEC_BUF_SIZE as u16{
            return None;
        }
       // assert!(idx<self.ratio as usize && self.hdr.src_pkts_len[idx]<MAX_FEC_BUF_SIZE as u16);
        return Some((&self.body[0..self.hdr.src_pkts_len[idx] as usize],seq));
    }

    ///读取FecPacket，用于FEC计算结果;buf是应用传入的缓冲区，应该大于MTU长度
    /// 返回FEC报文实际长度和起始rawPacket序号
    pub fn get_fec_packet(&self,buf:&mut [u8])->(usize,u64) {
        let fec_len = self.real_len as usize+FEC_HDR_SIZE;
        if buf.len() < fec_len {
            return (0,0)
        }
        unsafe {
            std::ptr::copy_nonoverlapping(&self.hdr as *const _ as *const u8, buf.as_mut_ptr(), fec_len);
        }
        (fec_len,self.hdr.start_seq+self.ratio as u64-1)
    }

    pub fn get_packet_len(&self)->usize {
        self.real_len as usize + FEC_HDR_SIZE
    }
    pub fn get_payload_len(&self)->usize {
        self.real_len as usize
    }

    pub fn as_slice(&self)->&[u8] {
        return unsafe {
            &(*(&self.hdr as *const fec_header_t as *const u8 as *const [u8;MAX_FEC_BUF_SIZE+FEC_HDR_SIZE]))[0..FEC_HDR_SIZE+self.real_len as usize]
        };
    }

    pub fn as_mut_slice(&mut self)->&[u8] {
        return unsafe {
            &mut (*(&mut self.hdr as *mut fec_header_t as *mut u8 as *mut [u8;MAX_FEC_BUF_SIZE+FEC_HDR_SIZE]))[0..FEC_HDR_SIZE+self.real_len as usize]
        };
    }

    pub fn get_recv_pkt_count(&self)->usize {
        let mut count=0;
        for b in self.src_pkts_added {
            if b {
                count+=1;
            }
        }
        count
    }

    pub fn is_fec_pkt_recved(&self)->bool {
        self.fec_exist
    }
    pub fn get_start_seq(&self)->u64 {
        self.hdr.start_seq
    }
}

///FEC Streaming Decoder，实现接收端流式的FEC报文恢复，自动移动序号
const MAX_FEC_PACKET_IN_STREAM:usize=16;
pub struct fec_decoder_stream_t {
    fecs:[fec_packet_t;MAX_FEC_PACKET_IN_STREAM],
    start_idx:usize,
    start_seq:u64,
    ratio:u8,
    max_seq:u64,
    fec_buf:[u8;MAX_FEC_BUF_SIZE],
}

impl fec_decoder_stream_t {
    pub fn new(start_seq:u64,ratio:u8)->fec_decoder_stream_t {
        let mut stream=unsafe { mem::zeroed::<fec_decoder_stream_t>() };
        stream.init(start_seq,ratio);
        return stream
    }
    fn init(&mut self,start_seq:u64,ratio:u8) {
        self.start_idx = 0;
        self.ratio = ratio;
        self.start_seq=start_seq;
        self.max_seq=start_seq+(MAX_FEC_PACKET_IN_STREAM *ratio as usize) as u64-1;
        for i in 0..MAX_FEC_PACKET_IN_STREAM as usize {
            self.fecs[i].start_decoder(start_seq+ i as u64 * (ratio as u64),ratio);
        }
    }

    pub fn start(&mut self,start_seq:u64,ratio:u8) {
        self.init(start_seq,ratio);
    }

    pub fn cleanup(&mut self) {
        while self.fecs[self.start_idx].is_fec_finished() {
            self.start_seq+= self.ratio as u64;
            self.fecs[self.start_idx].start_decoder(self.max_seq+1,self.ratio);
            self.max_seq +=self.ratio as u64;
            self.start_idx = (self.start_idx +1) % MAX_FEC_PACKET_IN_STREAM;
        }
    }
    ///将当前序号移动至包含new_seq的区间
    fn move_to_seq(&mut self,new_seq:u64) {        
        self.cleanup();
        if new_seq<= self.max_seq {
            return
        }
        //如果new_seq超过当前窗口2倍，重新初始化
        if new_seq>=self.max_seq+self.seq_window_len() {
            self.init(new_seq/self.ratio as u64 * self.ratio as u64,self.ratio);
            return
        }
        
        //计算需要移动几个FEC Packet，每个Packet的seq长度为self.ratio
        let step = common::ceiling(new_seq - self.max_seq,self.ratio as u64) as usize;
        let new_idx = (self.start_idx + step) % MAX_FEC_PACKET_IN_STREAM;
        let new_start_seq = self.start_seq+(step as u64 * self.ratio as u64);
        let mut  idx = self.start_idx;
        let mut seq = self.start_seq+(MAX_FEC_PACKET_IN_STREAM + step-1) as u64*self.ratio as u64;
        while idx!=new_idx {
            self.fecs[idx].start_decoder(seq,self.ratio);
            idx=(idx + 1) % MAX_FEC_PACKET_IN_STREAM;
            seq+=self.ratio as u64;
        }
        //self.fecs[idx].start_decoder(seq,self.ratio);
        self.start_idx = new_idx;
        self.start_seq = new_start_seq;
        self.max_seq = new_start_seq+(self.ratio as u64 * MAX_FEC_PACKET_IN_STREAM as u64)-1;
    }

    fn get_index_by_seq(&self,seq:u64)->usize {
        if seq<self.start_seq {
            return 0;
        }

        (((seq-self.start_seq) / self.ratio as u64)  as usize + self.start_idx)  % MAX_FEC_PACKET_IN_STREAM
    }

    fn seq_window_len(&self)->u64 {
        self.ratio as u64*MAX_FEC_PACKET_IN_STREAM as u64
    }

    ///产生了序号失步，需要重同步
    fn need_resync(&mut self,recv_seq:u64)->bool {
        if recv_seq<self.start_seq {
            if recv_seq + self.seq_window_len()< self.start_seq || (self.start_seq - recv_seq+1) % self.ratio as u64!=0 {
                return true;
            }
            return false;
        } else if (recv_seq - self.start_seq+1) % self.ratio as u64!=0 {
            return true
        }
        false
    }
    //fn get_decoder_by_seq(&mut self,seq:u64)->
    pub fn add_fec_packet(&mut self,seq:u64,pkt:&[u8])->Result<(&[u8],u64),errcode::RESULT> {        
        if self.need_resync(seq) {
            self.init(seq+1, self.ratio);
            return Err(errcode::ERROR_NO_OP);
        }
        if seq<self.start_seq {
            return Err(errcode::ERROR_INVALID_MSG);
        }

        if seq>self.max_seq {
            self.move_to_seq(seq);
        }
        let idx= self.get_index_by_seq(seq);
        let res = self.fecs[idx].add_fec_packet(seq,pkt);
        if res==errcode::RESULT_SUCCESS {
            let (seq,len) = match self.fecs[idx].get_raw_packet_slice() {
                None=> return Err(errcode::ERROR_COMMON),
                Some((pkt,seq))=>  { 
                    self.fec_buf[0..pkt.len()].copy_from_slice(pkt);
                    (seq,pkt.len())                 
                },
            };
            self.cleanup();
            return Ok((&self.fec_buf[0..len],seq));
        } else if self.fecs[idx].is_fec_finished() {
            self.cleanup();
        }

        Err(errcode::ERROR_NO_OP)
    }

    ///add_raw_packet，加入一个RawPacket，判断是否已经解码成功
    pub fn add_raw_packet(&mut self,seq:u64,pkt:&[u8])->Result<(&[u8],u64),errcode::RESULT> {
        if seq<self.start_seq {
            return Err(errcode::ERROR_INVALID_MSG);
        }
        if seq>self.max_seq {
            self.move_to_seq(seq);
        }
        let idx= self.get_index_by_seq(seq);
        let res = self.fecs[idx].add_raw_packet(seq,pkt);
        if res==errcode::RESULT_SUCCESS {
            let (seq,len) = match self.fecs[idx].get_raw_packet_slice() {
                None=> return Err(errcode::ERROR_COMMON),
                Some((pkt,seq))=>  { 
                    self.fec_buf[0..pkt.len()].copy_from_slice(pkt);
                    (seq,pkt.len())                 
                },
            };
            self.cleanup();
            return Ok((&self.fec_buf[0..len],seq));
        } else if self.fecs[idx].is_fec_finished() {
            self.cleanup();
        }

        Err(errcode::ERROR_NO_OP)
    }

    pub fn get_start_seq(&self)->u64 {
        self.start_seq
    }

    pub fn get_start_idx(&self)->usize {
        self.start_idx
    }
    
    pub fn print_stats(&self) {
        let mut idx=self.start_idx;
        println!("Fec Stream internal State, start_seq={},Max_seq={},ratio={},start_idx={}",
            self.start_seq,self.max_seq,self.ratio,self.start_idx);
        while idx!= (self.start_idx+MAX_FEC_PACKET_IN_STREAM-1) % MAX_FEC_PACKET_IN_STREAM {
            let p = &self.fecs[idx];
            println!("No {} fec packet,start_seq={},recv {} raw packet,recv_fec={}",idx,
            p.get_start_seq(),p.get_recv_pkt_count(),p.fec_exist);
            idx = (idx+1) % MAX_FEC_PACKET_IN_STREAM;
        }
        let p = &self.fecs[idx];
            println!("No {} fec packet,start_seq={},recv {} raw packet,recv_fec={}",idx,
            p.get_start_seq(),p.get_recv_pkt_count(),p.fec_exist);
    }
}