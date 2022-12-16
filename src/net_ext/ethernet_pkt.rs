#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
use super::*;
use super::mac_addr::mac_addr_t;
use std::net::{IpAddr};
use std::fmt;
use crate::common::errcode;


///ethernet packet parse result
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ethernet_packet_info_t {
    l3_offset: u16, //l3 header offset
    l4_offset: u16, //l4 header offset
    payload_offset:u16,//user payload offset
    total_len:u16,
    pub dst_mac: mac_addr_t,
    pub src_mac: mac_addr_t,
    pub vlan_layers: u8, //0 no vlan header,1-vlan，2-QinQ
    pub vlans: [u16; 2],
    pub ether_type: u16,
    pub mpls_label: u32,
    pub arp_op:u16,
    pub arp_sha: mac_addr_t,
    pub arp_tha: mac_addr_t,
    pub ip_hdr_len: u16,
    pub ip_payload_len: u16,
    pub ip_ttl: u8,
    pub src_ip: IpAddr,
    pub dst_ip: IpAddr,
    pub ip_proto: u8,
    pub tp_src: u16, //transport layer source port
    pub tp_dst: u16, //transport layer destination port
}


impl ethernet_packet_info_t {
    pub fn new_zero() -> ethernet_packet_info_t {
        let info:ethernet_packet_info_t=unsafe { std::mem::zeroed::<ethernet_packet_info_t>() };
        return info;
    }

    //解析报文的传输层头部
    fn parse_transport(pkt: &[u8], ip_proto: u8, info: &mut ethernet_packet_info_t) {
        if pkt.len()<UDP_HDR_SIZE {
            return;
        }
        unsafe {
        match ip_proto {
            IpProtos::Ip_Proto_TCP => {
                info.tp_src = ((*pkt.get_unchecked(0) as u16) << 8) + *pkt.get_unchecked(1) as u16;
                info.tp_dst = ((*pkt.get_unchecked(2) as u16) << 8) + *pkt.get_unchecked(3) as u16;
                let tcp_hdr_len=((pkt[12] & 0xF0)>>4)*4;
                info.payload_offset=info.l4_offset+tcp_hdr_len as u16;
            }
            IpProtos::Ip_Proto_UDP => {
                info.tp_src = ((*pkt.get_unchecked(0) as u16) << 8) + *pkt.get_unchecked(1) as u16;
                info.tp_dst = ((*pkt.get_unchecked(2) as u16) << 8) + *pkt.get_unchecked(3) as u16;
                info.payload_offset=info.l4_offset+8;
            },
            IpProtos::Ip_Proto_SCTP => {
                info.tp_src = ((*pkt.get_unchecked(0) as u16) << 8) + *pkt.get_unchecked(1) as u16;
                info.tp_dst = ((*pkt.get_unchecked(2) as u16) << 8) + *pkt.get_unchecked(3) as u16;
                info.payload_offset=info.l4_offset+12;
            }
            _ => (),
        }
    }
    }
    fn parse_ipv4(ipv4: &[u8], info: &mut ethernet_packet_info_t) {
        if ipv4.len()<IPV4_HDR_SIZE {
            return;
        }
        let mut cur_idx = 0;
        unsafe {
        info.ip_hdr_len = ((*ipv4.get_unchecked(0) & 0x0F) * 4) as u16;
        
        info.ip_payload_len = ((*ipv4.get_unchecked(2) as u16) << 8) + *ipv4.get_unchecked(3) as u16;
        if info.ip_hdr_len<IPV4_HDR_SIZE as u16 || info.ip_payload_len>ipv4.len() as u16 {
            return
        }
        info.ip_ttl = *ipv4.get_unchecked(8);
        info.ip_proto = *ipv4.get_unchecked(9);
        cur_idx += 12;

        info.src_ip = IpAddr::from([
            *ipv4.get_unchecked(cur_idx),
            *ipv4.get_unchecked(cur_idx + 1),
            *ipv4.get_unchecked(cur_idx + 2),
            *ipv4.get_unchecked(cur_idx + 3),
        ]);
        info.dst_ip = IpAddr::from([
            *ipv4.get_unchecked(cur_idx + 4),
            *ipv4.get_unchecked(cur_idx + 5),
            *ipv4.get_unchecked(cur_idx + 6),
            *ipv4.get_unchecked(cur_idx + 7),
        ]);
        info.l4_offset = info.l3_offset+info.ip_hdr_len;
        info.payload_offset=info.l4_offset;
        if ipv4.len()>=info.ip_hdr_len as usize + UDP_HDR_SIZE {
            Self::parse_transport(&ipv4[info.ip_hdr_len as usize..], info.ip_proto, info);
        }

    }
        
    }

    fn parse_ipv6(ipv6: &[u8], info: &mut ethernet_packet_info_t) {
        if ipv6.len()<IPV6_HDR_SIZE {
            return;
        }
        info.ip_hdr_len = IPV6_HDR_SIZE as u16;
        info.ip_payload_len = unsafe {  ((*ipv6.get_unchecked(4) as u16) << 8) + *ipv6.get_unchecked(5) as u16};
        if info.ip_payload_len>ipv6.len() as u16 {
            return
        }
        info.ip_proto = ipv6[6];
        info.ip_ttl = ipv6[7];
        let mut ipaddr: [u8; 16] = [0; 16];
        ipaddr.copy_from_slice(&ipv6[8..24]);

        info.src_ip = IpAddr::from(ipaddr);
        ipaddr.copy_from_slice(&ipv6[24..40]);
        info.dst_ip = IpAddr::from(ipaddr);
        info.l4_offset = info.l3_offset+info.ip_hdr_len;
        info.payload_offset=info.l4_offset;
        Self::parse_transport(&ipv6[info.ip_hdr_len as usize..], info.ip_proto, info);
    }

