#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use super::*;
use super::mac_addr::mac_addr_t;
use std::{net::Ipv4Addr};

pub mod ArpOperation {
    pub const ARP_OP_REQ:u16 = 1;
    pub const ARP_OP_RESP:u16 = 2;
    pub const ARP_OP_REQ_REVERSE:u16 = 3;
    pub const ARP_OP_RESP_REVERSE:u16 = 4;
}
pub mod ArpHardwareTypes {
    pub const HARDWARE_ETHERNET:u16=1;
    pub const HARDWARE_IEEE802:u16=6;
    pub const HARDWARE_HDLC:u16=17;
    
}
#[derive(Clone)]
pub struct Arp {
    pub dst_mac:mac_addr_t,
    pub src_mac:mac_addr_t,
    pub ethernet_type:u16,
    pub vlan_id:u16,
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hw_addr_len: u8,
    pub proto_addr_len: u8,
    pub operation: u16,
    pub arp_sha: mac_addr_t,
    pub arp_spa: Ipv4Addr,
    pub arp_tha: mac_addr_t,
    pub arp_tpa: Ipv4Addr,
}

impl Arp {
    //从一个完整的以太网报文导入
    pub fn from_packet(pkt:&[u8])->Option<Arp> {
        if pkt.len()< ARP_PACKET_SIZE+ETHERNET_HDR_SIZE {
            return None;
        }
        let mut arp_pkt = Self::new(ArpOperation::ARP_OP_REQ,&mac_addr_t::new_broadcast(),&mac_addr_t::new_broadcast());
        arp_pkt.dst_mac = mac_addr_t::new(
            pkt[0], pkt[1], pkt[2], pkt[3], pkt[4], pkt[5],
        );
        arp_pkt.src_mac = mac_addr_t::new(
            pkt[6], pkt[7], pkt[8], pkt[9], pkt[10], pkt[11],
        );
        let ether_type: u16 = ((pkt[12] as u16) << 8) + pkt[13] as u16;
        let mut payload_idx = 14; 
        if ether_type == EthernetTypes::Etherhet_Vlan {
            arp_pkt.vlan_id =  ((pkt[14] as u16) << 8) + pkt[15] as u16;
            arp_pkt.ethernet_type = ((pkt[16] as u16) << 8) + pkt[17] as u16;
            payload_idx+=4;
        } else {
            arp_pkt.ethernet_type = ether_type;
        }
        if arp_pkt.ethernet_type!=EthernetTypes::Etherhet_ARP {
            return None;
        }
        Self::parse_arp(&mut arp_pkt, &pkt[payload_idx..]);
        return Some(arp_pkt);
        
    }
    pub fn new(arp_op:u16,dst_mac:&mac_addr_t,src_mac:&mac_addr_t)->Arp {
        return Arp {
            dst_mac:dst_mac.clone(),
           src_mac:src_mac.clone(),
           ethernet_type:EthernetTypes::Etherhet_ARP,
           vlan_id:0,
            hardware_type:ArpHardwareTypes::HARDWARE_ETHERNET,
            protocol_type:EthernetTypes::Etherhet_Ipv4,
            hw_addr_len:MAC_ADDR_SIZE as u8,
            proto_addr_len:IPV4_ADDR_LEN as u8,
            operation:arp_op,
            arp_sha:src_mac.clone(),
            arp_spa:Ipv4Addr::from([0,0,0,0]),
            arp_tha:dst_mac.clone(),
            arp_tpa:Ipv4Addr::from([0,0,0,0]),
        };
    }
    pub fn set_tpa(&mut self,ipv4:&Ipv4Addr) {
        self.arp_tpa = ipv4.clone();
    }

    pub fn set_spa(&mut self,ipv4:&Ipv4Addr) {
        self.arp_spa = ipv4.clone();
    }

