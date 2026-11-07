//! Read-only size/crown decoding from MHGU 1.4.0's own size table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size { pub value: f32, pub crown: u8 }
// Binary evidence (offsets from main text, MHGU 1.4.0):
// 0x50b64 translates 137 species/index pairs at 0x1597ea4.
// 0x67310 uses sEnemy+0x2584, resource+0x64 count/+0x6c rows,
// stride 0x20, +4 index, +8 base size, +c/+10/+14 crown ratios.
// 0x3a61b4..d0 quantizes live scale as trunc((scale+0.001)*100).
// 0x67384..418 computes displayed size and thresholds with +0.005.
// Crown values: 3 small gold, 1 large gold, 2 large silver, 255 none.
// This returns the game's displayed size; a physical unit is not asserted.
pub fn capture(m: &impl crate::target::Memory, base: u32, enemy_manager: u32,
               species: u16, scale: f32) -> Option<Size> {
    if !scale.is_finite() || scale <= 0.0 { return None; }
    let quantized = (scale + 0.001f32) * 100.0f32;
    if !quantized.is_finite() || quantized < 1.0 || quantized >= u32::MAX as f32 { return None; }
    let quantized = quantized as u32;
    let mapping = base.checked_add(0x1597ea4)?;
    let mut index = None;
    for i in 0..137u32 {
        let pair = mapping.checked_add(i * 8)?;
        if m.u32(pair)? == u32::from(species) {
            index = Some(m.u32(pair.checked_add(4)?)?); break;
        }
    }
    let index = index?;
    let table = m.u32(enemy_manager.checked_add(0x2584)?)?;
    if table == 0 || table & 3 != 0 { return None; }
    let count = m.u32(table.checked_add(0x64)?)?;
    if count == 0 || count > 1024 { return None; }
    let rows = m.u32(table.checked_add(0x6c)?)?;
    if rows == 0 || rows & 3 != 0 { return None; }
    for i in 0..count {
        let row = rows.checked_add(i * 0x20)?;
        if m.u32(row.checked_add(4)?)? != index { continue; }
        let float = |offset| -> Option<f32> {
            let value = f32::from_bits(m.u32(row.checked_add(offset)?)?);
            (value.is_finite() && value >= 0.0).then_some(value)
        };
        let base_size = float(8)?;
        if base_size <= 0.0 { return None; }
        let threshold = |offset| -> Option<u32> {
            let value = (float(offset)? + 0.005f32) * 100.0f32;
            (value < u32::MAX as f32).then_some(value as u32)
        };
        let small = threshold(0xc)?;
        let silver = threshold(0x10)?;
        let gold = threshold(0x14)?;
        if small > silver || silver > gold { return None; }
        let value = (quantized as f32 / 100.0f32) * base_size;
        if !value.is_finite() { return None; }
        let crown = if quantized <= small { 3 } else if quantized >= gold { 1 }
                    else if quantized >= silver { 2 } else { 255 };
        return Some(Size { value, crown });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::Memory;
    use std::collections::BTreeMap;
    struct Mem(BTreeMap<u32,u32>);
    impl Memory for Mem {
        fn u8(&self,_:u32)->Option<u8>{None}
        fn u16(&self,_:u32)->Option<u16>{None}
        fn u32(&self,a:u32)->Option<u32>{self.0.get(&a).copied()}
    }
    const BASE:u32=0x200000;
    const MANAGER:u32=0x10000000;
    const TABLE:u32=0x11000000;
    const ROW:u32=0x12000000;
    fn fixture()->Mem {
        let mut m=Mem(BTreeMap::new());
        for (a,v) in [(BASE+0x1597ea4,1),(BASE+0x1597ea8,0),(MANAGER+0x2584,TABLE),(TABLE+0x64,1),(TABLE+0x6c,ROW),(ROW+4,0)] {m.0.insert(a,v);}
        for (offset,value) in [(8,1000.0f32),(12,0.90),(16,1.10),(20,1.20)] {m.0.insert(ROW+offset,value.to_bits());}
        m
    }
    #[test] fn game_crown_boundaries_and_size() {
        let m=fixture();
        for (scale,value,crown) in [(0.90,900.0,3),(0.91,910.0,255),(1.0,1000.0,255),(1.10,1100.0,2),(1.20,1200.0,1)] {
            let s=capture(&m,BASE,MANAGER,1,scale).unwrap();
            assert!((s.value-value).abs()<0.001);assert_eq!(s.crown,crown);
        }
    }
    #[test] fn live_quantization_is_not_threshold_rounding() {
        let m=fixture();
        // Game's live +0.001 then trunc differs from rounding to nearest percent.
        assert_eq!(capture(&m,BASE,MANAGER,1,1.195).unwrap().crown,2);
        assert_eq!(capture(&m,BASE,MANAGER,1,0.909).unwrap().crown,255);
    }
    #[test] fn variant_mapping_and_table_row_order_are_independent() {
        let mut m=fixture();
        m.0.insert(BASE+0x1597eac,513);m.0.insert(BASE+0x1597eb0,1);
        m.0.insert(TABLE+0x64,2);
        for (offset,value) in [(4,1),(8,2000.0f32.to_bits()),(12,0.8f32.to_bits()),(16,1.15f32.to_bits()),(20,1.25f32.to_bits())] {
            m.0.insert(ROW+0x20+offset,value);
        }
        let size=capture(&m,BASE,MANAGER,513,1.20).unwrap();
        assert_eq!(size.crown,2);assert!((size.value-2400.0).abs()<0.001);
        m.0.insert(TABLE+0x64,1025);
        assert!(capture(&m,BASE,MANAGER,513,1.20).is_none());
    }
    #[test] fn unavailable_and_bad_data_are_unknown() {
        let mut m=fixture();
        assert!(capture(&m,BASE,MANAGER,2,1.0).is_none());
        for scale in [f32::NAN,f32::INFINITY,0.0,-1.0] {assert!(capture(&m,BASE,MANAGER,1,scale).is_none());}
        m.0.insert(TABLE+0x64,0);assert!(capture(&m,BASE,MANAGER,1,1.0).is_none());
        let mut m=fixture();m.0.insert(ROW+8,(-1.0f32).to_bits());assert!(capture(&m,BASE,MANAGER,1,1.0).is_none());
        let mut m=fixture();m.0.remove(&(ROW+20));assert!(capture(&m,BASE,MANAGER,1,1.0).is_none());
        assert!(capture(&fixture(),u32::MAX,MANAGER,1,1.0).is_none());
    }
}
