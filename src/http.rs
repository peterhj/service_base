pub use http1::{Mime, Charset as HttpCharset, Encoding as HttpEncoding, Status as HttpStatus};
use rustc_serialize::base64;
use rustc_serialize::json::{Json};
use smol_str::{SmolStr};

use std::collections::{BTreeMap};
use std::convert::{TryFrom};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::str::{from_utf8};

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub enum HttpPayload {
  Utf8(Option<Mime>, Option<HttpEncoding>, String),
  Bin(Option<Mime>, Option<HttpCharset>, Option<HttpEncoding>, Box<[u8]>),
}

impl Debug for HttpPayload {
  fn fmt(&self, f: &mut Formatter) -> FmtResult {
    match self {
      &HttpPayload::Utf8(m, e, ..) => {
        write!(f, "HttpPayload::Utf8({:?}, {:?}, ...)", m, e)
      }
      &HttpPayload::Bin(m, c, e, ..) => {
        write!(f, "HttpPayload::Bin({:?}, {:?}, {:?}, ...)", m, c, e)
      }
    }
  }
}

impl HttpPayload {
  pub fn as_raw_bytes(&self) -> &[u8] {
    match self {
      &HttpPayload::Utf8(.., ref s) => {
        s.as_bytes()
      }
      &HttpPayload::Bin(.., ref buf) => {
        buf
      }
    }
  }
}

#[derive(Clone, RustcDecodable, RustcEncodable, Debug)]
pub struct HttpRequest {
  pub method: http1::Method,
  //pub host: Option<SmolStr>,
  pub path: Vec<SmolStr>,
  pub params: BTreeMap<SmolStr, SmolStr>,
  pub auth: Option<Box<[u8]>>,
  pub cookies: BTreeMap<SmolStr, SmolStr>,
  //pub headers: Vec<http1::Header>,
  pub payload: Option<HttpPayload>,
}

impl TryFrom<&Json> for HttpRequest {
  type Error = ();

  fn try_from(j: &Json) -> Result<HttpRequest, ()> {
    // FIXME
    unimplemented!();
    //HttpRequest::try_from_json(j)
  }
}

impl HttpRequest {
  /*pub fn try_from_json(j: &Json) -> Result<HttpRequest, ()> {
    match j {
      &Json::Array(ref j) => {
        if j.len() < 2 || j.len() > 4 {
          return Err(());
        }
        match &j[0] {
          &Json::String(ref s) => {
            let method: http1::Method = s.parse()?;
            let url = match &j[1] {
              &Json::String(ref s) => {
                http1::Url::parse(s).map_err(|_| ())?
              }
              _ => return Err(())
            };
            let headers = Vec::new();
            let auth = None;
            let cookies = BTreeMap::new();
            if j.len() > 2 {
              match &j[2] {
                &Json::Array(ref j) => {
                  for jitem in j.iter() {
                    match jitem {
                      &Json::Array(ref jitem) => {
                        if jitem.len() < 1 || jitem.len() > 2 {
                          return Err(());
                        }
                        // FIXME FIXME: headers.
                      }
                      _ => return Err(())
                    }
                  }
                }
                _ => return Err(())
              }
            }
            let payload = if j.len() > 3 {
              Some(match &j[3] {
                &Json::String(ref s) => {
                  // FIXME: mime type.
                  HttpPayload::Utf8(s.to_string(), None)
                }
                &Json::Object(ref j) => {
                  // FIXME: base64 binary.
                  unimplemented!();
                }
                _ => return Err(())
              })
            } else {
              None
            };
            return Ok(HttpRequest{
              method,
              url,
              headers,
              auth,
              cookies,
              payload,
            });
          }
          _ => return Err(())
        }
      }
      _ => return Err(())
    }
  }*/

