//! Executes the real storage module against a deterministic SDK filesystem.
#![allow(dead_code)]
macro_rules! info { ($($a:tt)*) => {{ let _=format_args!($($a)*); }} }
macro_rules! warn { ($($a:tt)*) => {{ let _=format_args!($($a)*); }} }
#[path="../shared/config_storage.rs"] mod storage;
use std::{collections::BTreeMap,sync::{Mutex,atomic::{AtomicBool,Ordering}}};
#[derive(Default)] struct Fs { files:BTreeMap<String,Vec<u8>>, pending:BTreeMap<String,Vec<u8>>, stale_commit:BTreeMap<String,Vec<u8>>, handles:BTreeMap<u32,(String,i32)>, next:u32, trace:Vec<String>, fail_commit:bool, fail_create:bool }
static FS:Mutex<Option<Fs>>=Mutex::new(None);
static READY:AtomicBool=AtomicBool::new(false);
static REENTER:AtomicBool=AtomicBool::new(false);
fn account_ready()->bool{READY.load(Ordering::Relaxed)}
fn fs<R>(f:impl FnOnce(&mut Fs)->R)->R { f(FS.lock().unwrap().as_mut().unwrap()) }
fn path(p:*const std::ffi::c_char)->String{let s=unsafe{std::ffi::CStr::from_ptr(p)}.to_str().unwrap();assert!(s.starts_with("huntlog:/"),"shared game mount used: {s}");s[9..].to_owned()}
#[repr(C)] #[derive(Clone,Copy)] struct Handle(u32);
#[unsafe(export_name="_ZN2nn7account17GetLastOpenedUserEPNS0_3UidE")]
unsafe extern "C" fn user(p:*mut u64)->u32{assert!(account_ready());unsafe{p.write(7);p.add(1).write(9)};fs(|f|f.trace.push("user".into()));0}
#[unsafe(export_name="_ZN2nn2fs13MountSaveDataEPKcRKNS_7account3UidE")]
unsafe extern "C" fn mount(n:*const std::ffi::c_char,_:*const u64)->u32{assert_eq!(unsafe{std::ffi::CStr::from_ptr(n)}.to_bytes(),b"huntlog");fs(|f|f.trace.push("mount".into()));0}
#[unsafe(export_name="_ZN2nn2fs14CommitSaveDataEPKc")]
unsafe extern "C" fn commit(n:*const std::ffi::c_char)->u32{assert_eq!(unsafe{std::ffi::CStr::from_ptr(n)}.to_bytes(),b"huntlog");fs(|f|{assert!(!f.handles.values().any(|(_,m)|m&2!=0),"commit with open writer");assert!(f.pending.values().all(Vec::is_empty),"new nonempty file enters shared copy buffer");f.trace.push("commit-empty".into());if f.fail_commit{f.stale_commit.append(&mut f.pending);return 1;}f.files.append(&mut f.pending);0})}
#[unsafe(export_name="_ZN2nn2fs8OpenFileEPNS0_10FileHandleEPKci")]
unsafe extern "C" fn open(h:*mut Handle,p:*const std::ffi::c_char,m:i32)->u32{if REENTER.swap(false,Ordering::Relaxed){assert!(storage::journal_size().is_err(),"session must fail closed, not block or nest");}let p=path(p);fs(|f|{if !f.files.contains_key(&p)&&!f.pending.contains_key(&p){return 0x202;}f.next+=1;let id=f.next;f.handles.insert(id,(p,m));unsafe{h.write(Handle(id))};0})}
#[unsafe(export_name="_ZN2nn2fs10CreateFileEPKcx")]
unsafe extern "C" fn create(p:*const std::ffi::c_char,n:i64)->u32{assert_eq!(n,0);let p=path(p);fs(|f|{f.trace.push(format!("create0:{p}"));f.pending.insert(p,Vec::new());if f.fail_create {1}else{0}})}
#[unsafe(export_name="_ZN2nn2fs11GetFileSizeEPxNS0_10FileHandleE")]
unsafe extern "C" fn size(n:*mut i64,h:Handle)->u32{fs(|f|{let p=&f.handles[&h.0].0;let b=f.files.get(p).or_else(||f.pending.get(p)).unwrap();unsafe{n.write(b.len() as i64)};0})}
#[unsafe(export_name="_ZN2nn2fs8ReadFileENS0_10FileHandleExPvj")]
unsafe extern "C" fn read(h:Handle,o:i64,p:*mut u8,n:usize)->u32{fs(|f|{let b=&f.files[&f.handles[&h.0].0];unsafe{std::ptr::copy_nonoverlapping(b.as_ptr().add(o as usize),p,n)};0})}
#[unsafe(export_name="_ZN2nn2fs11SetFileSizeENS0_10FileHandleEx")]
unsafe extern "C" fn resize(h:Handle,n:i64)->u32{fs(|f|{let p=f.handles[&h.0].0.clone();assert!(!f.pending.contains_key(&p));f.files.get_mut(&p).unwrap().resize(n as usize,0);f.trace.push(format!("resize:{p}"));0})}
#[unsafe(export_name="_ZN2nn2fs9WriteFileENS0_10FileHandleExPKvjRKNS0_11WriteOptionE")]
unsafe extern "C" fn write(h:Handle,o:i64,p:*const u8,n:usize,opt:*const i32)->u32{assert_eq!(unsafe{*opt},1);fs(|f|{let path=f.handles[&h.0].0.clone();assert!(!f.pending.contains_key(&path),"write before empty creation committed");let b=f.files.get_mut(&path).unwrap();b.resize(b.len().max(o as usize+n),0);b[o as usize..o as usize+n].copy_from_slice(unsafe{std::slice::from_raw_parts(p,n)});f.trace.push(format!("write:{path}"));0})}
#[unsafe(export_name="_ZN2nn2fs9CloseFileENS0_10FileHandleE")]
unsafe extern "C" fn close(h:Handle){fs(|f|{f.handles.remove(&h.0).unwrap();});}
#[test] fn private_storage_preserves_files_and_never_commits_native_transaction(){
 *FS.lock().unwrap()=Some(Fs::default());let mut bytes=[0;8192];
 assert_eq!(storage::DAMAGE.read(&mut bytes),Err(storage::ReadError::Unavailable));assert!(fs(|f|f.trace.is_empty()));
 READY.store(true,Ordering::Relaxed);
 assert_eq!(storage::DAMAGE.read(&mut bytes).unwrap(),storage::DAMAGE.defaults);
 assert_eq!(fs(|f|f.trace.iter().filter(|s|s.as_str()=="mount").count()),1);
 assert_eq!(fs(|f|f.files["hunter_hud.ini"].clone()),storage::DAMAGE.defaults.as_bytes());
 let trace=fs(|f|f.trace.clone());let create=trace.iter().position(|s|s=="create0:hunter_hud.ini").unwrap();assert_eq!(trace[create+1],"commit-empty");assert_eq!(trace[create+2],"resize:hunter_hud.ini");
 fs(|f|f.trace.clear());REENTER.store(true,Ordering::Relaxed);storage::DAMAGE.read(&mut bytes).unwrap();assert!(fs(|f|f.trace.is_empty()),"reading/dropping must not commit or remount");
 storage::DAMAGE.save("enabled = false\n").unwrap();assert_eq!(fs(|f|f.files["hunter_hud.ini"].clone()),b"enabled = false\n");
 assert_eq!(storage::journal_size(),Ok(0));
 storage::journal_write(0,b"one\n").unwrap();storage::journal_write(0,b"one\n").unwrap();assert_eq!(storage::journal_size(),Ok(4));
 assert_eq!(fs(|f|f.files["hunter_hunts.jsonl"].clone()),b"one\n");assert!(fs(|f|f.handles.is_empty()&&f.pending.is_empty()));
 // Match stock fs_commit's failure: pending is cleared but .mhgu-commit remains.
 fs(|f|f.fail_commit=true);assert!(storage::DEBUG.save("diagnostic").is_err());
 assert!(fs(|f|f.pending.is_empty()&&f.stale_commit["dps_debug.log"].is_empty()));
 let trace=fs(|f|f.trace.clone());fs(|f|f.fail_commit=false);
 assert!(storage::DAMAGE.save("must not write").is_err());assert!(storage::journal_size().is_err());
 assert_eq!(fs(|f|f.trace.clone()),trace,"failed creation must latch IO off until restart");
 assert_eq!(fs(|f|f.files["hunter_hud.ini"].clone()),b"enabled = false\n");
}
#[test] fn sdk_readiness_signature_fails_closed(){
 let words=[0xe1a01000,0xe59f000c,0xe79f0000,0xe5902000,0xe5922020,0xe12fff12,0x007aba4c];
 assert_eq!(storage::account_service_slot(0x024c2934,&words),Some(0x02c6e390));
 for i in 0..7 {let mut altered=words;altered[i]^=1;assert_eq!(storage::account_service_slot(0x024c2934,&altered),None);}
 assert_eq!(storage::account_service_slot(0xfffffff0,&words),None);
}

#[test] fn create_failure_with_pending_temp_disables_storage_until_restart(){
 // Run with fresh process statics, as this failure intentionally latches forever.
 if std::env::var_os("HUNTLOG_TEST_CREATE_FAILURE").is_none(){
  let output=std::process::Command::new(std::env::current_exe().unwrap())
   .args(["--exact","create_failure_with_pending_temp_disables_storage_until_restart","--test-threads=1"])
   .env("HUNTLOG_TEST_CREATE_FAILURE","1").output().unwrap();
  assert!(output.status.success(),"{}\n{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));return;
 }
 *FS.lock().unwrap()=Some(Fs{fail_create:true,..Fs::default()});READY.store(true,Ordering::Relaxed);
 assert!(storage::DEBUG.save("must not reach pending temp").is_err());
 assert!(fs(|f|f.pending["dps_debug.log"].is_empty()));let trace=fs(|f|f.trace.clone());
 fs(|f|f.fail_create=false);
 assert!(storage::DEBUG.save("must not reach pending temp").is_err());assert!(storage::journal_size().is_err());
 assert_eq!(fs(|f|f.trace.clone()),trace,"failed create must latch before any subsequent SDK IO");
}
