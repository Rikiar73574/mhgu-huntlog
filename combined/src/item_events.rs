//! Count actual mutations inside the local pouch operation, not periodic deltas.
use crate::target::Memory;
pub fn count(m:&impl Memory,pouch:u32,id:u16)->Option<i32>{
 if !(0x08000000..0xa8000000).contains(&pouch)||id==0{return None;}
 let mut total=0;
 // All slots: virtual entries are unchanged by real item mutations, so cancel
 // in before/after counts. Never log the starting contents as acquisitions.
 for slot in 0..50{let p=pouch+0x1a+slot*4;let item=m.u16(p)?;let n=m.u16(p+2)? as i16;if item==id&&n>0{total+=i32::from(n);}}
 Some(total)
}
pub fn action(delta:i32,caller:u32,context:u32)->&'static str{
 if delta<0&&matches!(context,0x2fb098|0x2fb8e8){"used"}
 else if delta>0&&context==0x2e0884{"gathered"}
 else if delta>0{"acquired_unknown"}
 else if matches!(caller,0x30163c)&&context!=0{"action_consumed"}
 else{"removed_unknown"}
}
#[cfg(test)]mod tests{use super::*;#[test]fn only_proven_action_is_usage(){assert_eq!(action(-1,0x30163c,0x2fb098),"used");assert_eq!(action(-1,0,0),"removed_unknown");assert_eq!(action(4,0,0x2e0884),"gathered");assert_eq!(action(3,0,0),"acquired_unknown");}}
