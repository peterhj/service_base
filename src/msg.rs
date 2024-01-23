use crate::http::*;

use rustc_serialize::json::{Json};

use std::convert::{TryFrom};

#[derive(Debug)]
#[non_exhaustive]
pub enum Msg<X=()> {
  Top,
  HUP,
  OKQ,
  OKR,
  // TODO: control msg variant.
  //XC(_),
  // Protocol version/metadata variant.
  //PV(Json),
  // TODO: network time.
  //NTP(Json),
  // TODO TODO
  //H1(Json),
  H1Q(HttpRequest),
  H1P(HttpResponse),
  // Generic json rpc variant.
  JSO(Json),
  Ext(X),
  Bot,
}

pub trait MsgCodex {
  fn encode_wire(&self, buf: &mut String) -> Result<[u8; 3], ()>;
  fn decode_wire(tag: [u8; 3], buf: &[u8]) -> Result<Self, ()> where Self: Sized;
}

impl MsgCodex for () {
  fn encode_wire(&self, buf: &mut String) -> Result<[u8; 3], ()> {
    Err(())
  }

  fn decode_wire(tag: [u8; 3], buf: &[u8]) -> Result<(), ()> {
    Err(())
  }
}
