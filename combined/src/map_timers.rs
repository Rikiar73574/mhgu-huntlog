//! Marker instance coordinates, read after the game's map transforms.
use crate::target::Memory;
fn pointer(p:u32)->bool {(0x08000000..0xa8000000).contains(&p)&&p&3==0}
pub fn instance(m:&impl Memory,controller:u32,id:u32)->Option<u32>{
 if !pointer(controller)||id>4095{return None;}
 let layout=m.u32(controller+0x24)?;if !pointer(layout){return None;}
 let split=m.u32(layout+0x298)?;
 let (table,index)=if id<split{(m.u32(layout+0xf8)?,id)}else{(m.u32(layout+0x294)?,id-split)};
 if !pointer(table)||index>4095{return None;}
 m.u32(table+index*4).filter(|p|pointer(*p))
}
pub fn position(m:&impl Memory,node:u32)->Option<(f32,f32)>{
 let mut p=node;
 for _ in 0..32 {
  if !pointer(p){return None;}
  // Bit 0 is visibility (a33e88); bit 1 is an override, not required.
  if m.u32(p+0x54)?&1==0{return None;}
  let parent=m.u32(p+0x68)?;
  if parent==0{
   let x=f32::from_bits(m.u32(node+0x40)?);let y=f32::from_bits(m.u32(node+0x44)?);
   return (x.is_finite()&&y.is_finite()&&(-100.0..1380.0).contains(&x)&&(-100.0..820.0).contains(&y)).then_some((x,y));
  }
  p=parent;
 }
 None
}
#[derive(Clone,Copy)]pub struct Marker{pub node:u32,pub object:u32,pub frame:u32}
impl Marker{pub const EMPTY:Self=Self{node:0,object:0,frame:0};}
