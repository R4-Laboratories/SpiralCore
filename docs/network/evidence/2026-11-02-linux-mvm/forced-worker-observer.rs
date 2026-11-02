use std::{collections::BTreeSet, ffi::CString, fs, os::unix::ffi::OsStrExt, path::{Path, PathBuf}, process::{Command, Stdio}, time::{Duration, Instant}};
fn walk(path: &Path, fd: i32, watched: &mut BTreeSet<PathBuf>, candidates: &mut Vec<(u32,u32,PathBuf)>) -> std::io::Result<()> {
    let items = match fs::read_dir(path) { Ok(v)=>v, Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(()), Err(e)=>return Err(e) };
    if watched.insert(path.into()) {
        let name=CString::new(path.as_os_str().as_bytes()).expect("path");
        assert!(unsafe {libc::inotify_add_watch(fd,name.as_ptr(),libc::IN_CREATE|libc::IN_MOVED_TO|libc::IN_MODIFY)}>=0,"watch directory");
    }
    for item in items {
        let item=item?; let p=item.path(); let kind=item.file_type()?;
        if kind.is_dir() {walk(&p,fd,watched,candidates)?;}
        else if kind.is_file() && item.file_name()=="fc.pid" {
            let Some(worker)=p.parent().and_then(Path::file_name).and_then(|v|v.to_str()).and_then(|v|v.strip_prefix("spiralcore-proof-")).and_then(|v|v.parse::<u32>().ok()) else {continue};
            let Some(fc)=fs::read_to_string(&p).ok().and_then(|v|v.trim().parse::<u32>().ok()) else {continue};
            if fs::read_to_string(format!("/proc/{fc}/comm")).is_ok_and(|v|v.trim()=="firecracker") {candidates.push((worker,fc,p));}
        }
    }
    Ok(())
}
fn main() {
    let a:Vec<String>=std::env::args().collect(); assert_eq!(a.len(),5,"helper ROOT HOST KERNEL GUEST");
    let root=Path::new(&a[1]); assert!(!root.exists());
    let fd=unsafe {libc::inotify_init1(libc::IN_CLOEXEC|libc::IN_NONBLOCK)}; assert!(fd>=0);
    let parent=CString::new(root.parent().expect("parent").as_os_str().as_bytes()).expect("path");
    assert!(unsafe {libc::inotify_add_watch(fd,parent.as_ptr(),libc::IN_CREATE|libc::IN_MOVED_TO|libc::IN_MODIFY)}>=0);
    let base=root.parent().expect("parent");
    let stdout=fs::File::create(base.join("forced-session.stdout")).expect("stdout"); let stderr=fs::File::create(base.join("forced-session.stderr")).expect("stderr");
    let mut child=Command::new("sudo").args(["-n","/usr/bin/setpriv","--reuid","callard","--regid","kvm","--init-groups","/usr/bin/env","XDG_RUNTIME_DIR=/run/user/1000","DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus"]).arg(format!("PATH={}/bin:/usr/bin:/bin",base.display())).arg(&a[2]).arg("session").arg(root).args([&a[3],&a[4],"two","whole-mixed","16777216","0","experimental64"]).stdout(Stdio::from(stdout)).stderr(Stdio::from(stderr)).spawn().expect("owned session");
    let deadline=Instant::now()+Duration::from_secs(100); let mut watched=BTreeSet::new(); let mut selected=None;
    while selected.is_none() {
        let mut candidates=vec![]; walk(root,fd,&mut watched,&mut candidates).expect("watch owned tree");
        for (worker,fc,pidfile) in candidates {
            let env=fs::read(format!("/proc/{worker}/environ")).expect("worker environ");
            let admitted=[root.join("mvm-sender"),root.join("mvm-receiver")].iter().any(|p|env.split(|v|*v==0).any(|v|v==format!("MVM_HOME={}",p.display()).as_bytes()));
            if !admitted {continue;}
            let pidfd=unsafe {libc::syscall(libc::SYS_pidfd_open,worker,0)} as i32; assert!(pidfd>=0,"owned worker pidfd");
            let stat_before=fs::read_to_string(format!("/proc/{worker}/stat")).expect("worker identity");
            let start=stat_before.rsplit_once(')').expect("stat").1.split_whitespace().nth(19).expect("start tick").to_owned();
            let after=fs::read_to_string(format!("/proc/{worker}/stat")).expect("worker identity");
            assert_eq!(after.rsplit_once(')').expect("stat").1.split_whitespace().nth(19),Some(start.as_str()));
            assert_eq!(unsafe {libc::syscall(libc::SYS_pidfd_send_signal,pidfd,libc::SIGKILL,std::ptr::null::<libc::siginfo_t>(),0)},0,"signal only owned worker");
            unsafe {libc::close(pidfd)}; println!("forced_worker={worker} firecracker={fc} worker_start_ticks={start} pidfile={}",pidfile.display()); selected=Some((worker,fc)); break;
        }
        if selected.is_some() {break;}
        assert!(child.try_wait().expect("owned child status").is_none(),"session exited before worker fault");
        let remaining=deadline.checked_duration_since(Instant::now()).expect("observer deadline");
        let mut poll=libc::pollfd{fd,events:libc::POLLIN,revents:0};
        assert!(unsafe {libc::poll(&mut poll,1,remaining.as_millis().min(100000) as i32)}>0,"observer event deadline");
        let mut buf=[0u8;65536]; while unsafe {libc::read(fd,buf.as_mut_ptr().cast(),buf.len())}>0 {}
    }
    let exit=child.wait().expect("session wait"); assert!(!exit.success(),"forced failure must not succeed");
    println!("parent_exit={exit}");
    let (_,fc)=selected.expect("fault injected"); assert!(!Path::new(&format!("/proc/{fc}")).exists(),"owned Firecracker must be reaped");
    println!("selected_firecracker_gone=true"); unsafe {libc::close(fd)};
}
