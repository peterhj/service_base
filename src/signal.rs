use once_cell::sync::{Lazy};
use signal_hook::consts::{SIGWINCH, SIGCONT, SIGHUP, SIGINT, SIGTERM, SIGQUIT};
use signal_hook::flag::{register as register_signal};

use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

pub static ONCE_SIGNALS: Lazy<SignalsState> = Lazy::new(|| SignalsState::new());

pub fn signals() -> &'static SignalsState {
  &*ONCE_SIGNALS
}

pub struct SignalsState {
  pub winch: Arc<AtomicBool>,
  pub cont: Arc<AtomicBool>,
  pub hup:  Arc<AtomicBool>,
  pub int_: Arc<AtomicBool>,
  pub term: Arc<AtomicBool>,
  pub quit: Arc<AtomicBool>,
}

impl SignalsState {
  pub fn new() -> SignalsState {
    SignalsState{
      winch: Arc::new(AtomicBool::new(false)),
      cont: Arc::new(AtomicBool::new(false)),
      hup:  Arc::new(AtomicBool::new(false)),
      int_: Arc::new(AtomicBool::new(false)),
      term: Arc::new(AtomicBool::new(false)),
      quit: Arc::new(AtomicBool::new(false)),
    }
  }

  pub fn get_winch(&self) -> bool {
    self.winch.load(AtomicOrdering::Relaxed)
  }

  pub fn unset_winch(&self) {
    self.winch.store(false, AtomicOrdering::SeqCst);
  }

  pub fn get_cont(&self) -> bool {
    self.cont.load(AtomicOrdering::Relaxed)
  }

  pub fn unset_cont(&self) {
    self.cont.store(false, AtomicOrdering::SeqCst);
  }

  pub fn get_hup(&self) -> bool {
    self.hup.load(AtomicOrdering::Relaxed)
  }

  pub fn unset_hup(&self) {
    self.hup.store(false, AtomicOrdering::SeqCst);
  }

  pub fn get_int(&self) -> bool {
    self.int_.load(AtomicOrdering::Relaxed)
  }

  pub fn get_term(&self) -> bool {
    self.term.load(AtomicOrdering::Relaxed)
  }

  pub fn get_quit(&self) -> bool {
    self.quit.load(AtomicOrdering::Relaxed)
  }
}

#[derive(Debug, Default)]
pub struct SignalsConfigOnce {
  pub winch: bool,
  pub cont: bool,
  pub hup:  bool,
  pub int_: bool,
  pub term: bool,
  pub quit: bool,
}

impl SignalsConfigOnce {
  pub fn init(self) {
    if self.winch {
      register_signal(SIGWINCH, Arc::clone(&ONCE_SIGNALS.winch)).unwrap();
    }
    if self.cont {
      register_signal(SIGCONT, Arc::clone(&ONCE_SIGNALS.cont)).unwrap();
    }
    if self.hup {
      register_signal(SIGHUP, Arc::clone(&ONCE_SIGNALS.hup)).unwrap();
    }
    if self.int_ {
      register_signal(SIGINT, Arc::clone(&ONCE_SIGNALS.int_)).unwrap();
    }
    if self.term {
      register_signal(SIGTERM, Arc::clone(&ONCE_SIGNALS.term)).unwrap();
    }
    if self.quit {
      register_signal(SIGTERM, Arc::clone(&ONCE_SIGNALS.quit)).unwrap();
    }
  }
}
