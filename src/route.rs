use crate::http::{HttpRequest, HttpResponse};

use constant_time_eq::{constant_time_eq};
pub use http1::Method::{GET, POST, PUT};
use http1::{Method, Url};
use rustc_serialize::base64;
use smol_str::{SmolStr};

use std::collections::{BTreeMap};
use std::convert::{TryFrom, TryInto};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum MatchErr {
  RedirectHttps443,
  Bot,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum RoutePort {
  RedirectHttps443,
  //OnlyHttps443,
  OnlyHttp80,
}

impl From<i32> for RoutePort {
  fn from(bits: i32) -> RoutePort {
    match bits {
      0x1fb => RoutePort::RedirectHttps443,
      //0x1bb => RoutePort::OnlyHttps443,
      0x50  => RoutePort::OnlyHttp80,
      _ => panic!("bug: RoutePort::from: invalid bits={:x}", bits)
    }
  }
}

impl RoutePort {
  pub fn match_(self, port: u16) -> Result<(), MatchErr> {
    match (self, port) {
      (RoutePort::RedirectHttps443, 443) => Ok(()),
      (RoutePort::RedirectHttps443,  80) => Err(MatchErr::RedirectHttps443),
      (RoutePort::OnlyHttp80,        80) => Ok(()),
      _ => Err(MatchErr::Bot)
    }
  }
}

pub enum Val {
  Str(SmolStr),
  U64(u64),
  Base64(Box<[u8]>),
}

impl Val {
  pub fn as_str(&self) -> Option<&str> {
    match self {
      &Val::Str(ref s) => Some(s),
      _ => None
    }
  }

  pub fn as_base64(&self) -> Option<&[u8]> {
    match self {
      &Val::Base64(ref v) => Some(v),
      _ => None
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Type_ {
  Str,
  U64,
  Base64,
}

impl TryFrom<&'static str> for Type_ {
  type Error = ();

  fn try_from(s: &'static str) -> Result<Type_, ()> {
    Ok(match s {
      "s" | "str" => Type_::Str,
      "u" | "u64" => Type_::U64,
      "base64" => Type_::Base64,
      _ => return Err(())
    })
  }
}

impl Type_ {
  pub fn match_(&self, s: &str) -> Result<Val, ()> {
    match *self {
      Type_::Str => {
        Ok(Val::Str(s.into()))
      }
      Type_::U64 => {
        let x = u64::from_str_radix(s, 10).map_err(|_| ())?;
        Ok(Val::U64(x))
      }
      Type_::Base64 => {
        let buf = base64::decode_from_str(s).map_err(|_| ())?;
        Ok(Val::Base64(buf.into()))
      }
    }
  }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pat {
  //Method(Method),
  Lit(&'static str),
  Str(SmolStr),
  U64(u64),
  Base64(Box<[u8]>),
  Sub(&'static str, Type_),
}

impl TryFrom<&'static str> for Pat {
  type Error = ();

  fn try_from(s: &'static str) -> Result<Pat, ()> {
    if s.starts_with("{") && s.ends_with("}") {
      let s_len = s.len();
      let (key, ty) = match s.find(":") {
        None => {
          let key_s = s.get(1 .. s_len - 1).unwrap();
          let ty = Type_::Str;
          (key_s, ty)
        }
        Some(delim_idx) => {
          let key_s = s.get(1 .. delim_idx).unwrap();
          let ty_s = s.get(delim_idx + 1 .. s_len - 1).unwrap();
          let ty = Type_::try_from(ty_s)?;
          (key_s, ty)
        }
      };
      if key.len() <= 0 {
        return Err(());
      }
      return Ok(Pat::Sub(key, ty));
    }
    Ok(Pat::Lit(s))
  }
}

/*impl TryFrom<Method> for Pat {
  type Error = ();

  fn try_from(m: Method) -> Result<Pat, ()> {
    Ok(Pat::Method(m))
  }
}*/

impl TryFrom<SmolStr> for Pat {
  type Error = ();

  fn try_from(s: SmolStr) -> Result<Pat, ()> {
    Ok(Pat::Str(s))
  }
}

impl TryFrom<String> for Pat {
  type Error = ();

  fn try_from(s: String) -> Result<Pat, ()> {
    Ok(Pat::Str(s.into()))
  }
}

impl TryFrom<u64> for Pat {
  type Error = ();

  fn try_from(x: u64) -> Result<Pat, ()> {
    Ok(Pat::U64(x))
  }
}

impl TryFrom<Box<[u8]>> for Pat {
  type Error = ();

  fn try_from(buf: Box<[u8]>) -> Result<Pat, ()> {
    Ok(Pat::Base64(buf))
  }
}

impl TryFrom<Vec<u8>> for Pat {
  type Error = ();

  fn try_from(buf: Vec<u8>) -> Result<Pat, ()> {
    Ok(Pat::Base64(buf.into()))
  }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct RoutePath {
  parts: Box<[Pat]>,
}

impl From<()> for RoutePath {
  fn from(_: ()) -> RoutePath {
    RoutePath{
      parts: Box::new([]),
    }
  }
}

impl<P0> From<P0> for RoutePath
where P0: TryInto<Pat> {
  fn from(p0: P0) -> RoutePath {
    RoutePath{
      parts: vec![p0.try_into().map_err(|_| ()).unwrap()].into(),
    }
  }
}

impl<P0> From<(P0,)> for RoutePath
where P0: TryInto<Pat> {
  fn from(p: (P0,)) -> RoutePath {
    RoutePath{
      parts: vec![p.0.try_into().map_err(|_| ()).unwrap()].into(),
    }
  }
}

impl<P0, P1> From<(P0, P1)> for RoutePath
where P0: TryInto<Pat>,
      P1: TryInto<Pat>,
{
  fn from(p: (P0, P1)) -> RoutePath {
    RoutePath{
      parts: vec![p.0.try_into().map_err(|_| ()).unwrap(),
                  p.1.try_into().map_err(|_| ()).unwrap(),
             ].into(),
    }
  }
}

impl<P0, P1, P2> From<(P0, P1, P2)> for RoutePath
where P0: TryInto<Pat>,
      P1: TryInto<Pat>,
      P2: TryInto<Pat>,
{
  fn from(p: (P0, P1, P2)) -> RoutePath {
    RoutePath{
      parts: vec![p.0.try_into().map_err(|_| ()).unwrap(),
                  p.1.try_into().map_err(|_| ()).unwrap(),
                  p.2.try_into().map_err(|_| ()).unwrap(),
             ].into(),
    }
  }
}

impl<P0, P1, P2, P3> From<(P0, P1, P2, P3)> for RoutePath
where P0: TryInto<Pat>,
      P1: TryInto<Pat>,
      P2: TryInto<Pat>,
      P3: TryInto<Pat>,
{
  fn from(p: (P0, P1, P2, P3)) -> RoutePath {
    RoutePath{
      parts: vec![p.0.try_into().map_err(|_| ()).unwrap(),
                  p.1.try_into().map_err(|_| ()).unwrap(),
                  p.2.try_into().map_err(|_| ()).unwrap(),
                  p.3.try_into().map_err(|_| ()).unwrap(),
             ].into(),
    }
  }
}

impl<P0, P1, P2, P3, P4> From<(P0, P1, P2, P3, P4)> for RoutePath
where P0: TryInto<Pat>,
      P1: TryInto<Pat>,
      P2: TryInto<Pat>,
      P3: TryInto<Pat>,
      P4: TryInto<Pat>,
{
  fn from(p: (P0, P1, P2, P3, P4)) -> RoutePath {
    RoutePath{
      parts: vec![p.0.try_into().map_err(|_| ()).unwrap(),
                  p.1.try_into().map_err(|_| ()).unwrap(),
                  p.2.try_into().map_err(|_| ()).unwrap(),
                  p.3.try_into().map_err(|_| ()).unwrap(),
                  p.4.try_into().map_err(|_| ()).unwrap(),
             ].into(),
    }
  }
}

impl RoutePath {
  pub fn new<P: Into<Box<[Pat]>>>(parts: P) -> RoutePath {
    let parts = parts.into();
    RoutePath{parts}
  }

  pub fn match_<S: AsRef<str>>(&self, parts: &[S], args: &mut RouteArgs) -> Option<()> {
    let rule_len = self.parts.len();
    if rule_len > parts.len() {
      return None;
    }
    if rule_len < parts.len() {
      // TODO: trailing should be none or empty.
      for i in self.parts.len() .. parts.len() {
        let v = parts[i].as_ref();
        if v.len() > 0 {
          return None;
        }
      }
    }
    for i in 0 .. rule_len {
      let v = parts[i].as_ref();
      match &self.parts[i] {
        &Pat::Lit(v0) => {
          if v0 != v {
            return None;
          }
        }
        &Pat::Str(ref v0) => {
          if v0 != v {
            return None;
          }
        }
        &Pat::U64(v0) => {
          let v = match u64::from_str_radix(v, 10) {
            Err(_) => return None,
            Ok(x) => x
          };
          if v0 != v {
            return None;
          }
        }
        &Pat::Base64(ref v0) => {
          let vbuf = match base64::decode_from_str(v) {
            Err(_) => return None,
            Ok(buf) => buf
          };
          if !constant_time_eq(v0, &vbuf) {
            return None;
          }
        }
        &Pat::Sub(k, ty) => {
          match ty.match_(v) {
            Err(_) => {
              return None;
            }
            Ok(v) => {
              args.insert(k, v);
            }
          }
        }
      }
    }
    Some(())
  }
}

pub type RouteArgs = BTreeMap<&'static str, Val>;
pub type RouteFire = Box<dyn 'static + Send + Sync + Fn(&[Pat], &RouteArgs, &HttpRequest) -> Option<HttpResponse>>;

pub struct Router {
  rules: BTreeMap<usize, BTreeMap<(RoutePort, Method, RoutePath), RouteFire>>,
}

impl Router {
  pub fn new() -> Router {
    Router{rules: BTreeMap::new()}
  }

  pub fn insert_get<R: Into<RoutePath>>(&mut self, rule: R, fire: RouteFire) {
    self.insert(443 | 80, GET, rule, fire)
  }

  pub fn insert_post<R: Into<RoutePath>>(&mut self, rule: R, fire: RouteFire) {
    self.insert(443 | 80, POST, rule, fire)
  }

  pub fn insert<P: Into<RoutePort>, R: Into<RoutePath>>(&mut self, port: P, method: Method, rule: R, fire: RouteFire) {
  //pub fn insert<R: Into<RoutePath>, F: 'static + Into<RouteFire>>(&mut self, rule: R, fire: F) {}
    let port = port.into();
    let rule = rule.into();
    let rule_len = rule.parts.len();
    //let fire = fire.into();
    match self.rules.get_mut(&rule_len) {
      None => {
        let mut rules = BTreeMap::new();
        rules.insert((port, method, rule), fire);
        self.rules.insert(rule_len, rules);
      }
      Some(rules) => {
        rules.insert((port, method, rule), fire);
      }
    }
  }

  pub fn remove_get<R: Into<RoutePath>>(&mut self, rule: R) {
    self.remove(443 | 80, GET, rule)
  }

  pub fn remove_post<R: Into<RoutePath>>(&mut self, rule: R) {
    self.remove(443 | 80, POST, rule)
  }

  pub fn remove<P: Into<RoutePort>, R: Into<RoutePath>>(&mut self, port: P, method: Method, rule: R) {
    let port = port.into();
    let rule = rule.into();
    let rule_len = rule.parts.len();
    match self.rules.get_mut(&rule_len) {
      None => {}
      Some(rules) => {
        rules.remove(&(port, method, rule));
      }
    }
  }

  pub fn match_(&self, q_port: u16, req: &HttpRequest) -> Result<Option<HttpResponse>, MatchErr> {
    // TODO TODO: payload.
    // FIXME: optional trailing-'/' stripping.
    let mut q_pathlen = req.path.len();
    for i in (0 .. req.path.len()).rev() {
      let v = &req.path[i];
      if v.len() > 0 {
        break;
      }
      q_pathlen -= 1;
    }
    if let Some(rules) = self.rules.get(&q_pathlen) {
      for (&(port, method, ref rule), fire) in rules.iter() {
        match port.match_(q_port) {
          Ok(_) => {}
          Err(MatchErr::RedirectHttps443) => {
            // FIXME FIXME: whose responsibility to build
            // the redirect response?
            return Err(MatchErr::RedirectHttps443);
          }
          Err(_) => continue
        }
        if method != req.method {
          continue;
        }
        let mut args = BTreeMap::new();
        if rule.match_(&req.path, &mut args).is_some() {
          let rep = (fire)(&rule.parts, &args, req);
          return Ok(rep);
        }
      }
    }
    Ok(None)
  }
}
