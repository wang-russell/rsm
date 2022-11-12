#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::net::IpAddr;

use rust_rsm::alg::{self,crypto,spf,spf_matrix_t};
use rust_rsm::common;

const MAX_TEST_LEN:usize=510;
const MAX_TEST_ROUND:usize=128*1024+1;
#[test]
fn test_xor_alg() {
    let slice1=[0x41u8;MAX_TEST_LEN];
    let mut slice2=[0x65u8;MAX_TEST_LEN];

    let mut cur = common::get_now_usec64();
    for _ in 0.. MAX_TEST_ROUND {
        alg::slice_xor_simple(&slice1,&mut slice2);
    }
    println!("\nslice xor simple {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&slice2));

     cur = common::get_now_usec64();
    for _ in 0.. MAX_TEST_ROUND {
        alg::slice_xor(&slice1,&mut slice2);
    }
    println!("\nslice xor  {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&slice2));


}

#[test]
fn test_crypto_aes() {
    let slice1=[0x41u8;MAX_TEST_LEN];
    let mut slice2=[0u8;MAX_TEST_LEN+32];
    let mut buffer=[0u8;MAX_TEST_LEN+32];
    let passwd="hello world";

    let enc = crypto::crypto_alg_t::new(crypto::E_ENCRYPT_ALG::enc_alg_aes_cbc_128,passwd.as_bytes()).unwrap();
    let dec = crypto::crypto_alg_t::new(crypto::E_ENCRYPT_ALG::enc_alg_aes_cbc_128,passwd.as_bytes()).unwrap();

    let enc_len = enc.encrypt(&slice1,&mut buffer).unwrap();
    let dec_len = match dec.decrypt(&buffer[0..enc_len],&mut slice2) {
        Err(e)=> {
            println!("decrypt error,e={}",e);
            0
         },
        Ok(l)=>l,
    };

    println!("AES128 origin slice={},\nencrypt slice={},\ndecrypt len={},content={}",
    common::rawstring::slice_to_hex_string(&slice1),
    common::rawstring::slice_to_hex_string(&buffer[0..enc_len]),
    dec_len,common::rawstring::slice_to_hex_string(&slice2[0..dec_len]));

    let mut cur = common::get_now_usec64();
    for _ in 0.. MAX_TEST_ROUND {
        let _ = enc.encrypt(&slice1,&mut buffer);
    }
    println!("\nAES 128 encrypt {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&buffer[0..enc_len]));

     cur = common::get_now_usec64();
    for _ in 0.. MAX_TEST_ROUND {
        let _ = dec.decrypt(&buffer[0..enc_len],&mut slice2);
    }
    println!("\nAES128 decrypt  {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&slice2[0..dec_len]));


}


#[test]
fn test_crypto_sm4() {
    let slice1=[0x41u8;MAX_TEST_LEN];
    let mut slice2=[0u8;MAX_TEST_LEN+32];
    let mut buffer=[0u8;MAX_TEST_LEN+32];
    let passwd="hello world";

    let enc = crypto::crypto_alg_t::new(crypto::E_ENCRYPT_ALG::enc_alg_sm4,passwd.as_bytes()).unwrap();
    let dec = crypto::crypto_alg_t::new(crypto::E_ENCRYPT_ALG::enc_alg_sm4,passwd.as_bytes()).unwrap();

    let enc_len = enc.encrypt(&slice1,&mut buffer).unwrap();
    let dec_len = match dec.decrypt(&buffer[0..enc_len],&mut slice2) {
        Err(e)=> {
            println!("decrypt error,e={}",e);
            0
         },
        Ok(l)=>l,
    };

    println!("SM4 origin slice={},\nencrypt slice={},\ndecrypt len={},content={}",
    common::rawstring::slice_to_hex_string(&slice1),
    common::rawstring::slice_to_hex_string(&buffer[0..enc_len]),
    dec_len,common::rawstring::slice_to_hex_string(&slice2[0..dec_len]));

    let mut cur = common::get_now_usec64();
    for _ in 0.. MAX_TEST_ROUND {
        let _ = enc.encrypt(&slice1,&mut buffer);
    }
    println!("\nSM4 encrypt {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&buffer[0..enc_len]));

     cur = common::get_now_usec64();
    for _ in 0.. MAX_TEST_ROUND {
        let _ = dec.decrypt(&buffer[0..enc_len],&mut slice2);
    }
    println!("\nSM4 decrypt  {} round, spend {} us,v={}",
    MAX_TEST_ROUND,common::get_now_usec64()-cur,common::rawstring::slice_to_hex_string(&slice2[0..dec_len]));


}

struct edge_test {
    src_node:u32,
    src_port:u32,
    dst_node:u32,
    dst_port:u32,
    metric:u16,
}

#[test]
fn test_spf_alg() {
    
    let nodes=vec![100u32,200u32,300u32,400u32,500,600,700];
    let edges=vec![edge_test{src_node:100,src_port:2,dst_node:200, dst_port:1,metric:10},
        edge_test{src_node:100,src_port:3,dst_node:300, dst_port:1,metric:25},
        edge_test{src_node:200,src_port:3,dst_node:300, dst_port:2,metric:10},
        edge_test{src_node:300,src_port:4,dst_node:400, dst_port:3,metric:40},
        edge_test{src_node:300,src_port:4,dst_node:600, dst_port:3,metric:50},
        edge_test{src_node:400,src_port:4,dst_node:500, dst_port:3,metric:35},
        edge_test{src_node:400,src_port:4,dst_node:600, dst_port:3,metric:35},
        edge_test{src_node:500,src_port:4,dst_node:700, dst_port:3,metric:12},
        edge_test{src_node:600,src_port:4,dst_node:700, dst_port:3,metric:46}];
    let mut spfm=spf_matrix_t::new();

    for n in nodes.iter() {
        spfm.add_node(n.clone());
    }
    println!("added {} node",nodes.len());

    for e in edges.iter() {
        spfm.AddEdge(e.src_node, e.src_port, e.dst_node, e.dst_port, e.metric);
        println!("add edge,{}.{}-{}.{},metric={}",e.src_node, e.src_port, e.dst_node, e.dst_port, e.metric);
    }

    println!("Begin calculate shortest path");
    let start=common::get_now_usec64();
    spfm.calc_all_path();
    let dur_usec=common::get_now_usec64()-start;
    println!("calculate spf spend {} us",dur_usec);
    assert!(dur_usec<1000*1000);

    let p=spfm.get_spf_path(100, 700).unwrap();
    spf::print_path(&p);
    assert_eq!(p.len(),5);  
    

}

#[test]
fn test_spf_perf() {
    const MAX_TEST_NODE:usize=512;
    let mut cost_m=[[65535u16;MAX_TEST_NODE];MAX_TEST_NODE];
    let mut edge_cnt=0;
    for i in 0..MAX_TEST_NODE {
        for j in 0..MAX_TEST_NODE {
            if i==j {
                cost_m[i][j]=0;
            } else {
                if i==j+2 || i+2==j {
                    cost_m[i][j]=(i+j) as u16;
                    edge_cnt+=1;
                }
            }
        }
    }
    cost_m[0][50]=100;
    cost_m[50][126]=128;
    if MAX_TEST_NODE==512 {
        cost_m[126][511]=511;
    }
    let mut spfm=spf_matrix_t::new();

    for n in 1..MAX_TEST_NODE+1 {
        spfm.add_node(n as u32);
    }

    for i in 0..MAX_TEST_NODE {
        for j in 0..MAX_TEST_NODE {
            let metric = cost_m[i][j];
            if metric>0 && metric<spf::METRIC_INFINITY {
                spfm.AddEdge((i+1) as u32, 1, (j+1) as u32, 1, metric);
            }
        }
    }


    println!("Begin test shortest path algorithm performance");
    let start=common::get_now_usec64();
    spfm.calc_all_path();
    let dur_usec=common::get_now_usec64()-start;
    println!("calculate spf for {} nodes {} edges,spend {} us",MAX_TEST_NODE, edge_cnt,dur_usec);

    let src_node=1;
    for dst in MAX_TEST_NODE-10..MAX_TEST_NODE+1 {
        let p=match spfm.get_spf_path(src_node, dst as u32) {
            None=>continue,
            Some(v)=>v,
        };
        spf::print_path(&p);
    }

    

}


#[test]
fn test_merge_slice() {

    const MAX_TEST_LOOP:usize=100;
    const addr1:u32=0x0a010101;
    const addr2:u32=0x0c010101;
    const addr_num:u32=500;
    let iv1=generate_ip(addr1, addr_num);
    let iv2=generate_ip(addr2, addr_num);

    let start=common::get_now_usec64();
    for _ in 0..MAX_TEST_LOOP {
        let mut out=iv1.clone();
        let inv = iv2.clone();
        alg::merge_slice(&mut out, &inv);
        if out.len()<(addr_num*2) as usize {
            println!("{:?}",out);
        }

    }

    println!("Merge slice {} round, addr_num={},spend {} us",MAX_TEST_LOOP,addr_num,common::get_now_usec64()-start);

    
    let start=common::get_now_usec64();
    for _ in 0..MAX_TEST_LOOP {
        let mut out=iv1.clone();
        let inv = iv2.clone();
        alg::merge_slice2(&mut out, &inv);
        if out.len()<(addr_num*2) as usize {
            println!("{:?}",out);
        }

    }

    println!("Merge slice2 {} round, addr_num={},spend {} us",MAX_TEST_LOOP,addr_num,common::get_now_usec64()-start);

}

fn generate_ip(start_addr:u32,num:u32)->Vec<IpAddr> {
    let mut dst:Vec<IpAddr>=Vec::new();
    for j in start_addr..start_addr+num {
        let aj=j.to_ne_bytes();
        let a = IpAddr::from(aj);
        dst.push(a);
    }    

    dst
}

