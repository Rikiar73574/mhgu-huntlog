//! MHGU 1.4.0 uEm057_00 charge points; see CHARGE-RESEARCH.md.
use crate::target::Memory;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Snapshot{pub active:bool,pub points:Option<u8>}
fn pointer(p:u32)->bool{(0x08000000..0xa8000000).contains(&p)&&p&3==0}
pub fn capture(m:&impl Memory,base:u32,object:u32,state:u32,species:u16)->Option<Snapshot>{
 if species!=57||!pointer(object)||!pointer(state)||m.u32(object)?!=base.checked_add(0x17d19bc)?||m.u32(object.checked_add(0x1428)?)?!=state{return None;}
 let active=match m.u8(state.checked_add(0x5df3)?)?{0=>false,1=>true,_=>return None};
 // The AI owner's local meter is authoritative. Replication of this custom
 // field is not established; a guest zero must not look like an empty charge.
 let points=if m.u8(object.checked_add(0x7420)?)==Some(1){
  m.u32(object.checked_add(0xcac0)?).filter(|p|pointer(*p)).and_then(|p|m.u16(p)).filter(|v|*v<=100).map(|v|v as u8)
 }else{None};
 Some(Snapshot{active,points})
}
