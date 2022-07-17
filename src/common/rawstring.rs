/*提供C格式兼容的以\0结尾的字符串的处理 */
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
use super::*;
//expect_max_len是指缓冲区的最大长度
pub fn raw_strlen(str: *const u8, expect_max_len: usize) -> usize {
    let mut len: usize = 0;
    unsafe {
        while *((str as usize + len) as *const u8) != 0 && len < expect_max_len {
            len += 1;
        }
    }
    len
}

//src_len是指raw buffer的最大长度，比如一个[u8,64]数组类型转换为*mut u8后，需要指明其最大长度为64，实际的字符串长度以0结尾
pub fn array_to_string(src: *const u8, src_len: usize) -> String {
    if src.is_null() {
        return "".to_string()
    }
    let strlen = raw_strlen(src, src_len);
    let mut buf:Vec<u8>=Vec::with_capacity(strlen+1);
    buf.resize(strlen+1,0);
    unsafe {        
        std::ptr::copy(src, buf.as_mut_ptr(), strlen);
        buf.set_len(strlen);
    }
  

    String::from_utf8_lossy(buf.as_slice()).to_string()
}

pub fn strncpy(dst: *mut u8, len: usize, src: &String) -> *mut u8 {
    let mut max_len = len;
    if len > src.len() {
        max_len = src.len();
    }

    for i in 0..max_len {
        unsafe {
            *((dst as usize + i) as *mut u8) = src.as_bytes()[i];
        }
    }
    return dst;
}

pub fn slice_to_hex_string(v:&[u8])->String {
    let mut s = String::with_capacity(v.len()*5);
    for j in v {
        s+=format!("0x{:02x} ",j).as_str();
    }
    s
}

pub fn unicode_str_to_string(ustr:*const u16)->String {
    let mut raw_len = 0;
    const MAX_STRING_LEN:usize=1024;
    unsafe {
    while *((ustr as usize + raw_len*2) as *const u16)!=0 && raw_len<MAX_STRING_LEN{
        raw_len+=1;
    }
    //println!("utf16 string len={}",raw_len);
    String::from_utf16_lossy(&(*(ustr as *const [u16;MAX_STRING_LEN]))[0..raw_len]).to_string()
}
}

///将一个ansi编码的字符串拷贝到16bit unicode编码的字符串中
pub fn ansi_str_to_unicode(ansi_str:&[u8],unicode:&mut [u16])->errcode::RESULT {
    if unicode.len()<ansi_str.len() {
        return errcode::ERROR_BUFFER_TOO_SMALL
    }

    for i in 0..ansi_str.len() {
        unicode[i]=ansi_str[i] as u16;
    }
    errcode::RESULT_SUCCESS
}