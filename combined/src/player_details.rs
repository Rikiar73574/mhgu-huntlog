//! Verified cEquip, cSkillParam and cFoodSkill layout (MHGU 1.4).
use core::fmt::Write;
use crate::target::Memory;
fn pointer(p:u32)->bool{(0x08000000..0xa8000000).contains(&p)&&p&3==0}
fn value(w:&mut impl Write,v:Option<u16>){if let Some(v)=v{let _=write!(w,"{}",v as i16);}else{let _=w.write_str("null");}}
pub fn append(w:&mut impl Write,m:&impl Memory,pd:u32){
 if !pointer(pd){let _=w.write_str(",\"equipment\":null,\"stats\":null,\"skills\":null,\"food_skills\":null");return;}
 let _=w.write_str(",\"equipment\":[");
 for slot in 0..7u32 {
  if slot>0{let _=w.write_char(',');}
  let p=pd+0x18+slot*0x2c;
  if let (Some(bits),Some(id))=(m.u16(p+4),m.u16(p+6)){
   let kind=bits&31;
   if kind<=19&&id<10000{
    let _=write!(w,"{{\"slot\":{},\"type\":{},\"id\":{},\"level\":{}",slot,kind,id,((bits>>5)&31)+1);
    let _=w.write_str(",\"decorations\":");
    if let (Some(a),Some(b),Some(c))=(m.u16(p+0xa),m.u16(p+0xc),m.u16(p+0xe)){
     let _=w.write_char('[');let mut n=0;for id in [a,b,c]{if id!=0{if n>0{let _=w.write_char(',');}n+=1;let _=write!(w,"{}",id);}}let _=w.write_char(']');
    }else{let _=w.write_str("null");}
    let _=w.write_str(",\"skill_points\":");
    if let Some(entries)=m.u32(p+0x28).filter(|p|pointer(*p)){
     let mut values=[(0u16,0i8);11];let mut valid=true;
     for i in 0..11{if let (Some(id),Some(points))=(m.u16(entries+i as u32*4),m.u8(entries+i as u32*4+2)){values[i]=(id,points as i8);}else{valid=false;}}
     if valid{let _=w.write_char('[');let mut n=0;for(id,points)in values{if id!=0{if n>0{let _=w.write_char(',');}n+=1;let _=write!(w,"{{\"tree\":{},\"points\":{}}}",id,points);}}let _=w.write_char(']');}else{let _=w.write_str("null");}
    }else{let _=w.write_str("null");}
    let _=w.write_char('}');
   }else{let _=w.write_str("null");}
  }else{let _=w.write_str("null");}
 }
 let _=w.write_str("],\"stats\":{\"attack\":");value(w,m.u16(pd+0x33e));
 let _=w.write_str(",\"defense\":");value(w,m.u16(pd+0x34a));
 let _=w.write_str(",\"health\":");value(w,m.u16(pd+0x338));
 let _=w.write_str(",\"resistances\":[");for i in 0..5{if i>0{let _=w.write_char(',');}value(w,m.u16(pd+0x34c+i*2));}
 let _=w.write_str("]},\"skills\":");
 if let Some(p)=m.u32(pd+0x18c).filter(|p|pointer(*p)){
  let mut n=0;let _=w.write_char('[');
  for i in 0..20{if let Some(id)=m.u16(p+0x3a0+i*2).filter(|id|*id>0&&*id<328){if n>0{let _=w.write_char(',');}n+=1;let _=write!(w,"{}",id);}}
  let _=w.write_char(']');
 }else{let _=w.write_str("null");}
 let _=w.write_str(",\"food_skills\":");
 match m.u16(pd+0x504){
 Some(flags) if flags&4==0=>{let _=w.write_str("[]");},
 Some(_)=>if let Some(p)=m.u32(pd+0x510).filter(|p|pointer(*p)){
  let mut n=0;let _=w.write_char('[');for i in 0..3{if let Some(id)=m.u8(p+0xd+i).filter(|id|*id>0&&*id<66){if n>0{let _=w.write_char(',');}n+=1;let _=write!(w,"{}",id);}}let _=w.write_char(']');
 }else{let _=w.write_str("null");},
 None=>{let _=w.write_str("null");}
 }
}
#[cfg(test)]mod tests{
 use super::*;
 struct Mem{data:Vec<u8>,extra:std::collections::BTreeMap<u32,u8>}
 const PD:u32=0x08010000;
 impl Memory for Mem{
 fn u8(&self,a:u32)->Option<u8>{self.extra.get(&a).copied().or_else(||a.checked_sub(PD).and_then(|i|self.data.get(i as usize).copied()))}
 fn u16(&self,a:u32)->Option<u16>{Some(u16::from_le_bytes([self.u8(a)?,self.u8(a+1)?]))}
 fn u32(&self,a:u32)->Option<u32>{Some(u32::from_le_bytes([self.u8(a)?,self.u8(a+1)?,self.u8(a+2)?,self.u8(a+3)?]))}
 }
 impl Mem{fn put(&mut self,a:u32,v:u32,n:usize){for(i,b)in v.to_le_bytes()[..n].iter().enumerate(){self.extra.insert(a+i as u32,*b);}}}
 #[test]fn controlled_meal_snapshot_preserves_equipment_levels_and_signed_stats(){
 // Synthetic fixture: never ship a player's captured memory or save data.
 let mut m=Mem{data:vec![0;0x600],extra:Default::default()};
 m.put(PD+0x18+4,10,2);m.put(PD+0x18+6,18,2);
 m.put(PD+0x18+3*0x2c+4,3|(4<<5),2);m.put(PD+0x18+3*0x2c+6,44,2);
 for(offset,value)in [(0x33e,98u16),(0x34a,119),(0x338,110),(0x34c,6),(0x34e,2),(0x350,(-4i16)as u16),(0x352,(-4i16)as u16),(0x354,0)]{m.put(PD+offset,value as u32,2);}
 let mut s=String::new();append(&mut s,&m,PD);
 assert!(s.contains("\"slot\":0,\"type\":10,\"id\":18,\"level\":1"));
 assert!(s.contains("\"slot\":3,\"type\":3,\"id\":44,\"level\":5"));
 assert!(s.contains("\"attack\":98,\"defense\":119,\"health\":110,\"resistances\":[6,2,-4,-4,0]"));
 }
 #[test]fn equipment_records_decorations_and_signed_skill_points(){
 let mut m=Mem{data:vec![0;0x600],extra:Default::default()};let p=PD+0x18;let entries=0x09002000;
 m.put(p+0xa,2638,2);m.put(p+0xc,2640,2);m.put(p+0x28,entries,4);
 for i in 0..11{m.put(entries+i*4,0,4);}m.put(entries,25,2);m.put(entries+2,255,1);m.put(entries+4,35,2);m.put(entries+6,3,1);
 let mut text=String::new();append(&mut text,&m,PD);
 assert!(text.contains("\"decorations\":[2638,2640]"));assert!(text.contains("\"skill_points\":[{\"tree\":25,\"points\":-1},{\"tree\":35,\"points\":3}]"));
 }
 #[test]fn reads_active_skill_and_food_arrays_not_stat_change_flag(){
 let mut m=Mem{data:vec![0;0x600],extra:Default::default()};m.put(PD+0x18c,0x09000000,4);m.put(0x090003a0,10,2);m.put(0x090003a2,329,2);m.put(PD+0x504,4,2);m.put(PD+0x510,0x09001000,4);m.put(0x0900100d,4,1);
 let mut s=String::new();append(&mut s,&m,PD);assert!(s.contains("\"skills\":[10],\"food_skills\":[4]"));
 m.put(PD+0x504,0,2);s.clear();append(&mut s,&m,PD);assert!(s.ends_with("\"food_skills\":[]"));
 s.clear();append(&mut s,&m,0);assert!(s.contains("\"skills\":null"));
 }
}
