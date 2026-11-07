//! uUIResult reward lists, captured before the game consumes their entries.
//! Main+0x386c6c displays them through cUIOResultRem (main+0x623c08).
use crate::target::Memory;
pub const SLOTS:usize=48;
pub const GROUPS:usize=4;
#[derive(Clone,Copy,Default,Debug,PartialEq)]
pub struct Item{pub id:u16,pub quantity:u16}
impl Item{pub const EMPTY:Self=Self{id:0,quantity:0};}
#[derive(Clone,Copy)]
pub struct Snapshot{pub items:[[Item;SLOTS];GROUPS]}
impl Snapshot{pub const EMPTY:Self=Self{items:[[Item::EMPTY;SLOTS];GROUPS]};}
pub fn capture(m:&impl Memory,object:u32)->Snapshot{
 let mut out=Snapshot::EMPTY;
 for group in 0..GROUPS{
  let vector=object.wrapping_add(0x5c+group as u32*8);
  let Some(count)=m.u32(vector+4).filter(|n|*n<=SLOTS as u32) else{continue};
  let Some(data)=m.u32(vector).filter(|p|(0x08000000..0xa8000000).contains(p)&&p&3==0) else{continue};
  for slot in 0..count as usize{
   let p=data+slot as u32*4;
   if let (Some(id),Some(quantity))=(m.u16(p),m.u16(p+2)){
    if id>0 && id<20000 && quantity>0 && quantity<=999{out.items[group][slot]=Item{id,quantity};}
   }
  }
 }
 out
}
/// Keep the first observed award in each slot. Taking/selling/page redraws do
/// not create new awards. Never infer quantities from the player's item box.
pub struct Tracker{seen:[[bool;SLOTS];GROUPS]}
impl Tracker{
 pub const fn new()->Self{Self{seen:[[false;SLOTS];GROUPS]}}
 pub fn observe(&mut self,s:&Snapshot,mut emit:impl FnMut(usize,usize,Item)){
  for g in 0..GROUPS{for i in 0..SLOTS{let item=s.items[g][i];if item.id!=0&&!self.seen[g][i]{self.seen[g][i]=true;emit(g,i,item);}}}
 }
}
#[cfg(test)]mod tests{
 use super::*;
 struct Mem(std::collections::BTreeMap<u32,u32>);
 impl Memory for Mem{fn u8(&self,a:u32)->Option<u8>{self.0.get(&a).map(|v|*v as u8)}fn u16(&self,a:u32)->Option<u16>{self.0.get(&a).map(|v|*v as u16)}fn u32(&self,a:u32)->Option<u32>{self.0.get(&a).copied()}}
 #[test]fn reads_independent_reward_lists_and_rejects_bad_counts(){let mut m=Mem(std::collections::BTreeMap::from([(0x1000+0x5c,0x08001000),(0x1000+0x60,2),(0x08001000,576),(0x08001002,2),(0x08001004,407),(0x08001006,1)]));let s=capture(&m,0x1000);assert_eq!(s.items[0][0],Item{id:576,quantity:2});assert_eq!(s.items[0][1].id,407);assert_eq!(s.items[1][0],Item::EMPTY);m.0.insert(0x1060,1000);assert_eq!(capture(&m,0x1000).items[0][0],Item::EMPTY);m.0.insert(0x1060,2);m.0.insert(0x08001002,65535);assert_eq!(capture(&m,0x1000).items[0][0],Item::EMPTY);}

 #[test]fn redraw_and_consumption_do_not_duplicate(){let mut t=Tracker::new();let mut s=Snapshot::EMPTY;s.items[0][0]=Item{id:576,quantity:2};s.items[0][1]=Item{id:576,quantity:1};let mut rows=Vec::new();t.observe(&s,|g,i,v|rows.push((g,i,v)));t.observe(&s,|g,i,v|rows.push((g,i,v)));s.items[0][0]=Item::EMPTY;t.observe(&s,|g,i,v|rows.push((g,i,v)));assert_eq!(rows.len(),2);assert_eq!(rows.iter().map(|x|x.2.quantity).sum::<u16>(),3);}
 #[test]fn late_groups_and_new_run(){let mut s=Snapshot::EMPTY;let mut t=Tracker::new();let mut n=0;t.observe(&s,|_,_,_|n+=1);s.items[2][0]=Item{id:407,quantity:1};t.observe(&s,|_,_,_|n+=1);assert_eq!(n,1);let mut next=Tracker::new();next.observe(&s,|_,_,_|n+=1);assert_eq!(n,2);}
}
