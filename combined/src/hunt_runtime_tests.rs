//! Integration harness runs the real runtime against deterministic memory/filesystem adapters.
mod item_events;mod part_names;mod parts;mod player_details;mod rewards;mod status_events;mod target;mod quest;mod recorder;mod monster_size;mod hunt_runtime;
mod data {pub struct Shared<T>(std::sync::Mutex<T>);impl<T> Shared<T>{pub const fn new(v:T)->Self{Self(std::sync::Mutex::new(v))}pub fn with<R>(&self,f:impl FnOnce(&mut T)->R)->R{f(&mut self.0.lock().unwrap())}}}
static MEMORY:std::sync::Mutex<std::collections::BTreeMap<u32,u32>>=std::sync::Mutex::new(std::collections::BTreeMap::new());
mod monsters {
 #[derive(Clone,Copy,Default)] pub struct Monster{pub object:u32,pub network:u16,pub species:u16,pub hp:i32,pub max_hp:i32,pub room:u8,pub size_scale:f32,pub status:Option<crate::status_events::Signals>}
 pub struct Snapshot{pub entries:[Monster;10],pub count:usize}
 pub struct GameMemory{pub base:u32}
 impl crate::target::Memory for GameMemory{fn u8(&self,a:u32)->Option<u8>{self.u32(a).map(|v|v as u8)}fn u16(&self,a:u32)->Option<u16>{self.u32(a).map(|v|v as u16)}fn u32(&self,a:u32)->Option<u32>{crate::MEMORY.lock().unwrap().get(&a).copied()}}
 pub fn name(_:u16)->Option<&'static str>{Some("Rathalos")}
 pub fn manager()->u32{0}
}
mod plugin_config {
 pub static FILE:std::sync::Mutex<Vec<u8>>=std::sync::Mutex::new(Vec::new());
 pub static FAIL:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
 pub fn journal_size()->Result<u64,&'static str>{Ok(FILE.lock().unwrap().len() as u64)}
 pub fn journal_write(offset:u64,bytes:&[u8])->Result<(),&'static str>{if FAIL.load(std::sync::atomic::Ordering::Relaxed){return Err("simulated disk failure");}let mut f=FILE.lock().unwrap();let o=offset as usize;let size=(o+bytes.len()).max(f.len());f.resize(size,0);f[o..o+bytes.len()].copy_from_slice(bytes);Ok(())}
}
struct Name{bytes:[u8;32],len:u8}unsafe fn owner_name(_:u32,_:u32)->Name{Name{bytes:[0;32],len:0}}
#[test] fn background_runs_survive_loading_and_retry_without_duplicate_damage(){
 use std::sync::atomic::Ordering;
 *MEMORY.lock().unwrap()=std::collections::BTreeMap::from([(0x188ca48,1000),(1076,255),(0x188ad90,2000),(2164,3000),(2224,42),(3008,42),(2204,50),(2028,180000),(0x2126764,4000),(4056,60f32.to_bits())]);
 MEMORY.lock().unwrap().extend([(1048,1),(0x1885b58,5000),(5000+0x20,0x08001000),(0x08001000+0x13c4,0x08003000),(0x08003000+0x554,47)]);
 let mut snap=monsters::Snapshot{entries:[monsters::Monster::default();10],count:2};
 snap.entries[0]=monsters::Monster{object:8000,network:1,species:2,hp:100,max_hp:100,room:1,size_scale:1.0,status:Some(status_events::Signals::default())};snap.entries[1]=monsters::Monster{object:9000,network:2,..snap.entries[0]};
 hunt_runtime::update(0,1.0,&snap);let first=hunt_runtime::status().1;assert_ne!(first,0);
 snap.entries[0].status=Some(status_events::Signals{flags:4,counters:[1,0,0,0]});hunt_runtime::hit(777,0,8000,30,0);MEMORY.lock().unwrap().insert(2028,179940);hunt_runtime::update(0,1.0,&snap);
 MEMORY.lock().unwrap().remove(&3008);hunt_runtime::update(0,1.0,&snap);assert_eq!(hunt_runtime::status().1,first);
 let mut reward=crate::rewards::Snapshot::EMPTY;reward.items[0][0]=crate::rewards::Item{id:576,quantity:2};hunt_runtime::reward_snapshot(reward);hunt_runtime::reward_snapshot(reward);
 plugin_config::FAIL.store(true,Ordering::Relaxed);MEMORY.lock().unwrap().insert(1076,0);hunt_runtime::update(0,1.0,&snap);assert_eq!(hunt_runtime::status().0,"simulated disk failure");
 plugin_config::FAIL.store(false,Ordering::Relaxed);hunt_runtime::update(0,5.0,&snap);
 MEMORY.lock().unwrap().extend([(1076,255),(3008,42),(2028,180000)]);hunt_runtime::update(0,1.0,&snap);let second=hunt_runtime::status().1;assert_ne!(second,first);
 MEMORY.lock().unwrap().insert(1076,0);hunt_runtime::update(0,1.0,&snap);
 let file=plugin_config::FILE.lock().unwrap();let text=std::str::from_utf8(&file).unwrap();assert_eq!(text.matches("\"event\":\"start\"").count(),2);assert_eq!(text.matches("\"event\":\"end\"").count(),2);assert_eq!(text.matches("\"event\":\"hit\"").count(),1);assert!(text.contains("\"id\":2,\"network_id\":2"));assert!(text.contains("\"player\":1"));assert!(text.contains("\"hr\":47,\"is_local\":true"));
 assert!(text.contains("\"kind\":\"paralysis\",\"phase\":\"start\""));assert!(text.contains("\"kind\":\"exhaust_proc\""));assert_eq!(text.matches("\"event\":\"loot\"").count(),1);assert!(text.contains("\"item_id\":576,\"quantity\":2"));
 if let Ok(path)=std::env::var("HUNT_TEST_JOURNAL"){std::fs::write(path,&*file).unwrap();}
}
