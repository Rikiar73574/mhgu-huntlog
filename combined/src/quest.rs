//! Quest lifecycle: explicit town closes a run; unavailable memory never does.
use crate::target::Memory;
#[derive(Clone,Copy,Debug,PartialEq)] pub enum Observation {Unknown,Town,Quest{id:u32,remaining:u32,elapsed:f32,stage:u8}}
pub fn capture(m:&impl Memory,base:u32)->Observation {
 let Some(gc)=m.u32(base.wrapping_add(0x188ca48)) else{return Observation::Unknown};
 match m.u8(gc.wrapping_add(0x4c)) {Some(0..=7)=>return Observation::Town,Some(255)=>{},_=>return Observation::Unknown}
 let Some(sq)=m.u32(base.wrapping_add(0x188ad90)) else{return Observation::Unknown};
 let Some(qd)=m.u32(sq.wrapping_add(0xa4)) else{return Observation::Unknown};
 let Some(id)=m.u32(sq.wrapping_add(0xe0)) else{return Observation::Unknown};
 if id==0 || m.u32(qd.wrapping_add(8))!=Some(id){return Observation::Unknown;}
 let Some(limit)=m.u32(sq.wrapping_add(0xcc)) else{return Observation::Unknown};
 let Some(remaining)=m.u32(sq.wrapping_add(0x1c)) else{return Observation::Unknown};
 let Some(clock)=m.u32(base.wrapping_add(0x2126764)) else{return Observation::Unknown};
 let Some(rate)=m.u32(clock.wrapping_add(0x38)).map(f32::from_bits) else{return Observation::Unknown};
 let rate=rate as u32;
 if !(1..=180).contains(&limit)|| !(1..=240).contains(&rate)||remaining>limit*60*rate{return Observation::Unknown;}
 Observation::Quest{id,remaining,elapsed:(limit*60*rate-remaining) as f32/rate as f32,stage:m.u8(qd.wrapping_add(0x10)).unwrap_or(0)}
}
#[cfg(test)] mod tests {
 use super::*;use std::collections::BTreeMap;
 struct Mem(BTreeMap<u32,u32>);impl Memory for Mem{fn u8(&self,a:u32)->Option<u8>{self.0.get(&a).map(|v|*v as u8)}fn u16(&self,a:u32)->Option<u16>{self.0.get(&a).map(|v|*v as u16)}fn u32(&self,a:u32)->Option<u32>{self.0.get(&a).copied()}}
 fn memory()->Mem {Mem(BTreeMap::from([(0x188ca48,1000),(1048+28,255),(0x188ad90,2000),(2164,3000),(2224,42),(3008,42),(2204,50),(2028,179400),(0x2126764,4000),(4056,60f32.to_bits())]))}
 #[test] fn frame_step_is_converted_to_seconds(){assert!((frame_seconds(&memory(),0,1.0)-1.0/60.0).abs()<0.000001);}
 #[test] fn timer_comes_from_game_clock(){assert!(matches!(capture(&memory(),0),Observation::Quest{id:42,elapsed:10.0,..}));}
 #[test] fn invalid_fields_preserve_run(){let mut m=memory();m.0.remove(&3008);assert_eq!(capture(&m,0),Observation::Unknown);}
 #[test] fn explicit_town_does_not_require_player_or_quest(){let m=Mem(BTreeMap::from([(0x188ca48,1000),(1076,0)]));assert_eq!(capture(&m,0),Observation::Town);}
}

/// Recomp passes the game frame-step (normally 1), not elapsed seconds.
pub fn frame_seconds(m:&impl Memory,base:u32,step:f32)->f32 {
 let rate=base.checked_add(0x2126764).and_then(|a|m.u32(a)).and_then(|p|p.checked_add(0x38)).and_then(|a|m.u32(a)).map(f32::from_bits).filter(|v|v.is_finite()&&*v>=1.0&&*v<=240.0).unwrap_or(60.0);
 if step.is_finite()&&step>0.0{step/rate}else{1.0/rate}
}
