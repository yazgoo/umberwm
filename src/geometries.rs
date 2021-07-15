use crate::model::Geometry;

pub fn geometries_bsp(
    i: usize,
    window_count: usize,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    vertical: usize,
) -> Vec<Geometry> {
    if window_count == 0 {
        vec![]
    } else if window_count == 1 {
        vec![Geometry(left, top, width, height)]
    } else if i % 2 == vertical {
        let mut res = vec![Geometry(left, top, width, height / 2)];
        res.append(&mut geometries_bsp(
            i + 1,
            window_count - 1,
            left,
            top + height / 2,
            width,
            height / 2,
            vertical,
        ));
        res
    } else {
        let mut res = vec![Geometry(left, top, width / 2, height)];
        res.append(&mut geometries_bsp(
            i + 1,
            window_count - 1,
            left + width / 2,
            top,
            width / 2,
            height,
            vertical,
        ));
        res
    }
}
