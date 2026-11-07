//! Read-only MHGU 1.4 effect timers. See docs/timers-plan.md for code traces.
use crate::target::Memory;
fn pointer(p:u32)->bool{(0x08000000..0xa8000000).contains(&p)&&p&3==0}
fn seconds(m:&impl Memory,a:u32,multiplier:f32)->Option<u32>{
 let frames=f32::from_bits(m.u32(a)?);
 if !frames.is_finite()||frames<=0.0||frames>216000.0{return None;}
 let seconds=frames/(60.0*multiplier);
 let whole=seconds as u32;
 Some(whole+u32::from(seconds>whole as f32))
}
pub fn compatible(m:&impl Memory,base:u32)->bool{
 [(0x2a6da8,0xed800a00),(0x2a6de8,0xed800a00),(0xc0928,0xe58121c4),
  (0x29d980,0xeeb70a08),(0x29d9e8,0xeeb70a08),(0x4f9058,0xe0811081),(0x500858,0xe5d0000e)]
 .iter().all(|&(offset,word)|base.checked_add(offset).and_then(|a|m.u32(a))==Some(word))
}
pub fn local_player(m:&impl Memory,base:u32)->Option<u32>{
 let context=m.u32(base.checked_add(0x188ca48)?)?;if !pointer(context){return None;}
 let slot=m.u32(context+0x30)?;if slot>=4{return None;}
 let manager=m.u32(base.checked_add(0x1885b58)?)?;if !pointer(manager){return None;}
 m.u32(manager+0x1c+4*slot).filter(|p|pointer(*p))
}
pub fn temperature(m:&impl Memory,base:u32,player:u32)->Option<u8>{
 if !pointer(player){return None;}
 let stage=m.u32(base.checked_add(0x18977a0)?)?;if !pointer(stage){return None;}
 let map=m.u32(stage+0x3bc)?;if !pointer(map){return None;}
 let room=u32::from(m.u8(player+0x1054)?);
 let count=m.u32(map+0x64)?;if count>256||room>=count{return None;}
 let rows=m.u32(map+0x6c)?;if !pointer(rows){return None;}
 m.u8(rows.checked_add(room*24+14)?)
}
pub fn drinks(m:&impl Memory,player:u32,temperature:Option<u8>)->[Option<u32>;2]{
 if !pointer(player){return [None,None];}
 let Some(t)=temperature else{return [None,None]};
 [seconds(m,player+0x287c,if t==2{1.5}else{1.0}),seconds(m,player+0x2880,if t==4{1.5}else{1.0})]
}
pub fn paint(m:&impl Memory,object:u32)->Option<u32>{
 if !pointer(object){return None;}
 let state=m.u32(object+0x1428)?;if !pointer(state){return None;}
 seconds(m,state+0x1c4,1.0)
}
#[derive(Clone,Copy)]pub struct Paint{pub seconds:u32,pub species:u16,pub revealed:bool,pub number:u8}
impl Paint{pub const EMPTY:Self=Self{seconds:0,species:0,revealed:false,number:0};}
#[derive(Clone,Copy)]pub struct Snapshot{pub drinks:[Option<u32>;2],pub paint:[Paint;10],pub count:usize}
impl Snapshot{pub const EMPTY:Self=Self{drinks:[None,None],paint:[Paint::EMPTY;10],count:0};}