  pub fn try_from_raw_strip_headers(req: http1::Request) -> Result<(HttpRequest, Vec<http1::Header>), ()> {
    let method = req.method.ok_or(())?;
    let url = req.url.ok_or(())?;
    let mut path: Vec<_> =
        match url.path_segments() {
          None => return Err(()),
          Some(path) => path.map(|s| s.into()).collect()
        };
    let mut params = BTreeMap::new();
    for (k, v) in url.query_pairs() {
      params.insert(k.into(), v.into());
    }
    let mut mime: Option<Mime> = None;
    let mut charset: Option<http1::Charset> = None;
    // FIXME: encoding.
    let mut auth = None;
    let mut cookies_once = false;
    let mut cookies = BTreeMap::new();
    let mut headers = Vec::new();
    for h in req.headers.into_iter() {
      match (h.name, h.value) {
        (Ok(http1::HeaderName::ContentType), _) => {
          // FIXME: parse mime type.
        }
        /*(Ok(http1::HeaderName::ContentEncoding), _) => {
          // FIXME
        }*/
        (Ok(http1::HeaderName::Authorization), Err(vbuf)) => {
          let s = match from_utf8(&vbuf) {
            Err(_) => continue,
            Ok(s) => s
          };
          let x = match base64::decode_from_str(s) {
            Err(_) => continue,
            Ok(x) => x.into()
          };
          auth = Some(x);
        }
        (Ok(http1::HeaderName::Authorization), _) => {}
        (Ok(http1::HeaderName::Cookie), Ok(http1::HeaderValue::Cookies(kvs))) => {
          if cookies_once {
            return Err(());
          }
          cookies_once = true;
          for (k, v) in kvs.into_iter() {
            cookies.insert(k, v);
          }
        }
        (name, value) => {
          headers.push(http1::Header{name, value});
        }
      }
    }
    let payload = match (req.payload, charset) {
      (None, _) => None,
      (Some(buf), Some(http1::Charset::Utf8)) => {
        Some(HttpPayload::Utf8(mime, None, String::from_utf8(buf.to_vec()).map_err(|_| ())?))
      }
      (Some(buf), _) => {
        Some(HttpPayload::Bin(mime, charset, None, buf))
      }
    };
    Ok((HttpRequest{
      method,
      path,
      params,
      payload,
      auth,
      cookies,
      //headers: Vec::new(),
    }, headers))
  }
}

#[derive(Clone, RustcDecodable, RustcEncodable, Debug)]
pub struct HttpResponse {
  pub status: HttpStatus,
  pub cookies: BTreeMap<SmolStr, SmolStr>,
  //pub headers: Vec<http1::Header>,
  pub payload: Option<HttpPayload>,
}

/*impl TryFrom<&Json> for HttpResponse {
  type Error = ();

  fn try_from(j: &Json) -> Result<HttpResponse, ()> {
    match j {
      &Json::Array(ref j) => {
        if j.len() < 2 || j.len() > 3 {
          return Err(());
        }
        let status = match &j[0] {
          &Json::I64(x) => {
            HttpStatus::try_from(x as u16)?
          }
          &Json::U64(x) => {
            HttpStatus::try_from(x as u16)?
          }
          _ => return Err(())
        };
        let headers = Vec::new();
        let cookies = BTreeMap::new();
        if j.len() > 1 {
          // TODO TODO
        }
        let payload = if j.len() > 2 {
          Some(match &j[2] {
            &Json::String(ref s) => {
              // FIXME: mime type.
              HttpPayload::Utf8(None, s.to_string())
            }
            &Json::Object(ref j) => {
              // FIXME: base64 binary.
              unimplemented!();
            }
            _ => return Err(())
          })
        } else {
          None
        };
        return Ok(HttpResponse{
          status,
          headers,
          cookies,
          payload,
        });
      }
      _ => return Err(())
    }
  }
}*/

pub fn ok() -> HttpResponse {
  HttpResponse::ok()
}

pub fn created() -> HttpResponse {
  HttpResponse::created()
}

impl HttpResponse {
  pub fn ok() -> HttpResponse {
    HttpResponse::from_status(HttpStatus::OK)
  }

  pub fn created() -> HttpResponse {
    HttpResponse::from_status(HttpStatus::Created)
  }

  pub fn not_found() -> HttpResponse {
    HttpResponse::from_status(HttpStatus::NotFound)
  }

  pub fn from_status(status: HttpStatus) -> HttpResponse {
    HttpResponse{
      status,
      cookies: BTreeMap::new(),
      //headers: Vec::new(),
      payload: None,
    }
  }