    fn parse_arp(arp:&mut Arp,pkt: &[u8]) {
        if pkt.len()<ARP_PACKET_SIZE {
            return;
        }
        arp.hardware_type = pkt[1] as u16 + ((pkt[0] as u16) << 8);
        arp.protocol_type = pkt[3] as u16 + ((pkt[2] as u16) << 8);
        arp.hw_addr_len = pkt[4];
        arp.proto_addr_len = pkt[5];
        arp.arp_sha = mac_addr_t::new(pkt[8], pkt[9], pkt[10], pkt[11], pkt[12], pkt[13]);
        arp.arp_tha = mac_addr_t::new(pkt[18], pkt[19], pkt[20], pkt[21], pkt[22], pkt[23]);
        arp.arp_spa = Ipv4Addr::from([pkt[14],pkt[15],pkt[16],pkt[17]]);
        arp.arp_tpa = Ipv4Addr::from([pkt[24],pkt[25],pkt[26],pkt[27]]);
        arp.operation = ((pkt[6] as u16) <<8 )+pkt[7] as u16;
    }

    //转换成一段EthernetPacket
    pub fn to_ethernet_packet(&mut self)->Vec<u8> {
        let mut pVec:Vec<u8>=Vec::new();
        pVec.extend_from_slice(&self.dst_mac.to_slice());
        pVec.extend_from_slice(&self.src_mac.to_slice());
        if self.vlan_id>0 {
            pVec.extend_from_slice(&EthernetTypes::Etherhet_Vlan.to_be_bytes()[..]);
            pVec.extend_from_slice(&self.vlan_id.to_be_bytes()[..]);
        }
        pVec.extend_from_slice(&self.ethernet_type.to_be_bytes()[..]);
        pVec.extend_from_slice(&self.generate_arp_payload().as_slice());
        return pVec;
    }

     //转换成一段EthernetPacket
     pub fn to_arp_payload(&mut self)->Vec<u8> {
        return self.generate_arp_payload();
    }
    //生成ARP Payload Packet
    fn generate_arp_payload(&self)->Vec<u8> {
        let mut pVec:Vec<u8>=Vec::new();
        pVec.extend_from_slice(&self.hardware_type.to_be_bytes()[..]);
        pVec.extend_from_slice(&self.protocol_type.to_be_bytes()[..]);
        pVec.push(self.hw_addr_len);
        pVec.push(self.proto_addr_len);
        pVec.extend_from_slice(&self.operation.to_be_bytes()[..]);
        pVec.extend_from_slice(&self.arp_sha.to_slice());
        pVec.extend_from_slice(&self.arp_spa.octets()[..]);
        pVec.extend_from_slice(&self.arp_tha.to_slice());
        pVec.extend_from_slice(&self.arp_tpa.octets()[..]);
        return pVec;
    }

    //判断是否是免费ARP
    pub fn is_gratuitous_arp(&self)->bool {
        return self.operation==ArpOperation::ARP_OP_REQ && self.arp_tpa.eq(&self.arp_spa);
    }
    //是否ARP通告，包括响应和免费ARP
    pub fn is_arp_announcement(&self)->bool {
        return self.operation==ArpOperation::ARP_OP_RESP || self.is_gratuitous_arp()
    }
    //是否普通的ARP请求
    pub fn is_arp_request(&self)-> bool {
        return self.operation==ArpOperation::ARP_OP_REQ && !self.is_gratuitous_arp()
    }

    pub fn to_string(&self)->String {
        format!("dst_mac:{},src_mac:{},ether_type:{:#02x},vlan_id:{},hardware_type={},proto_type={:#0x},
        hw_len={},proto_len={},arp_op:{},sha:{},spa:{},tha:{},tpa:{}",
        self.dst_mac,self.src_mac,self.ethernet_type,self.vlan_id,self.hardware_type,self.protocol_type,
        self.hw_addr_len,self.proto_addr_len,self.operation,self.arp_sha,self.arp_spa,
        self.arp_tha,self.arp_tpa)
    }
}

