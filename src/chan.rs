use crate::msg::*;

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian as LE};
use rustc_serialize::{Encodable};
use rustc_serialize::json::{Json, JsonEncoder};

use std::fmt::{Write as FmtWrite};
use std::io::{Read, Write, BufReader, BufWriter, Cursor};
use std::marker::{PhantomData};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc};
use std::thread::{spawn};
use std::time::{Duration as StdDuration};

#[derive(Debug)]
#[non_exhaustive]
pub enum SendErr {
  // TODO
  Top,
  IO,
  Seq,
  Overflow,
  JsonWrite,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum RecvErr {
  // TODO
  Top,
  IO,
  Seq,
  Trailing,
  JsonBuild,
  JsonDecode,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum QueryErr {
  Seq,
  Send(SendErr),
  Recv(RecvErr),
}

impl From<SendErr> for QueryErr {
  fn from(e: SendErr) -> QueryErr {
    QueryErr::Send(e)
  }
}

impl From<RecvErr> for QueryErr {
  fn from(e: RecvErr) -> QueryErr {
    QueryErr::Recv(e)
  }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ReplyErr {
  Seq,
  Recv(RecvErr),
  Send(SendErr),
}

impl From<SendErr> for ReplyErr {
  fn from(e: SendErr) -> ReplyErr {
    ReplyErr::Send(e)
  }
}

impl From<RecvErr> for ReplyErr {
  fn from(e: RecvErr) -> ReplyErr {
    ReplyErr::Recv(e)
  }
}

pub struct Chan<MsgX=()> {
  rx:   BufReader<TcpStream>,
  tx:   BufWriter<TcpStream>,
  rseq: u64,
  tseq: u64,
  // FIXME: should be able to reuse the buffers.
  rbuf: Vec<u8>,
  //tbuf: Vec<u8>,
  tbuf: String,
  _mrk: PhantomData<fn (MsgX) -> MsgX>,
}

impl<MsgX> Chan<MsgX> {
  pub fn new(stream: TcpStream) -> Chan<MsgX> {
    //stream.set_read_timeout(Some(StdDuration::from_secs(2))).unwrap();
    //stream.set_write_timeout(Some(StdDuration::from_secs(2))).unwrap();
    let rx_stm = stream.try_clone().unwrap();
    let rx = BufReader::with_capacity(0x10000, rx_stm);
    let tx_stm = stream;
    let tx = BufWriter::with_capacity(0x10000, tx_stm);
    let rseq = 0;
    let tseq = 0;
    let rbuf = Vec::new();
    //let tbuf = Vec::new();
    let tbuf = String::new();
    Chan{rx, tx, rseq, tseq, rbuf, tbuf, _mrk: PhantomData}
  }
}

impl<MsgX: MsgCodex> Chan<MsgX> {
  pub fn send(&mut self, item: &Msg<MsgX>) -> Result<u64, SendErr> {
    let tseq = self.tseq + 1;
    self.tseq = tseq;
    self.tbuf.clear();
    let tag = match &item {
      &Msg::Top => *b"...",
      &Msg::HUP => *b"HUP",
      &Msg::OKQ => *b"OK?",
      &Msg::OKR => *b"OK.",
      /*&Msg::NTP(ref j) => {
        write!(&mut self.tbuf, "{}", j)
          .map_err(|_| SendErr::JsonWrite)?;
        *b"NTP"
      }*/
      &Msg::H1Q(ref req) => {
        let mut enc = JsonEncoder::new(&mut self.tbuf);
        req.encode(&mut enc)
          .map_err(|_| SendErr::JsonWrite)?;
        *b"H1?"
      }
      &Msg::H1P(ref rep) => {
        let mut enc = JsonEncoder::new(&mut self.tbuf);
        rep.encode(&mut enc)
          .map_err(|_| SendErr::JsonWrite)?;
        *b"H1."
      }
      &Msg::JSO(ref j) => {
        write!(&mut self.tbuf, "{}", j)
          .map_err(|_| SendErr::JsonWrite)?;
        *b"JSO"
      }
      &Msg::Ext(ref x) => {
        match x.encode_wire(&mut self.tbuf) {
          Err(_) => {
            return Err(SendErr::Top);
          }
          Ok(tag) => tag
        }
      }
      &Msg::Bot => *b"!!!",
      _ => return Err(SendErr::Top)
    };
    let mask_bytes = [tag[0], tag[1], tag[2], 0];
    let mask = u32::from_le_bytes(mask_bytes);
    // FIXME: configure max message size.
    if self.tbuf.len() >= 16 * (u16::max_value() as usize) {
      return Err(SendErr::Overflow);
    }
    let len = self.tbuf.len() as u32;
    self.tx.write_u64::<LE>(tseq).map_err(|_| SendErr::IO)?;
    self.tx.write_u32::<LE>(mask).map_err(|_| SendErr::IO)?;
    self.tx.write_u32::<LE>(len).map_err(|_| SendErr::IO)?;
    self.tx.write_all(self.tbuf.as_bytes()).map_err(|_| SendErr::IO)?;
    self.tx.flush().map_err(|_| SendErr::IO)?;
    Ok(tseq)
  }

  pub fn recv(&mut self) -> Result<(Msg<MsgX>, u64), RecvErr> {
    let rseq = self.rx.read_u64::<LE>().map_err(|_| RecvErr::IO)?;
    let mask = self.rx.read_u32::<LE>().map_err(|_| RecvErr::IO)?;
    let mask_bytes = mask.to_le_bytes();
    let tag = [mask_bytes[0], mask_bytes[1], mask_bytes[2]];
    let len = self.rx.read_u32::<LE>().map_err(|_| RecvErr::IO)? as usize;
    self.rbuf.clear();
    self.rbuf.resize(len, 0);
    self.rx.read_exact(&mut self.rbuf).map_err(|_| RecvErr::IO)?;
    if self.rseq >= rseq {
      return Err(RecvErr::Seq);
    }
    self.rseq = rseq;
    let msg = match &tag {
      b"..." => {
        if len > 0 {
          return Err(RecvErr::Trailing);
        }
        Msg::Top
      }
      b"HUP" => {
        if len > 0 {
          return Err(RecvErr::Trailing);
        }
        Msg::HUP
      }
      b"OK?" => {
        if len > 0 {
          return Err(RecvErr::Trailing);
        }
        Msg::OKQ
      }
      b"OK." => {
        if len > 0 {
          return Err(RecvErr::Trailing);
        }
        Msg::OKR
      }
      /*b"NTP" => {
        let j = Json::from_reader(Cursor::new(&self.rbuf))
          .map_err(|_| RecvErr::JsonBuild)?;
        // TODO TODO
        Msg::NTP(j)
      }*/
      b"H1?" => {
        let j = Json::from_reader(Cursor::new(&self.rbuf))
          .map_err(|_| RecvErr::JsonBuild)?;
        let req = j.decode_into()
          .map_err(|_| RecvErr::JsonDecode)?;
        Msg::H1Q(req)
      }
      b"H1." => {
        let j = Json::from_reader(Cursor::new(&self.rbuf))
          .map_err(|_| RecvErr::JsonBuild)?;
        let rep = j.decode_into()
          .map_err(|_| RecvErr::JsonDecode)?;
        Msg::H1P(rep)
      }
      b"JSO" => {
        let j = Json::from_reader(Cursor::new(&self.rbuf))
          .map_err(|_| RecvErr::JsonBuild)?;
        // TODO TODO
        Msg::JSO(j)
      }
      b"!!!" => {
        // TODO: allow an error message.
        /*if len > 0 {
          return Err(RecvErr::Trailing);
        }*/
        Msg::Bot
      }
      _ => {
        match MsgX::decode_wire(tag, &self.rbuf) {
          Err(_) => {
            return Err(RecvErr::Top);
          }
          Ok(x) => Msg::Ext(x)
        }
      }
    };
    Ok((msg, rseq))
  }

  pub fn query(&mut self, query: &Msg<MsgX>) -> Result<Msg<MsgX>, QueryErr> {
    let tseq = self.send(query)?;
    let (reply, rseq) = self.recv()?;
    if tseq != rseq {
      return Err(QueryErr::Seq);
    }
    Ok(reply)
  }

  /*pub fn query_timeout(&mut self, query: &Msg<MsgX>, timeout: StdDuration) -> Result<Msg<MsgX>, QueryErr> {
    let tseq = self.send(query)?;
    let (reply, rseq) = self.recv_timeout(timeout)?;
    if tseq != rseq {
      return Err(QueryErr::Seq);
    }
    Ok(reply)
  }*/

  pub fn reply<P: Fn(&Msg<MsgX>) -> Msg<MsgX>>(&mut self, proc_: P) -> Result<bool, ReplyErr> {
    let (query, rseq) = self.recv()?;
    let reply: Msg<MsgX> = (proc_)(&query);
    let tseq = self.send(&reply)?;
    if rseq != tseq {
      return Err(ReplyErr::Seq);
    }
    Ok(false)
  }

  pub fn replying<P: Fn(&Msg<MsgX>) -> Msg<MsgX>>(&mut self, proc_: P) {
    // FIXME: pre, post callbacks.
    loop {
      match self.reply(&proc_) {
        Err(_) => {
          break;
        }
        Ok(halt) => if halt {
          break;
        }
      }
    }
  }
}

pub struct SpawnPool<MsgX=()> {
  bind: TcpListener,
  _mrk: PhantomData<fn (MsgX) -> MsgX>,
}

impl<MsgX> SpawnPool<MsgX> {
  pub fn new(bind: TcpListener) -> SpawnPool<MsgX> {
    SpawnPool{bind, _mrk: PhantomData}
  }
}

impl<MsgX: 'static + MsgCodex> SpawnPool<MsgX> {
  pub fn replying(&self, proc_: Arc<dyn 'static + Send + Sync + Fn(&Msg<MsgX>) -> Msg<MsgX>>) {
    loop {
      match self.bind.accept() {
        Err(_) => {
        }
        Ok((stream, _addr)) => {
          let proc_ = proc_.clone();
          let _ = spawn(move || {
            let mut chan = Chan::<MsgX>::new(stream);
            chan.replying(&*proc_);
          });
        }
      }
    }
  }
}
