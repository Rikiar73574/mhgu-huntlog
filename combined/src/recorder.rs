//! Append-only, HUD-independent hunt journal. Game memory and filesystem live in adapters.
use core::fmt::{self,Write};
// Recomp implements the standard SDK clock using host Unix time.
#[cfg(target_arch="arm")]
fn wall_clock()->Option<i64>{
 unsafe extern "C" { #[link_name="_ZN2nn4time23StandardUserSystemClock14GetCurrentTimeEPNS0_9PosixTimeE"] fn now(out:*mut i64)->u32; }
 let mut t=0; if unsafe{now(&mut t)}==0 && t>0{Some(t)}else{None}
}
#[cfg(not(target_arch="arm"))]
fn wall_clock()->Option<i64>{None}
pub const CAP:usize=65536;
pub struct Text<const N:usize>{pub bytes:[u8;N],pub len:usize}
impl<const N:usize> Text<N>{pub const fn new()->Self{Self{bytes:[0;N],len:0}} pub fn text(&self)->&str{core::str::from_utf8(&self.bytes[..self.len]).unwrap()}}
impl<const N:usize> Write for Text<N>{fn write_str(&mut self,s:&str)->fmt::Result{if self.len+s.len()>N{return Err(fmt::Error)}self.bytes[self.len..self.len+s.len()].copy_from_slice(s.as_bytes());self.len+=s.len();Ok(())}}
pub fn quoted(out:&mut impl Write,s:&str)->fmt::Result {out.write_char('"')?;for c in s.chars(){match c{'"'=>out.write_str("\\\"")?,'\\'=>out.write_str("\\\\")?,'\n'=>out.write_str("\\n")?,'\r'=>out.write_str("\\r")?,'\t'=>out.write_str("\\t")?,c if c<' '=>write!(out,"\\u{:04x}",c as u32)?,c=>out.write_char(c)?}}out.write_char('"')}
#[derive(Clone,Copy)] pub struct Monster {pub object:u32,pub network:u16,pub species:u16,}
impl Monster {#[cfg(test)] pub const EMPTY:Self=Self{object:0,network:0,species:0};}
#[derive(Clone,Copy)] pub struct Hit {pub part:u8,pub player:u32,pub monster:u32,pub damage:i32}
impl Hit {pub const EMPTY:Self=Self{part:255,player:0,monster:0,damage:0};}
#[derive(Clone,Copy)] struct Identity {object:u32,network:u16,species:u16}
pub struct Recorder {
 pub bytes:Text<CAP>,pub offset:u64,pub run:u64,pub time:f32,pub quest:u32,pub dropped:u32,
 identities:[Identity;64],count:usize,
}
impl Recorder {
 pub const fn new()->Self{Self{bytes:Text::new(),offset:0,run:0,time:0.0,quest:0,dropped:0,identities:[Identity{object:0,network:0,species:0};64],count:0}}
 pub fn initialize(&mut self,offset:u64){self.offset=offset;let _=self.bytes.write_str("\n");}
 pub fn committed(&mut self){self.offset+=self.bytes.len as u64;self.bytes.len=0;}
 pub fn append(&mut self,line:&str){if line.len()+1>CAP.saturating_sub(1024+self.bytes.len){self.dropped=self.dropped.saturating_add(1);return;}let _=self.bytes.write_str(line);let _=self.bytes.write_str("\n");}
 pub fn start(&mut self,quest:u32,elapsed:f32){self.run=self.offset+self.bytes.len as u64+1;self.quest=quest;self.time=elapsed;self.count=0;self.dropped=0;let mut l=Text::<1024>::new();let _=write!(l,"{{\"schema\":1,\"run\":{},\"event\":\"start\",\"t\":{},\"quest_id\":{},\"timer_source\":\"game_quest_timer\",\"partial\":{},\"recorder_version\":2,\"status_tracking\":true,\"quest_name\":null,\"started_at\":null}}",self.run,self.time,quest,elapsed>2.0);self.append(l.text());}
 pub fn monster_id(&mut self,m:Monster)->u32 {
  if let Some(i)=self.identities[..self.count].iter().position(|x|x.object==m.object&&x.network==m.network&&x.species==m.species){return i as u32+1;}
  if self.count==64{self.dropped=self.dropped.saturating_add(1);return 0;}
  self.identities[self.count]=Identity{object:m.object,network:m.network,species:m.species};self.count+=1;self.count as u32
 }
 pub fn hit(&mut self,h:Hit){if self.run==0||h.damage<=0{return;}let id=self.identities[..self.count].iter().rposition(|m|m.object==h.monster).map(|i|i+1);if id.is_none(){return;}let mut l=Text::<256>::new();let _=write!(l,"{{\"schema\":1,\"run\":{},\"event\":\"hit\",\"t\":{},\"player\":{},\"monster\":",self.run,self.time,h.player);if let Some(id)=id{let _=write!(l,"{}",id);}else{let _=l.write_str("null");}let _=write!(l,",\"damage\":{},\"part\":",h.damage);if h.part<8{let _=write!(l,"{}",h.part);}else{let _=l.write_str("null");}let _=l.write_char('}');self.append(l.text());}
 pub fn end(&mut self,reason:&str){if self.run==0{return;}let mut l=Text::<512>::new();let _=write!(l,"{{\"schema\":1,\"run\":{},\"event\":\"end\",\"t\":{},\"outcome\":null,\"dropped_events\":{},\"reason\":",self.run,self.time,self.dropped);let _=quoted(&mut l,reason);if let Some(t)=wall_clock(){let _=write!(l,",\"completed_at\":{}",t);}let _=l.write_str("}");if l.len+1<=CAP-self.bytes.len {let _=self.bytes.write_str(l.text());let _=self.bytes.write_str("\n");self.run=0;}}
}
#[cfg(test)] mod tests{use super::*;
 #[test] fn repeat_quests_have_distinct_durable_ids(){let mut r=Recorder::new();r.initialize(900);r.start(42,0.0);let first=r.run;r.end("returned_to_town");r.start(42,0.0);assert_ne!(r.run,first);r.committed();let offset=r.offset;let mut restarted=Recorder::new();restarted.initialize(offset);restarted.start(42,0.0);assert!(restarted.run>r.run);}
 #[test] fn same_species_are_distinct_instances_and_ids_survive_absence(){let mut r=Recorder::new();let a=Monster{object:10,network:1,species:2,..Monster::EMPTY};let b=Monster{object:20,network:2,..a};assert_eq!(r.monster_id(a),1);assert_eq!(r.monster_id(b),2);assert_eq!(r.monster_id(a),1);assert_eq!(r.monster_id(Monster{network:3,..a}),3);}
 #[test] fn buffer_overflow_keeps_complete_lines_and_reports_loss(){let mut r=Recorder::new();r.bytes.len=CAP-2;r.append("abc");assert_eq!(r.bytes.len,CAP-2);assert_eq!(r.dropped,1);}
 #[test] fn json_escapes_names(){let mut t=Text::<64>::new();quoted(&mut t,"a\"\\\n\u{1}").unwrap();assert_eq!(t.text(),"\"a\\\"\\\\\\n\\u0001\"");}
 #[test] fn end_survives_full_normal_buffer(){let mut r=Recorder::new();r.start(7,0.0);r.bytes.len=CAP-1024;r.append("sample");assert_eq!(r.dropped,1);r.end("returned_to_town");assert_eq!(r.run,0);assert!(r.bytes.text().ends_with("\"returned_to_town\"}\n"));}
 #[test] fn damage_to_small_or_unknown_monsters_is_not_recorded(){let mut r=Recorder::new();r.start(1,0.0);r.monster_id(Monster{object:10,network:1,species:17});let before=r.bytes.len;r.hit(Hit{part:255,player:1,monster:99,damage:30});assert_eq!(r.bytes.len,before);r.hit(Hit{part:255,player:1,monster:10,damage:20});assert!(r.bytes.text().contains("\"monster\":1,\"damage\":20"));}
 #[test] fn idle_hits_are_ignored(){let mut r=Recorder::new();r.hit(Hit{part:255,damage:10,..Hit::EMPTY});assert_eq!(r.bytes.len,0);}
}