    fn parse_arp(arp: &[u8], info: &mut ethernet_packet_info_t) {
        if arp.len()<ARP_PACKET_SIZE {
            return;
        }
        info.arp_sha = mac_addr_t::new(arp[8], arp[9], arp[10], arp[11], arp[12], arp[13]);
        info.arp_tha = mac_addr_t::new(arp[18], arp[19], arp[20], arp[21], arp[22], arp[23]);
        info.src_ip = IpAddr::from([arp[14],arp[15],arp[16],arp[17]]);
        info.dst_ip = IpAddr::from([arp[24],arp[25],arp[26],arp[27]]);
        info.arp_op = ((arp[6] as u16) <<8 )+arp[7] as u16;
    }

    //从原始报文中获取信息
    pub fn from_ethernet_packet(packet: &[u8]) -> Result<Self,errcode::RESULT> {
        let pkt_len = packet.len();
        if packet.len()<ETHERNET_HDR_SIZE {
            return Err(errcode::ERROR_INVALID_MSG);
        }
        let mut pkt_info = Self::new_zero();
        pkt_info.total_len = pkt_len as u16;
        pkt_info.dst_mac = mac_addr_t::from_slice(packet);
        pkt_info.src_mac = mac_addr_t::from_slice(&packet[MAC_ADDR_SIZE..]);
        
        let ether_type: u16 = unsafe { ((*packet.get_unchecked(12) as u16) << 8) + *packet.get_unchecked(13) as u16 };
        let mut cur_ptr: usize = ETHERNET_HDR_SIZE;
        unsafe {
        pkt_info.ether_type = match ether_type {
            EthernetTypes::Ethernet_Vlan => {
                if pkt_len<ETHERNET_HDR_SIZE+4 {
                    return Err(errcode::ERROR_INVALID_MSG);
                }
                pkt_info.vlan_layers = 1;
                pkt_info.vlans[0] = ((*packet.get_unchecked(cur_ptr) as u16) << 8) + *packet.get_unchecked(cur_ptr + 1) as u16;
                cur_ptr += 4;
                ((*packet.get_unchecked(cur_ptr - 2) as u16) << 8) + *packet.get_unchecked(cur_ptr - 1) as u16
            }
            EthernetTypes::Ethernet_SVlan => {
                if pkt_len<ETHERNET_HDR_SIZE+4*2 {
                    return Err(errcode::ERROR_INVALID_MSG);
                }
                pkt_info.vlan_layers = 2;
                pkt_info.vlans[0] = ((*packet.get_unchecked(cur_ptr) as u16) << 8) + *packet.get_unchecked(cur_ptr + 1) as u16;
                pkt_info.vlans[1] = ((*packet.get_unchecked(cur_ptr + 4) as u16) << 8) + *packet.get_unchecked(cur_ptr + 5) as u16;
                cur_ptr += 8;
                ((*packet.get_unchecked(cur_ptr - 2) as u16) << 8) + *packet.get_unchecked(cur_ptr - 1) as u16
            }
            _ => ether_type,
        };
        pkt_info.l3_offset = cur_ptr as u16;
        pkt_info.payload_offset=pkt_info.l3_offset;
        if cur_ptr>=pkt_len {
            return Err(errcode::ERROR_INVALID_MSG);
        }
    }
        match pkt_info.ether_type {
            EthernetTypes::Ethernet_Ipv4 => Self::parse_ipv4(&packet[cur_ptr..], &mut pkt_info),
            EthernetTypes::Ethernet_Ipv6 => Self::parse_ipv6(&packet[cur_ptr..], &mut pkt_info),
            EthernetTypes::Ethernet_ARP => Self::parse_arp(&packet[cur_ptr..], &mut pkt_info),
            _ => (),
        }

        return Ok(pkt_info);
    }

    pub fn get_payload_offset(&self)->Result<u16,errcode::RESULT> {
        if self.payload_offset>0 {
            return Ok(self.payload_offset)
        }
        Err(errcode::ERROR_INVALID_MSG)
    }
    pub fn get_l3_hdr_offset(&self)->Result<u16,errcode::RESULT> {
        return Ok(self.l3_offset)
    }

    pub fn get_l4_hdr_offset(&self)->Result<u16,errcode::RESULT> {
        if self.ether_type==EthernetTypes::Ethernet_Ipv4 || self.ether_type==EthernetTypes::Ethernet_Ipv6 {
            return Ok(self.l4_offset)
        }
        Err(errcode::ERROR_INVALID_MSG)
    }

    pub fn to_string(&self)->String {
        format!("total_len={},ether_type={:#x},src_mac={},dst_mac={},vlan={},
        ip_hdr_len={},src_ip={},dst_ip={},ip_proto={},ip_payload_len={},tp_src={},tp_dst={}",
        self.total_len,self.ether_type,self.src_mac,self.dst_mac,self.vlans[0],
        self.ip_hdr_len,self.src_ip, self.dst_ip,self.ip_proto,self.ip_payload_len,
        self.tp_src,self.tp_dst)
    }

}

impl fmt::Display for ethernet_packet_info_t {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"total_len={},ether_type={:#x},src_mac={},dst_mac={},vlan={},
        ip_hdr_len={},src_ip={},dst_ip={},ip_proto={},ip_payload_len={},tp_src={},tp_dst={}",
        self.total_len,self.ether_type,self.src_mac,self.dst_mac,self.vlans[0],
        self.ip_hdr_len,self.src_ip, self.dst_ip,self.ip_proto,self.ip_payload_len,
        self.tp_src,self.tp_dst)
    }
}
