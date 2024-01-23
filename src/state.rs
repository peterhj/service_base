use once_cell::sync::{Lazy};

use std::convert::{TryFrom, TryInto};
use std::sync::{Arc};
use std::sync::atomic::{AtomicU8, Ordering as AtomicOrdering};

pub static ONCE_STATE: Lazy<Arc<AtomicU8>> = Lazy::new(|| Arc::new(AtomicU8::new(0)));

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum ServiceState {
  Uninit = 0,
  Listen,
  Hup,
  Halt,
}

impl Default for ServiceState {
  fn default() -> ServiceState {
    ServiceState::Uninit
  }
}

impl TryFrom<u8> for ServiceState {
  type Error = ();

  fn try_from(v: u8) -> Result<ServiceState, ()> {
    Ok(match v {
      0 => ServiceState::Uninit,
      1 => ServiceState::Listen,
      2 => ServiceState::Hup,
      3 => ServiceState::Halt,
      _ => return Err(())
    })
  }
}

impl ServiceState {
  pub fn get() -> ServiceState {
    // TODO: atomic ordering.
    ONCE_STATE.load(AtomicOrdering::Acquire).try_into().unwrap()
  }

  pub fn set(next: ServiceState) -> ServiceState {
    // TODO: atomic ordering.
    ONCE_STATE.swap(next as u8, AtomicOrdering::AcqRel).try_into().unwrap()
  }
}
