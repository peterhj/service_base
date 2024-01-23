pub use unix2::{set_gid, set_uid, umask};

use std::env::{set_current_dir};
//use std::fs::{create_dir_all};
use std::io::{Error as IoError};
use std::os::unix::fs::{chroot};
use std::path::{Path};
//use std::process::{Command};

pub fn protect<P: AsRef<Path>>(chroot_dir: P, uid: u32, gid: u32) -> Result<(), IoError> {
  set_current_dir(chroot_dir)?;
  chroot(".")?;
  set_current_dir("/")?;
  set_gid(gid)?;
  set_uid(uid)?;
  Ok(())
}
