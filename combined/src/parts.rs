//! MHGU part pools and confirmed break flags; see PARTS-RESEARCH.md.
use crate::target::Memory;
use core::fmt::Write;
#[derive(Clone,Copy,Default,Debug)]pub struct Part{pub active:bool,pub name:&'static str,pub hp:i16,pub flinches:u8,pub sever_hp:i16,pub sever_count:u8,pub groups:u8,pub base:u16,pub max:i16,pub sever_base:u16,pub sever_required:u16,pub sever_type:u8,pub sever_max:i16,pub requirements_lr:[u8;8],pub requirements_hr:[u8;8],pub linked_groups:u8}
#[derive(Clone,Copy,Default,Debug)]pub struct Snapshot{pub parts:[Part;8],pub breaks:u8}
fn ptr(p:u32)->bool{(0x08000000..0xa8000000).contains(&p)&&p&3==0}
pub fn capture(m:&impl Memory,owner:u32,species:u16)->Option<Snapshot>{
 // Siege actors override the generic pool accessors.
 if matches!(species,12|84|87){return None;}
 let s=m.u32(owner.checked_add(0x1428)?).filter(|p|ptr(*p))?;
 let param=m.u32(owner+0x75ec).filter(|p|ptr(*p))?;
 // main+71078 returns param+0x64; its caller reads the pointer at +4.
 let thresholds=m.u32(param+0x68).filter(|p|ptr(*p))?;
 let signature=core::array::from_fn(|i|m.u16(thresholds+i as u32*10).unwrap_or(0));
 let labels=if crate::part_names::signature(species)==Some(signature){crate::part_names::rows(species)}else{&[]};
 let mut out=Snapshot{breaks:m.u8(s+0x5cbe)?,..Snapshot::default()};
 for i in 0..8u32{
  if m.u16(thresholds+i*10).unwrap_or(0)==0{continue;}
  let p=s+i*12;let row=thresholds+i*10;
  out.parts[i as usize]=Part{active:true,name:labels.get(i as usize).map_or("",|r|r.0),hp:m.u16(p+0x3be)? as i16,flinches:m.u8(p+0x3bc)?,sever_hp:m.u16(p+0x3c0)? as i16,sever_count:m.u8(p+0x3bd)?,groups:0,base:m.u16(row)?,sever_base:m.u16(row+6).unwrap_or(0),sever_required:m.u16(row+2).unwrap_or(0),sever_type:m.u8(row+4).unwrap_or(255),..Part::default()};
 }
 // Break definitions: [pool, LR flinch requirement, HR requirement, group].
 // Multiple pools can contribute to one break group; do not invent body names.
 if let Some(def)=m.u32(owner+0x75f0).filter(|p|ptr(*p)){
  if let (Some(rows),Some(count))=(m.u32(def+0x64).filter(|p|ptr(*p)),m.u8(def+0x8c).filter(|n|*n<=64)){
   for i in 0..u32::from(count){let p=rows+i*4;let Some(slot)=m.u8(p).filter(|n|*n<8)else{continue};if let Some(group)=m.u8(p+3).map(|n|n&0x7f).filter(|n|*n<8){let part=&mut out.parts[slot as usize];part.groups|=1<<group;part.requirements_lr[group as usize]=m.u8(p+1).unwrap_or(0);part.requirements_hr[group as usize]=m.u8(p+2).unwrap_or(0);if m.u8(p+3).unwrap_or(0)&0x80!=0{part.linked_groups|=1<<group;}}}
  }
 }
 if !labels.is_empty(){for p in &mut out.parts{p.active=!p.name.is_empty()||p.groups!=0||p.sever_required>0;}}
 MAXIMA.with(|cache|{if let Some(c)=cache.iter().find(|c|c.owner==owner&&c.state==s&&c.param==param){for i in 0..8{out.parts[i].max=c.hp[i];out.parts[i].sever_max=c.sever[i];}}});
 Some(out)
}
pub fn append(w:&mut impl Write,s:Option<Snapshot>){
 let _=w.write_str(",\"parts\":");let Some(s)=s else{let _=w.write_str("null");return};
 let _=w.write_char('[');let mut n=0;
 for(i,p)in s.parts.iter().enumerate(){if !p.active{continue;}if n>0{let _=w.write_char(',');}n+=1;
 let _=write!(w,"{{\"slot\":{},\"name\":{:?},\"flinch_hp\":{},\"flinches\":{},\"sever_hp\":{},\"sever_count\":{},\"break_groups\":{},\"broken_groups\":{},\"base_stagger\":{},\"stagger_max\":{},\"sever_base\":{},\"sever_required\":{},\"sever_type\":{},\"sever_max\":{},\"linked_groups\":{},\"requirements_lr\":{:?},\"requirements_hr\":{:?}}}",i,p.name,p.hp,p.flinches,p.sever_hp,p.sever_count,p.groups,p.groups&s.breaks,p.base,p.max,p.sever_base,p.sever_required,p.sever_type,p.sever_max,p.linked_groups,p.requirements_lr,p.requirements_hr);
 }let _=write!(w,"],\"break_mask\":{}",s.breaks);
}
pub struct Tracker{seen:[Option<u8>;64],counts:[[Option<(u8,u8)>;8];64]}
impl Tracker{
 pub const fn new()->Self{Self{seen:[None;64],counts:[[None;8];64]}}
 pub fn observe_pools(&mut self,id:u32,s:&Snapshot,mut emit:impl FnMut(usize,&'static str,u8)){
  if id==0||id>64{return;}
  for (slot,p) in s.parts.iter().enumerate(){if !p.active{continue;}let last=&mut self.counts[id as usize-1][slot];
   if let Some((flinch,sever))=*last{if p.flinches>flinch{emit(slot,"flinch",p.flinches);}if p.sever_count>sever{emit(slot,if p.severed(){"sever"}else{"secondary"},p.sever_count);}}
   *last=Some((p.flinches,p.sever_count));
  }
 }
 pub fn observe(&mut self,id:u32,mask:u8,mut emit:impl FnMut(u8,bool)){
  if id==0||id>64{return;}let old=self.seen[id as usize-1];let fresh=mask&!old.unwrap_or(0);
  for group in 0..8{if fresh&(1<<group)!=0{emit(group,old.is_some());}}
  self.seen[id as usize-1]=Some(old.unwrap_or(0)|mask);
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn flinches_are_not_breaks_and_reappearance_does_not_duplicate(){let mut t=Tracker::new();let mut e=Vec::new();t.observe(1,0,|g,new|e.push((g,new)));t.observe(1,1,|g,new|e.push((g,new)));t.observe(1,1,|g,new|e.push((g,new)));t.observe(1,0,|g,new|e.push((g,new)));t.observe(1,1,|g,new|e.push((g,new)));t.observe(2,1,|g,new|e.push((g,new)));assert_eq!(e,vec![(0,true),(0,false)]);}
 struct Mem(std::collections::BTreeMap<u32,u32>);
 impl Memory for Mem{fn u8(&self,a:u32)->Option<u8>{self.0.get(&a).map(|v|*v as u8)}fn u16(&self,a:u32)->Option<u16>{self.0.get(&a).map(|v|*v as u16)}fn u32(&self,a:u32)->Option<u32>{self.0.get(&a).copied()}}
 #[test]fn decoder_distinguishes_pool_damage_and_confirmed_break_group(){
 let owner=0x08000000;let state=0x08100000;let param=0x08200000;let table=0x08300000;let def=0x08400000;let rows=0x08500000;
 let mut m=Mem(std::collections::BTreeMap::from([(owner+0x1428,state),(owner+0x75ec,param),(param+4,0x5253),(param+0x68,table),(table,100),(state+0x5cbe,0),(state+0x3be,61),(state+0x3bc,2),(state+0x3c0,0),(state+0x3bd,0),(owner+0x75f0,def),(def+0x64,rows),(def+0x8c,1),(rows,0),(rows+3,1)]));
 let s=capture(&m,owner,85).unwrap();assert_eq!(s.parts[0].hp,61);assert_eq!(s.parts[0].flinches,2);assert_eq!(s.parts[0].groups,2);assert_eq!(s.breaks,0);assert!(!s.parts[1].active);
 m.0.insert(state+0x5cbe,2);let mut text=String::new();append(&mut text,capture(&m,owner,85));assert!(text.contains("\"broken_groups\":2"));assert!(capture(&m,owner,84).is_none());m.0.insert(param+0x68,1);assert!(capture(&m,owner,85).is_none());
 }
 #[test]fn complete_live_arzuros_signature_names_only_five_real_pools(){
 let owner=0x08600000;let state=0x08700000;let param=0x08800000;let table=0x08900000;
 let mut m=Mem(std::collections::BTreeMap::from([(owner+0x1428,state),(owner+0x75ec,param),(param+4,0x5253),(param+0x68,table),(state+0x5cbe,2)]));
 for (i,base) in [130,180,80,150,220,100,100,100].iter().enumerate(){let i=i as u32;m.0.extend([(table+i*10,*base),(state+0x3be+i*12,70),(state+0x3bc+i*12,1),(state+0x3c0+i*12,0),(state+0x3bd+i*12,0)]);}
 record_reset(&m,owner,2,None);
 let snapshot=capture(&m,owner,60).unwrap();assert_eq!(snapshot.parts.iter().filter(|p|p.active).count(),5);assert_eq!(snapshot.parts[2].name,"Forelegs");assert_eq!(snapshot.parts[2].max,70);
 m.0.insert(state+0x3be+24,64);assert_eq!(capture(&m,owner,60).unwrap().parts[2].max,70);
 m.0.insert(table+10,179);assert_eq!(capture(&m,owner,60).unwrap().parts[2].name,"");
 }
 #[test]fn options_and_sever_progress_do_not_hide_stagger_after_break(){
 let ordinary=Part{active:true,name:"Head",..Part::default()};let breakable=Part{groups:1,..ordinary};
 assert!(!ordinary.visible(true,true,false));assert!(ordinary.visible(false,false,true));assert!(breakable.visible(true,false,false));assert!(breakable.visible(false,true,false));assert!(!breakable.visible(false,false,false));
 let mut tail=Part{active:true,name:"Tail",sever_type:1,sever_base:230,sever_required:1,..Part::default()};assert!(tail.severable());assert!(!tail.severed());tail.sever_count=1;assert!(tail.severed());assert!(tail.visible(false,false,true));tail.name="Head";assert!(!tail.severed());
 }
 #[test]fn part_transitions_are_per_monster_and_do_not_replay_initial_state(){
 let mut t=Tracker::new();let mut s=Snapshot::default();s.parts[2]=Part{active:true,name:"Tail",sever_type:1,sever_base:230,sever_required:1,..Part::default()};let mut events=Vec::new();
 t.observe_pools(1,&s,|a,b,c|events.push((a,b,c)));s.parts[2].flinches=1;s.parts[2].sever_count=1;
 t.observe_pools(1,&s,|a,b,c|events.push((a,b,c)));t.observe_pools(1,&s,|a,b,c|events.push((a,b,c)));t.observe_pools(2,&s,|a,b,c|events.push((a,b,c)));
 assert_eq!(events,vec![(2,"flinch",1),(2,"sever",1)]);
 }
 #[test]fn interior_padding_keeps_kecha_hind_legs_and_tail_in_their_slots(){let rows=crate::part_names::rows(65);assert_eq!(rows[3],("",0));assert_eq!(rows[4].0,"Hind Legs");assert_eq!(rows[5].0,"Tail");}
 #[test]fn saved_maccao_pool_is_flinch_hp_not_break_count(){
 let before=[0xbd,0xd4,0xb8,0x43,0x93,0x01,0,0,61,0,0,0];let after=[0xbd,0xd4,0xb8,0x43,0x93,0x01,2,0,2,0,0,0];
 assert_eq!(before[6],0);assert_eq!(after[6],2);assert_eq!(after[7],0);assert_eq!(i16::from_le_bytes([before[8],before[9]]),61);assert_eq!(i16::from_le_bytes([after[8],after[9]]),2);
 }
}

