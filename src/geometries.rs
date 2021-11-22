use crate::model::{Geometry, GeometryInfo};

pub fn with_quota(quota: f32, size: u32) -> u32 {
    (size as f32 * quota) as u32
}

pub fn geometries_bsp(
    i: usize,
    window_count: usize,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    info: &GeometryInfo,
) -> Vec<Geometry> {
    if window_count == 0 {
        vec![]
    } else if window_count == 1 {
        vec![Geometry(left, top, width, height)]
    } else if i % 2 == info.vertical {
        let quota_height = with_quota(info.quota, height);
        let mut res = vec![Geometry(left, top, width, quota_height)];
        res.append(&mut geometries_bsp(
            i + 1,
            window_count - 1,
            left,
            top + quota_height,
            width,
            height - quota_height,
            info,
        ));
        res
    } else {
        let quota_width = with_quota(info.quota, width);
        let mut res = vec![Geometry(left, top, quota_width, height)];
        res.append(&mut geometries_bsp(
            i + 1,
            window_count - 1,
            left + quota_width,
            top,
            width - quota_width,
            height,
            info,
        ));
        res
    }
}
