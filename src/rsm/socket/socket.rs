use crate::common::errcode;
use crate::net_ext;
use std::net::{IpAddr,SocketAddr};
use super::*;
#[cfg(windows)]
use crate::net_ext::windows::rawsocket;

#[cfg(unix)]
use crate::net_ext::unix::rawsocket;

///Inner Socket ecapsulation implementation, other socket must derive from this implementation
/// Each socket is associated with a socket index, which is maintain outside, e.g. a socket pool instance
///
impl Socket {
    pub fn new_socket(sock_idx:i32,sock_af:SOCKET_ADDRESS_FAMILY,sock_type:SOCKET_TYPE,proto:u8)->Result<Self,errcode::RESULT> {
         let sock = match socket(sock_af.clone(), sock_type.clone(), proto) {
            Ok(s)=>s,
            Err(e)=>return Err(e),
        };

        let mut sock = Socket {
            sock_id:sock_idx,
            os_fd:sock,
            sock_af:sock_af,
            sock_type:sock_type,
            proto:proto,
            state:SOCKET_STATE::SOCK_INIT,
            tcp_server:false,
            lb_policy:SOCKET_LB_POLICY::SOCK_LB_ALL_INSTANCE,
            local_addr:SocketAddr::new(IpAddr::from([0,0,0,0]), 0),
            peer_addr:SocketAddr::new(IpAddr::from([0,0,0,0]), 0),
        };
        sock.set_non_blocking();

        return Ok(sock)
    }

    pub fn bind(&mut self,addr:&SocketAddr)->errcode::RESULT {
        if self.state!=SOCKET_STATE::SOCK_INIT {
            return errcode::ERROR_INVALID_STATE
        }
        let ret = rawsocket::bind(self.get_raw_fd(), addr);
        if ret == errcode::RESULT_SUCCESS {
            self.state=SOCKET_STATE::SOCK_BIND
        }
        return ret
    }

    pub fn listen(&mut self,back_log:i32)->errcode::RESULT{
        if self.state!=SOCKET_STATE::SOCK_BIND {
            return errcode::ERROR_INVALID_STATE
        }
        let ret = rawsocket::listen(self.get_raw_fd(), back_log);
        if ret == errcode::RESULT_SUCCESS {
            self.tcp_server = true;
            self.state=SOCKET_STATE::SOCK_LISTENING
        } else {
            println!("listen socket failed,local={},ret={},os_err={}",self.local_addr,ret,std::io::Error::last_os_error());
        }         

        return ret
    }

    pub fn accept(&mut self,new_idx:i32)->Result<Self,errcode::RESULT> {
        if self.state!=SOCKET_STATE::SOCK_LISTENING {
            return Err(errcode::ERROR_INVALID_STATE)
        }
        let (fd,peer) = match rawsocket::accept(self.get_raw_fd()) {
            Ok((f,p))=>(f,p),
            Err(e)=>return Err(e),
        };
        let mut new_sock = self.clone();
        new_sock.os_fd=fd;
        new_sock.sock_id=new_idx;
        new_sock.tcp_server=false;
        new_sock.peer_addr = peer;
        new_sock.state=SOCKET_STATE::SOCK_CONNECTED;
        new_sock.set_non_blocking();
       Ok(new_sock)
    }

    pub fn connect(&mut self,dst:&SocketAddr)->errcode::RESULT {
        let state = self.state;
        self.state=SOCKET_STATE::SOCK_CONNECTING;
        let ret=rawsocket::connect(self.os_fd, dst);
        if ret==errcode::RESULT_SUCCESS {
            self.state=SOCKET_STATE::SOCK_CONNECTED;
        } else {
            self.state = state;
        }
        return ret;
    }
    pub fn send(&mut self,buf:&[u8])->Result<usize,errcode::RESULT> {
        return rawsocket::write_fd(self.os_fd, buf, 0);
    }

    pub fn send_to(&mut self,dst:&SocketAddr,buf:&[u8])->Result<usize,errcode::RESULT> {
        return rawsocket::send_to(self.os_fd, buf, 0, dst)
    }

    pub fn recv(&mut self,buf:&mut [u8])->Result<usize,errcode::RESULT> {

       return rawsocket::read_fd(self.os_fd, buf, 0)
    }

    pub fn recv_from(&mut self,buf:&mut [u8])->Result<(usize,SocketAddr),errcode::RESULT> {
        let (len,peer) = match rawsocket::recv_from(self.os_fd, buf) {
            Ok((l,a))=>(l,a),
            Err(_)=>return Err(errcode::ERROR_RECV_MSG),
        };

        return Ok((len,peer))
    }

    pub fn get_raw_fd(&self)->net_ext::RawFdType {
        self.os_fd
    }

    pub fn set_send_buffer(&mut self,size:usize)->errcode::RESULT {
        return rawsocket::set_socket_sendbuf(self.os_fd, size as i32)        
    }

    pub fn set_recv_buffer(&mut self,size:usize)->errcode::RESULT {
        return rawsocket::set_socket_recvbuf(self.os_fd, size as i32)
    }

    pub fn set_non_blocking(&mut self)->errcode::RESULT {
        if rawsocket::set_non_blocking(self.os_fd).is_ok() {
            return  errcode::RESULT_SUCCESS
        }

        return errcode::ERROR_OS_CALL_FAILED
       
    }

    pub fn get_socket_id(&self)->i32 {
        self.sock_id
    }

    pub(crate) fn get_sock_state(&self)->SOCKET_STATE {
        self.state
    }

    pub fn is_tcp_server(&self)->bool {
        return self.tcp_server
    }

    pub fn set_lb_policy(&mut self,policy:SOCKET_LB_POLICY)->errcode::RESULT {
        if !self.is_tcp_server() {
            return errcode::ERROR_INVALID_STATE
        }
        self.lb_policy = policy;
        errcode::RESULT_SUCCESS
    }

    pub fn get_lb_policy(&self)->Result<SOCKET_LB_POLICY,errcode::RESULT> {
        if self.is_tcp_server() {
            return Ok(self.lb_policy)
        }
        Err(errcode::ERROR_INVALID_STATE)
    }

    pub fn get_local_addr(&self)->SocketAddr {
        self.local_addr
    }

    pub fn get_peer_addr(&self)->SocketAddr {
        self.peer_addr
    }

    pub fn get_sock_type(&self)->SOCKET_TYPE {
        self.sock_type
    }

}

//close the Socket on drop the memory
impl Drop for Socket {
    fn drop(&mut self) {
        rawsocket::close_fd(self.os_fd)
    }
}