  pub fn to_raw(&self) -> http1::Response {
    let mut rep = http1::Response::default();
    rep.version = Some(http1::Version::HTTP_1_1);
    rep.status = self.status.into();
    //rep.headers = self.headers.clone();
    match self.payload.as_ref() {
      None => {}
      Some(&HttpPayload::Utf8(mime, _encoding, ref s)) => {
        //println!("DEBUG:  service_base: HttpResponse::to_raw: mime={:?}", mime);
        let buf: Box<[u8]> = s.clone().into_bytes().into();
        rep.push_header(http1::HeaderName::ContentLength, format!("{}", buf.len()));
        if let Some(m) = mime.and_then(|m| m.to_str()) {
          //println!("DEBUG:  service_base: HttpResponse::to_raw:   to str={:?}", m);
          rep.push_header(http1::HeaderName::ContentType, format!("{}; charset=utf-8", m));
        }
        rep.payload = Some(buf);
      }
      Some(&HttpPayload::Bin(mime, charset, encoding, ref buf)) => {
        let buf = buf.clone();
        rep.push_header(http1::HeaderName::ContentLength, format!("{}", buf.len()));
        if let Some(m) = mime.and_then(|m| m.to_str()) {
          if let Some(c) = charset.and_then(|c| c.to_str()) {
            rep.push_header(http1::HeaderName::ContentType, format!("{}; charset={}", m, c));
          } else {
            rep.push_header(http1::HeaderName::ContentType, m);
          }
        }
        if let Some(e) = encoding.and_then(|e| e.to_str()) {
          rep.push_header(http1::HeaderName::ContentEncoding, e);
        }
        rep.payload = Some(buf);
      }
    }
    rep
  }

  #[inline]
  pub fn with_payload_str<S: Into<String>>(mut self, s: S) -> HttpResponse {
    self.set_payload_str(s);
    self
  }

  #[inline]
  pub fn set_payload_str<S: Into<String>>(&mut self, s: S) {
    self.set_payload_utf8(s, None, None)
  }

  #[inline]
  pub fn with_payload_str_mime<S: Into<String>>(mut self, s: S, m: Mime) -> HttpResponse {
    self.set_payload_str_mime(s, m);
    self
  }

  #[inline]
  pub fn set_payload_str_mime<S: Into<String>>(&mut self, s: S, m: Mime) {
    self.set_payload_utf8(s, m, None)
  }

  #[inline]
  pub fn with_payload_str_mime_encoding<S: Into<String>>(mut self, s: S, m: Mime, e: HttpEncoding) -> HttpResponse {
    self.set_payload_str_mime_encoding(s, m, e);
    self
  }

  #[inline]
  pub fn set_payload_str_mime_encoding<S: Into<String>>(&mut self, s: S, m: Mime, e: HttpEncoding) {
    self.set_payload_utf8(s, m, e)
  }

  #[inline]
  pub fn with_payload_utf8<S: Into<String>, M: Into<Option<Mime>>, E: Into<Option<HttpEncoding>>>(mut self, s: S, m: M, e: E) -> HttpResponse {
    self.set_payload_utf8(s, m, e);
    self
  }

  #[inline]
  pub fn set_payload_utf8<S: Into<String>, M: Into<Option<Mime>>, E: Into<Option<HttpEncoding>>>(&mut self, s: S, m: M, e: E) {
    self.payload = Some(HttpPayload::Utf8(m.into(), e.into(), s.into()));
  }

  /*#[inline]
  pub fn with_payload_bytes<B: Into<Box<[u8]>>>(mut self, buf: B) -> HttpResponse {
    self.set_payload_bytes(buf);
    self
  }

  #[inline]
  pub fn set_payload_bytes<B: Into<Box<[u8]>>>(&mut self, buf: B) {
    self.set_payload_bin(buf, None)
  }*/

  /*#[inline]
  pub fn with_payload_bin<B: Into<Box<[u8]>>, E: Into<Option<HttpEncoding>>>(mut self, buf: B, e: E) -> HttpResponse {
    self.set_payload_bin(buf, e);
    self
  }

  #[inline]
  pub fn set_payload_bin<B: Into<Box<[u8]>>, E: Into<Option<HttpEncoding>>>(&mut self, buf: B, e: E) {
    self.payload = Some(HttpPayload::Bin(Mime::ApplicationOctetStream.into(), e.into(), buf.into()));
  }*/

  #[inline]
  pub fn with_payload_bin<B: Into<Box<[u8]>>, M: Into<Option<Mime>>, C: Into<Option<HttpCharset>>, E: Into<Option<HttpEncoding>>>(mut self, buf: B, m: M, c: C, e: E) -> HttpResponse {
    self.set_payload_bin(buf, m, c, e);
    self
  }

  #[inline]
  pub fn set_payload_bin<B: Into<Box<[u8]>>, M: Into<Option<Mime>>, C: Into<Option<HttpCharset>>, E: Into<Option<HttpEncoding>>>(&mut self, buf: B, m: M, c: C, e: E) {
    self.payload = Some(HttpPayload::Bin(m.into(), c.into(), e.into(), buf.into()));
  }
}
