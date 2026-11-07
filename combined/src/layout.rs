//! Measure and fit the complete panel before placing its bottom edge.
pub fn top(screen_height:f32,height:f32,bottom:f32)->f32 {
 screen_height-bottom-height
}
pub fn fit_scale(screen_width:f32,screen_height:f32,width:f32,height:f32,bottom:f32)->f32 {
 ((screen_width-8.0).max(1.0)/width).min((screen_height-8.0).max(1.0)/(height+bottom)).min(1.0)
}
#[cfg(test)] mod tests {use super::*;
 #[test] fn content_grows_up_from_same_bottom(){
  for screen in [720.0,1080.0,1440.0] {for height in [48.0,120.0,500.0] {
   assert_eq!(top(screen,height,12.0)+height,screen-12.0);
  }}
 }
 #[test] fn oversized_panel_fits_without_moving_bottom_offscreen(){
  let f=fit_scale(640.0,480.0,700.0,900.0,12.0);
  let (h,b)=(900.0*f,12.0*f);
  assert!(top(480.0,h,b)>=7.9);
  assert!((top(480.0,h,b)+h-(480.0-b)).abs()<0.001);
  assert!(700.0*f<=632.0);
 }
 #[test] fn normal_panel_keeps_scale(){assert_eq!(fit_scale(1920.0,1080.0,320.0,120.0,12.0),1.0);}
}
