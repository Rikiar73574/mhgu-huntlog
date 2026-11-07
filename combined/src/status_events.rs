//! Verified MHGU 1.4.0 status signals; see STATUS-RESEARCH.md.
#[derive(Clone,Copy,Default)]
pub struct Signals {pub flags:u8,pub counters:[u8;4]}
#[derive(Clone,Copy)]
struct Previous {signals:Signals,seen:bool,known:bool}
impl Previous {const EMPTY:Self=Self{signals:Signals{flags:0,counters:[0;4]},seen:false,known:false};}
pub struct Tracker {last:[Previous;64],present:[bool;64]}
impl Tracker {
 pub const fn new()->Self{Self{last:[Previous::EMPTY;64],present:[false;64]}}
 pub fn begin(&mut self){self.present=[false;64];}
 pub fn observe(&mut self,id:u32,next:Signals,mut emit:impl FnMut(&'static str,&'static str,u8)){
  if id==0||id>64{return;}let i=(id-1) as usize;let old=self.last[i];self.present[i]=true;
  for (mask,name) in [(1,"sleep"),(2,"poison"),(4,"paralysis"),(16,"stun")]{
   let active=next.flags&mask!=0;let before=old.signals.flags&mask!=0;
   if !old.known||!old.seen{if active{emit(name,"observed",1);}}
   else if active!=before{emit(name,if active{"start"}else{"end"},1);}
  }
  if old.known&&old.seen{for (j,name) in ["exhaust_proc","mount_opening","mount_ended","blast"].iter().enumerate(){if next.counters[j]>old.signals.counters[j]{emit(name,"trigger",next.counters[j]-old.signals.counters[j]);}}}
  self.last[i]=Previous{signals:next,seen:true,known:true};
 }
 pub fn finish(&mut self){for i in 0..64{self.last[i].seen=self.present[i];}}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn transitions_not_buildup_and_no_initial_counter_replay(){let mut t=Tracker::new();let mut e=Vec::new();t.begin();t.observe(1,Signals{flags:2,counters:[3,2,1,4]},|a,b,c|e.push((a,b,c)));t.finish();assert_eq!(e,vec![("poison","observed",1)]);e.clear();t.begin();t.observe(1,Signals{flags:4,counters:[4,3,2,6]},|a,b,c|e.push((a,b,c)));t.finish();assert!(e.contains(&("poison","end",1)));assert!(e.contains(&("paralysis","start",1)));assert!(e.contains(&("blast","trigger",2)));assert!(e.contains(&("mount_ended","trigger",1)));}
 #[test]fn absence_reset_and_distinct_monsters_do_not_create_procs(){let mut t=Tracker::new();let mut e=Vec::new();t.begin();t.observe(1,Signals{flags:0,counters:[5;4]},|_,_,_|{});t.finish();t.begin();t.finish();t.begin();t.observe(1,Signals{flags:4,counters:[7;4]},|a,b,c|e.push((a,b,c)));t.observe(2,Signals::default(),|a,b,c|e.push((a,b,c)));t.finish();assert_eq!(e,vec![("paralysis","observed",1)]);e.clear();t.begin();t.observe(1,Signals{flags:4,counters:[0;4]},|a,b,c|e.push((a,b,c)));t.finish();assert!(e.is_empty());}
}
