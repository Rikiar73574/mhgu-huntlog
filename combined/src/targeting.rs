//! MHGU 1.4.0 Target Cam state. See TARGET-REVEAL.md for instruction evidence.
#[derive(Clone,Copy,Default,Debug)]
pub struct TargetState {pub selected:u32,pub known:[(u32,u16);3]}
impl TargetState {pub fn revealed(&self,object:u32,species:u16)->bool {
 object!=0 && self.known.iter().any(|&(p,s)|p==object && s==species)
}}
fn heap(p:u32,n:u32)->bool {p>=0x08000000 && p<0xa8000000 && p&3==0 && n<=0xa8000000-p}
pub fn capture(read:impl Fn(u32,usize)->Option<u32>,base:u32,eligible:&[u32])->TargetState {
 let empty=TargetState::default();
 // Exact getter signature + exact quest-camera vtable prevent reading town/cutscene objects.
 if base!=0x00200000 || read(base+0x29868,4)!=Some(0xe5900a6c) || read(base+0x2986c,4)!=Some(0xe12fff1e) {return empty;}
 let Some(manager)=read(base+0x184d750,4) else {return empty;};
 if !heap(manager,0x58){return empty;}
 let Some(camera)=read(manager+0x54,4) else {return empty;};
 if !heap(camera,0xb00) || read(camera,4)!=Some(base+0x1730d0c){return empty;}
 let (Some(index),Some(special))=(read(camera+0xa6c,4),read(camera+0xa8e,1)) else {return empty;};
 if special>1{return empty;}
 let mut result=empty;
 for i in 0..3 {
  let (Some(p),Some(species))=(read(camera+0xa70+i*4,4),read(camera+0xa80+i*2,2)) else {return empty;};
  if heap(p,4) && species>0 && species<=u16::MAX as u32 {result.known[i as usize]=(p,species as u16);}
 }
 if special==0 {
  // Main+29a58 indexes the non-b5f7 enemy sequence, not the remembered camera slots.
  if index>0 {result.selected=eligible.get((index-1) as usize).copied().unwrap_or(0);}
  result.known[2]=(0,0); // third slot is only used by the multipart boss camera.
 } else if (1..=3).contains(&index) {
  // Nakarkos Target Cam can select its two arm objects; HUD refers to their body.
  let body=result.known[1].0;
  if result.known[1].1==84 && eligible.contains(&body) &&
     read(camera+0xa70,4)==body.checked_add(0xcc00) && read(camera+0xa78,4)==body.checked_add(0x19750) {
    result.selected=body;
  }
 }
 result
}
#[cfg(test)] mod tests {
 use super::*;
 use std::collections::BTreeMap;
 const BASE:u32=0x200000;const MAN:u32=0x10000000;const CAM:u32=0x11000000;
 const A:u32=0x12000000;const B:u32=0x13000000;
 fn fixture()->BTreeMap<(u32,usize),u32> {BTreeMap::from([
  ((BASE+0x29868,4),0xe5900a6c),((BASE+0x2986c,4),0xe12fff1e),
  ((BASE+0x184d750,4),MAN),((MAN+0x54,4),CAM),((CAM,4),BASE+0x1730d0c),
  ((CAM+0xa6c,4),2),((CAM+0xa8e,1),0),
  ((CAM+0xa70,4),A),((CAM+0xa74,4),B),((CAM+0xa78,4),0),
  ((CAM+0xa80,2),2),((CAM+0xa82,2),17),((CAM+0xa84,2),0),
 ])}
 fn run(m:&BTreeMap<(u32,usize),u32>,eligible:&[u32])->TargetState {capture(|a,n|m.get(&(a,n)).copied(),BASE,eligible)}
 #[test] fn follows_selected_ordinal_not_cache_slot(){let m=fixture();let s=run(&m,&[B,A]);assert_eq!(s.selected,A);assert!(s.revealed(B,17));}
 #[test] fn switching_and_unselecting(){let mut m=fixture();for (index,expected) in [(1,A),(2,B),(0,0),(3,0),(u32::MAX,0)] {m.insert((CAM+0xa6c,4),index);assert_eq!(run(&m,&[A,B]).selected,expected);}}
 #[test] fn loaded_but_not_discovered_is_anonymous(){let mut m=fixture();m.insert((CAM+0xa74,4),0);let s=run(&m,&[A,B]);assert!(!s.revealed(B,17));assert!(s.revealed(A,2));}
 #[test] fn stale_slot_species_cannot_reveal_new_monster(){assert!(!run(&fixture(),&[A,B]).revealed(A,17));}
 #[test] fn wrong_camera_and_build_fail_closed(){let mut m=fixture();m.insert((CAM,4),BASE+123);assert_eq!(run(&m,&[A,B]).selected,0);assert!(!run(&m,&[A,B]).revealed(A,2));let mut m=fixture();m.insert((BASE+0x29868,4),0);assert!(!run(&m,&[A,B]).revealed(A,2));}
 #[test] fn missing_memory_fails_closed(){assert_eq!(capture(|_,_|None,BASE,&[A]).selected,0);}
 #[test] fn special_boss_part_targets_resolve_to_body(){let mut m=fixture();m.insert((CAM+0xa8e,1),1);m.insert((CAM+0xa70,4),A+0xcc00);m.insert((CAM+0xa74,4),A);m.insert((CAM+0xa78,4),A+0x19750);m.insert((CAM+0xa82,2),84);for selected in 1..=3 {m.insert((CAM+0xa6c,4),selected);let s=run(&m,&[A]);assert_eq!(s.selected,A);assert!(s.revealed(A,84));}}
}
