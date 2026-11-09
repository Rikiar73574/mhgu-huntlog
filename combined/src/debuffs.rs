//! The status HUD cycles two icons; read the selected table entry after its update.
use crate::{target::Memory,map_timers};
fn pointer(p:u32)->bool{(0x08000000..0xa8000000).contains(&p)&&p&3==0}
#[derive(Clone,Copy)]pub struct Icon{pub node:u32,pub actor:u32,pub group:u8,pub low:u32,pub high:u32,pub frame:u32}
impl Icon{pub const EMPTY:Self=Self{node:0,actor:0,group:0,low:0,high:0,frame:0};}
pub fn capture(m:&impl Memory,base:u32,c:u32,frame:u32)->[Icon;2]{
 let mut out=[Icon::EMPTY;2];
 if !pointer(c){return out;}
 let Some(slot)=m.u32(c+0x4c).filter(|v|*v<4)else{return out;};
 // Palico timer layout has not been verified.
 let Some(kind)=m.u32(c+0x50).filter(|v|*v==0)else{return out;};
 let Some(manager)=m.u32(base+if kind==0{0x1885b58}else{0x1885468}).filter(|v|pointer(*v))else{return out;};
 let Some(actor)=m.u32(manager+0x1c+slot*4).filter(|v|pointer(*v))else{return out;};
 let Some(root_id)=m.u32(c+0x58)else{return out;};
 let Some(root)=map_timers::instance(m,c,root_id)else{return out;};
 let Some(children)=m.u32(root+0xd8).filter(|v|pointer(*v))else{return out;};
 let Some(row)=m.u32(c+0x54).filter(|v|*v<8)else{return out;};
 // The HUD row table lives in main's BSS, not the heap. HD UI textures
 // still use this live table; accepting only heap pointers hid every icon.
 let Some(rows)=m.u32(base+0x18402fc).filter(|v|v&3==0&&(pointer(*v)||(*v>=base&&v.checked_add(256).is_some_and(|end|base.checked_add(0x2200000).is_some_and(|limit|end<=limit)))))else{return out;};
 for (i,(record,table,count)) in [(0xf0,0x1639250,54),(0xfc,0x16395b0,36)].iter().enumerate(){
  if m.u8(c+record+8)!=Some(1){continue;}
  let Some(selected)=m.u32(c+record).filter(|v|*v<*count)else{continue;};
  let entry=base+table+selected*16;
  let Some(child)=m.u32(rows+row*32+8+i as u32*4).filter(|v|*v<4096)else{continue;};
  let Some(node)=m.u32(children+child*4).filter(|v|pointer(*v))else{continue;};
  if let (Some(group),Some(low),Some(high))=(m.u8(entry),m.u32(entry+8),m.u32(entry+12)){
   out[i]=Icon{node,actor,group,low,high,frame};
  }
 }
 out
}
// Verified against MHGU 1.4's flag constructor main+0x29edf4 (stores at
// 0x29f814/0x29f834), and countdown updater main+0x53aa84. A table entry
// spanning distinct timers has no single truthful remaining duration.
pub fn remaining(m:&impl Memory,icon:Icon)->Option<u32>{
 if !pointer(icon.actor){return None;}
 let mappings:&[(u32,u32,u32)]=match icon.group{
  0=>&[
   // Stench: main+0x29f358 tests the remaining timer via main+0x2f5508.
   (0x800,0,0x28dc),
   (0x2000,0,0x13ec),(0x4000,0,0x2820),(0x20000,0,0x2834),
   (0,0x8,0x27c0),(0,0x10,0x27c4),(0,0x20,0x27cc),
   (0,0x40,0x27d0),(0,0x80,0x27d4),(0,0x100,0x282c),
   (0,0x200,0x277c),(0,0x1000,0x27d8),(0,0x2000,0x2b34),
   (0,0x8000,0x2814),(0,0x10000,0x2818),(0,0x20000,0x281c),
   (0,0x400000,0x2b38),(0,0x800000,0x27e8),
   // 0x1000000 alone also represents another condition (0x27f4).
   (0,0x6000000,0x27f0),(0,0x8000000,0x27ec),
   (0,0x10000000,0x27fc),(0,0x20000000,0x29d8),(0,0x40000000,0x2b3c),
  ],
  // main+0x29d7a8 decrements these five elemental-blight timers;
  // each pair of flags is the minor/major form of the same blight.
  2=>&[(0x3,0,0x28ec),(0xc,0,0x28f4),(0x30,0,0x28f8),
        (0xc0,0,0x28fc),(0x300,0,0x2900),
        (0x400,0,0x28c0),(0x800,0,0x28c4),(0x2000,0,0x29c4),
        (0x1c000,0,0x2964),(0x4000000,0,0x2988),(0x8000000,0,0x2990)],
  _=>return None,
 };
 let mut offset=None;
 for &(low,high,field) in mappings{
  if icon.low&low==0&&icon.high&high==0{continue;}
  if offset.is_some_and(|old|old!=field){return None;}
  offset=Some(field);
 }
 let frames=f32::from_bits(m.u32(icon.actor.checked_add(offset?)?)?);
 if !frames.is_finite()||frames<=0.0||frames>216000.0{return None;}
 let seconds=frames/60.0;
 let whole=seconds as u32;
 Some(whole+u32::from(seconds>whole as f32))
}

// Flag builder main+29edf4: 2840 bits 0x200/0x400 are Cool/Hot Drink.
pub fn drink(icon:Icon)->Option<usize>{
 if icon.group!=0||icon.high!=0{return None;}
 match icon.low {0x200=>Some(0),0x400=>Some(1),_=>None}
}
pub fn drink_remaining(m:&impl Memory,base:u32,icon:Icon)->Option<u32>{
 let index=drink(icon)?;
 crate::timers::drinks(m,icon.actor,crate::timers::temperature(m,base,icon.actor))[index]
}
