use std::{fs::{self,OpenOptions},io::{Read,Write},os::{fd::AsRawFd,unix::{fs::OpenOptionsExt,net::UnixStream,process::CommandExt}},process::{Command,Stdio}};
unsafe extern "C" { fn read(fd:i32,p:*mut u8,n:usize)->isize; fn write(fd:i32,p:*const u8,n:usize)->isize; fn getpid()->i32; }
fn main(){
 let root=std::env::temp_dir().join(format!("spiral-fork-window-{}",std::process::id()));fs::create_dir(&root).unwrap();
 let path=root.join("holders");let shared=OpenOptions::new().read(true).write(true).create_new(true).mode(0o600).open(&path).unwrap();shared.try_lock_shared().unwrap();
 let (mut parent,child)=UnixStream::pair().unwrap();let fd=child.as_raw_fd();
 let task=std::thread::spawn(move||{let mut c=Command::new("/bin/true");c.stdin(Stdio::null()).stdout(Stdio::null());unsafe{c.pre_exec(move||{let pid=getpid().to_ne_bytes();if write(fd,pid.as_ptr(),4)!=4{return Err(std::io::Error::last_os_error())}let mut release=[0u8;1];if read(fd,release.as_mut_ptr(),1)!=1{return Err(std::io::Error::last_os_error())}Ok(())});}let result=c.spawn();drop(child);result.unwrap()});
 let mut pid=[0u8;4];parent.read_exact(&mut pid).unwrap();let child_pid=i32::from_ne_bytes(pid);drop(shared);
 let exclusive=OpenOptions::new().read(true).write(true).open(&path).unwrap();let busy=matches!(exclusive.try_lock(),Err(fs::TryLockError::WouldBlock));
 parent.write_all(&[1]).unwrap();let mut proc=task.join().unwrap();assert!(proc.wait().unwrap().success());let after=exclusive.try_lock().is_ok();
 println!("{{\"scope\":\"real-CLOEXEC-preexec-flock\",\"child\":{child_pid},\"busy_before_exec\":{busy},\"exclusive_after_exec\":{after}}}");assert!(busy&&after);drop(exclusive);fs::remove_dir_all(root).unwrap();
}
