//! 动态占用、邻域、寻路与视野。

mod navigation;
mod neighborhood;
mod occupancy;
mod visibility;

pub(crate) use navigation::{
    facing_toward, is_adjacent_to_footprint, is_mobile, manhattan, nearest_adjacent_to_footprint, turn_facing_toward,
};
