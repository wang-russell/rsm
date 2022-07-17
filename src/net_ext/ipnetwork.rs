#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use std::net::{IpAddr};
use crate::common::errcode;
use super::*;

#[derive(Clone,Debug)]
pub struct IpNetwork {
    ip:IpAddr,
    mask:IpAddr,
    mask_len:u8,
}

impl IpNetwork {
    pub fn new(ip:IpAddr,mask_len:u8)->Result<Self,errcode::RESULT> {
        if (ip.is_ipv4() && mask_len>IPV4_ADDR_LEN as u8*8) || (ip.is_ipv6() && mask_len>IPV6_ADDR_LEN as u8*8) {
            return Err(errcode::ERROR_INVALID_PARAM)
        }
        let mask = match Self::get_ip_mask(ip.is_ipv6(),mask_len) {
            Some(m)=>m,
            None=>return Err(errcode::ERROR_INVALID_PARAM),
        };
        let ipnet= Self{
            ip:ip,
            mask:mask,
            mask_len:mask_len,
        };

        Ok(ipnet)
    }

    ///从IP、掩码构建一个IpNetwork
    pub fn from(ip:IpAddr,mask:IpAddr)->IpNetwork {
        let mask_len = Self::get_ip_mask_len(&mask);
        let ipnet =  Self{
            ip:ip,
            mask:mask,
            mask_len:mask_len,
        };
        return ipnet
    }

    ///get_ip_mask_len，根据一个掩码格式的IP，获取ip addr的mask_len
    pub fn get_ip_mask_len(ip:&IpAddr)->u8 {
        match ip {
            IpAddr::V4(addr)=> {
                let u32_addr = u32::from_be_bytes(addr.octets());
                //println!("ip={},u32={}",ip,u32_addr);
                if u32_addr==0 {
                    return 0;
                }
                let mask:u32=0xFFFFFFFF;
                for i in 0..32 {
                    let prefix = mask << i;
                    if u32_addr == prefix {
                        return 32-i as u8
                    }
                }
                return 0
            },
            IpAddr::V6(addr)=> {
                let u128_addr = u128::from_be_bytes(addr.octets());
                let mask:u128 = !0u128;
                if u128_addr==0 {
                    return 0;
                }
                for i in 0..128 {
                    let prefix = mask << i;
                    if u128_addr == prefix {
                        return 128-i as u8
                    }
                }
                return 0
            }
        }
    }

    pub fn get_ip_mask(isv6:bool,mask_len:u8)->Option<IpAddr> {
        if isv6 {
            if mask_len >128 {
                return None;
            }
            let umask=!0u128 << (128-mask_len);
            return Some(IpAddr::from(umask.to_be_bytes()));
        } else {
            if mask_len >128 {
                return None;
            }
            let umask=!0u32 << (32-mask_len);
            return Some(IpAddr::from(umask.to_be_bytes()));
        }
    }

    ///get_ip_netmask，根据IP地址和掩码长度返回子网号
    pub fn get_ip_subnet(ip:&IpAddr,mask_len:u8)->IpAddr {
        match ip {
            IpAddr::V4(addr)=> {
                if mask_len==0 {
                    return IpAddr::from(Ipv4Addr::from(0));
                }
                let mut u32_addr = u32::from_be_bytes(addr.octets());
                let mask:u32 = 0xFFFFFFFF << ((32-mask_len) as usize);
                u32_addr &= mask;
                let masked_addr = u32_addr.to_be_bytes();
                return IpAddr::from(masked_addr);
            },
            IpAddr::V6(addr)=> {
                if mask_len==0 {
                    return IpAddr::from(Ipv6Addr::from(0));
                }
                let mut u128_addr = u128::from_be_bytes(addr.octets());
                let mask:u128 = !0u128 << ((128-mask_len) as usize);
                u128_addr &= mask;
                let masked_addr = u128_addr.to_be_bytes();
                return IpAddr::from(masked_addr);
            }
        }
        
    }


    pub fn get_ip_prefix(&self)->IpAddr {
        self.mask
    }
    pub fn get_mask_len(&self)->u8 {
        self.mask_len
    }

    pub fn get_ip_addr(&self)->IpAddr {
        self.ip
    }
    ///libpnet兼容
    pub fn ip(&self)->IpAddr {
        self.ip
    }
    pub fn mask(&self)->IpAddr {
        self.mask
    }
    pub fn prefix(&self)->IpAddr {
        self.ip
    }

    pub fn is_ipv4(&self)->bool {
        self.ip.is_ipv4()
    }

    pub fn is_ipv6(&self)->bool {
        self.ip.is_ipv6()
    }

    ///判断一个IP、Mask是否有效
    pub fn is_valid_ipmask(ip:&IpAddr,mask_len:u8)->bool {
        if ip.is_ipv4() && mask_len as usize<=IPV4_ADDR_LEN*8 {
            return true
        }
        if ip.is_ipv6() && mask_len as usize<=IPV6_ADDR_LEN*8 {
            return true
        }
        false
    }
}

impl std::cmp::PartialEq for IpNetwork {
    fn eq(&self,other:&Self)->bool {
        return self.ip.eq(&other.ip) && self.mask_len==other.mask_len
    }

}