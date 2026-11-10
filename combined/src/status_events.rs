//! Verified MHGU 1.4.0 status signals; see STATUS-RESEARCH.md.
pub fn buildups(m:&impl crate::target::Memory,state:u32)->[(i16,i16);7]{
 core::array::from_fn(|i|{
  let (a,b)=[(0x5c94,0x5ca0),(0x5c98,0x5c96),(0x5cae,0x5cac),(0x5d76,0x5d78),(0x5d82,0x5d84),(0x5d9a,0x5d9c),(0x5daa,0x5da8)][i];
  let (v,max)=(m.u16(state+a).unwrap_or(0) as i16,m.u16(state+b).unwrap_or(0) as i16);
  if v>=0&&max>0&&v<=max{(v,max)}else{(0,0)}
 })
}
// Keep KO, exhaust and mount visible through zero/reset on host and guest.
pub fn buildup_visible(index:usize,(value,max):(i16,i16))->bool{max>0&&(value>0||(3..=5).contains(&index))}
#[derive(Clone,Copy,Default)]
pub struct Signals {pub charge:Option<bool>,pub flags:u8,pub counters:[u8;4]}
#[derive(Clone,Copy)]
struct Previous {signals:Signals,seen:bool,known:bool}
impl Previous {const EMPTY:Self=Self{signals:Signals{charge:None,flags:0,counters:[0;4]},seen:false,known:false};}
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
  if let Some(active)=next.charge {
   if old.known&&old.seen {if let Some(before)=old.signals.charge {if active!=before {emit("zinogre_charge",if active{"start"}else{"end"},1);}}else if active{emit("zinogre_charge","observed",1);}}
   else if active{emit("zinogre_charge","observed",1);}
  }
  if old.known&&old.seen{for (j,name) in ["exhaust_proc","mount_opening","mount_ended","blast"].iter().enumerate(){if next.counters[j]>old.signals.counters[j]{emit(name,"trigger",next.counters[j]-old.signals.counters[j]);}}}
  self.last[i]=Previous{signals:next,seen:true,known:true};
 }
 pub fn finish(&mut self){for i in 0..64{self.last[i].seen=self.present[i];}}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn guest_buildups_stay_visible_before_hits_and_after_a_proc(){
  struct Mem(std::collections::BTreeMap<u32,u16>);
  impl crate::target::Memory for Mem{
   fn u8(&self,_:u32)->Option<u8>{None}
   fn u16(&self,a:u32)->Option<u16>{self.0.get(&a).copied()}
   fn u32(&self,_:u32)->Option<u32>{None}
  }
  let state=0x08010000;
  let mut m=Mem(std::collections::BTreeMap::from([(state+0x5d78,150),(state+0x5d84,180),(state+0x5d9c,260)]));
  // Guest actors receive these three values from netEmAction, independently
  // of which player posted the quest or owns the monster's AI.
  for values in [[0,0,0],[60,25,100],[0,0,0]]{
   for (offset,value) in [0x5d76,0x5d82,0x5d9a].into_iter().zip(values){m.0.insert(state+offset,value);}
   let b=buildups(&m,state);
   assert_eq!([b[3],b[4],b[5]],[(values[0] as i16,150),(values[1] as i16,180),(values[2] as i16,260)]);
   assert_eq!(b.iter().copied().enumerate().filter(|&(i,p)|buildup_visible(i,p)).count(),3);
  }
  m.0.insert(state+0x5d76,u16::MAX);m.0.insert(state+0x5d82,181);m.0.remove(&(state+0x5d9c));
  assert!(buildups(&m,state).iter().copied().enumerate().all(|(i,p)|!buildup_visible(i,p)));
  assert!(!buildup_visible(0,(0,180)));assert!(buildup_visible(0,(60,180)));
 }
 #[test]fn charge_transitions_and_reacquisition_are_not_fabricated(){
  let mut t=Tracker::new();let mut events=Vec::new();
  for (state,expected) in [(Some(false),None),(Some(true),Some("start")),(Some(false),Some("end")),(None,None),(Some(true),Some("observed"))]{
   events.clear();t.begin();t.observe(1,Signals{charge:state,..Signals::default()},|k,p,_|events.push((k,p)));t.finish();
   assert_eq!(events,expected.map(|p|vec![("zinogre_charge",p)]).unwrap_or_default());
  }
  t.begin();t.finish();events.clear();t.begin();t.observe(1,Signals{charge:Some(true),..Signals::default()},|k,p,_|events.push((k,p)));t.finish();assert_eq!(events,vec![("zinogre_charge","observed")]);
 }
 #[test]fn transitions_not_buildup_and_no_initial_counter_replay(){let mut t=Tracker::new();let mut e=Vec::new();t.begin();t.observe(1,Signals{charge:None,flags:2,counters:[3,2,1,4]},|a,b,c|e.push((a,b,c)));t.finish();assert_eq!(e,vec![("poison","observed",1)]);e.clear();t.begin();t.observe(1,Signals{charge:None,flags:4,counters:[4,3,2,6]},|a,b,c|e.push((a,b,c)));t.finish();assert!(e.contains(&("poison","end",1)));assert!(e.contains(&("paralysis","start",1)));assert!(e.contains(&("blast","trigger",2)));assert!(e.contains(&("mount_ended","trigger",1)));}
 #[test]fn absence_reset_and_distinct_monsters_do_not_create_procs(){let mut t=Tracker::new();let mut e=Vec::new();t.begin();t.observe(1,Signals{charge:None,flags:0,counters:[5;4]},|_,_,_|{});t.finish();t.begin();t.finish();t.begin();t.observe(1,Signals{charge:None,flags:4,counters:[7;4]},|a,b,c|e.push((a,b,c)));t.observe(2,Signals::default(),|a,b,c|e.push((a,b,c)));t.finish();assert_eq!(e,vec![("paralysis","observed",1)]);e.clear();t.begin();t.observe(1,Signals{charge:None,flags:4,counters:[0;4]},|a,b,c|e.push((a,b,c)));t.finish();assert!(e.is_empty());}
}
