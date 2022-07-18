#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
use crate::common::errcode;
use std::net::{self, TcpListener};
use std::thread;
use tiny_http as http;

const _DEBUG: bool = true;

pub const MAX_BUFFER_SIZE: usize = 32768;
pub const DEF_SERVER_PORT: u16 = 10011;
pub type Method = http::Method;

pub enum E_CONTENT_TYPE {
    e_content_text =0,
    e_application_json =1,
}
impl E_CONTENT_TYPE {
    pub fn to_string(&self)->&str{
        match self {
            Self::e_content_text=>"text/plain",
            Self::e_application_json=>"application/json",            
        }
    }
}
pub type RestCallBack =
    fn(method: &Method, path: &str, body: &String) -> Result<(String,E_CONTENT_TYPE), errcode::RESULT>;

pub struct RestServer {
    local_addr: net::SocketAddr,
    server: http::Server,
    call_back: RestCallBack,
    terminated: bool,
}
//从URL中获取路径，去掉头部
pub fn get_url_path(url: &str) -> Result<String, errcode::RESULT> {
    let res = url.find("//");
    match res {
        None => {
            let res1 = url.find("/");
            match res1 {
                None => Err(errcode::ERROR_NOT_FOUND),
                Some(pos) => Ok(String::from(&url[pos..url.len()])),
            }
        }
        Some(pos) => {
            let subs = String::from(&url[pos + 2..url.len()]);
            let res1 = subs.find("/");
            match res1 {
                None => Err(errcode::ERROR_NOT_FOUND),
                Some(pos) => Ok(String::from(&subs[pos..subs.len()])),
            }
        }
    }
}

//map between errcode and HTTP CODE
pub fn error_to_http_code(ec: errcode::RESULT) -> http::StatusCode {
    let code = match ec {
        errcode::RESULT_SUCCESS => errcode::HTTP_SUCCESS,
        errcode::ERROR_DECODE_MSG => errcode::HTTP_BAD_REQUEST,
        errcode::ERROR_NO_PERMISSION => errcode::HTTP_FORBIDDEN,
        errcode::ERROR_COMMON => errcode::HTTP_SERVER_NOT_IMPLEMENT,
        errcode::ERROR_NOT_FOUND => errcode::HTTP_NOT_FOUND,
        errcode::ERROR_NOT_SUPPORT => errcode::HTTP_SERVER_NOT_IMPLEMENT,
        _ => errcode::HTTP_BAD_REQUEST,
    };

    http::StatusCode::from(code)
}

impl RestServer {
    pub fn new(
        ip: net::IpAddr,
        port: u16,
        call_back: RestCallBack,
    ) -> Result<RestServer, errcode::RESULT> {
        let localAddr = net::SocketAddr::new(ip, port);

        let res = TcpListener::bind(localAddr);
        let rs = match res {
            Ok(listener) => http::Server::from_listener(listener, None).unwrap(),
            Err(_e) => return Err(errcode::ERROR_BIND_SOCKET),
        };
        let server = RestServer {
            local_addr: localAddr,
            server: rs,
            call_back: call_back,
            terminated: false,
        };

        return Ok(server);
    }

    pub fn run(self) -> thread::JoinHandle<()> {
        let h = thread::spawn(move || self.event_loop());
        return h;
    }
    pub fn terminate(&mut self) {
        if !self.terminated {
            self.terminated = true;
        }
    }

    fn event_loop(&self) {
        println!("[RestServer]Entering Event Loop");
        for req in self.server.incoming_requests() {
            //println!("recv req={}", req.method().as_str());
            self.handle_request(req);
            if self.terminated {
                break;
            }
        }
    }

    fn handle_request(&self, mut req: http::Request) -> errcode::RESULT {
        //let mut buf=[0u8; MaxBufferSize];
        let mut buf = String::with_capacity(MAX_BUFFER_SIZE);

        let res = req.as_reader().read_to_string(&mut buf);
        let url = req.url();
        //println!("[restserver]recv request,url={},msg={}",url,buf);
        let _len = match res {
            Ok(len) => len,
            Err(_) => {
                self.send_err_resp(req, errcode::ERROR_DECODE_MSG, "Read Buffer Error");
                return errcode::ERROR_DECODE_MSG;
            }
        };
        let path = match get_url_path(&url.to_lowercase()) {
            Ok(s) => s,
            Err(_) => String::default(),
        };
        let ret = (self.call_back)(req.method(), &path, &buf);
        match ret {
            Ok((body,ctype)) => self.send_success_resp(req, ctype, body),
            Err(ec) => self.send_err_resp(req, ec, errcode::errcode_to_string(ec)),
        }
        errcode::RESULT_SUCCESS
    }

    //发送错误响应
    fn send_err_resp(&self, req: http::Request, err: errcode::RESULT, msg: &str) {
        
        let respCode = error_to_http_code(err);
        let new_msg=format!("{}\r\n",msg);
        let mut resp = http::Response::new_empty(respCode)
            .with_data(new_msg.as_bytes(), Some(new_msg.len()))
            .with_header(http::Header::from_bytes(&b"Server"[..], &b"Rest-Server"[..]).unwrap());
        resp.add_header(
            http::Header::from_bytes(&b"content-type"[..], &b"text/plain"[..]).unwrap(),
        );
        //
        //let resp = http::Response::from_string("HTTP/1.1 200 OK\r\n Server: Rest-Rust\r\n");
        match req.respond(resp) {
            Ok(())=>(),
            Err(e)=>println!("Send response error,code={},err={},msg_body={}",err,e,msg),
        }
    }

    fn send_success_resp(&self, req: http::Request, ctype:E_CONTENT_TYPE,body: String) {
        let respCode = http::StatusCode::from(errcode::HTTP_SUCCESS);
        // /let new_body = !format("{}\r\n",body);
        let mut resp = http::Response::new_empty(respCode)
            .with_data(body.as_bytes(), Some(body.len()))        
            .with_header(http::Header::from_bytes(&b"Server"[..], &b"Rest-Server"[..]).unwrap());
        if body.len()> 0 {            
            resp.add_header(
                http::Header::from_bytes(&b"content-type"[..], ctype.to_string().as_bytes()).unwrap(),
            );
        }

        req.respond(resp).unwrap();
    }
}