// Filled immediately after the game's own reset calculation, retaining quest and
// monster-state scaling rather than substituting the database's base threshold.
#[derive(Clone,Copy)]struct Maximum{owner:u32,state:u32,param:u32,hp:[i16;8],sever:[i16;8]}
const EMPTY_MAX:Maximum=Maximum{owner:0,state:0,param:0,hp:[0;8],sever:[0;8]};
static MAXIMA:crate::data::Shared<[Maximum;32]>=crate::data::Shared::new([EMPTY_MAX;32]);
pub fn reset_maxima(){MAXIMA.with(|c|*c=[EMPTY_MAX;32]);}
pub fn record_reset(m:&impl Memory,owner:u32,slot:u32,before_sever:Option<i16>){
 if slot>=8{return;}
 let Some(state)=m.u32(owner+0x1428).filter(|p|ptr(*p))else{return};
 let Some(param)=m.u32(owner+0x75ec).filter(|p|ptr(*p))else{return};
 let Some(hp)=m.u16(state+0x3be+slot*12).map(|v|v as i16)else{return};
 let sever=m.u16(state+0x3c0+slot*12).map(|v|v as i16).unwrap_or(0);
 MAXIMA.with(|cache|{let i=cache.iter().position(|c|c.owner==owner).or_else(||cache.iter().position(|c|c.owner==0));if let Some(i)=i{let c=&mut cache[i];if c.owner!=owner||c.state!=state||c.param!=param{*c=Maximum{owner,state,param,..EMPTY_MAX};}c.hp[slot as usize]=hp.max(0);if before_sever.is_some_and(|before|sever>before)&&sever>0{c.sever[slot as usize]=sever;}}});
}

impl Part {
 pub fn secondary(&self)->bool{self.sever_required>0 && self.sever_base>0 && self.sever_base<65535}
 pub fn severable(&self)->bool{self.secondary()&&self.sever_type==1&&self.name.contains("Tail")}
 pub fn severed(&self)->bool{self.severable()&&u16::from(self.sever_count)>=self.sever_required}
 pub fn breakable(&self)->bool{self.groups!=0||self.secondary()}
 pub fn visible(&self,names:bool,break_hp:bool,all_hp:bool)->bool{self.active&&((all_hp&&(!self.name.is_empty()||self.breakable()||self.flinches>0))||((names||break_hp)&&self.breakable()))}
}
