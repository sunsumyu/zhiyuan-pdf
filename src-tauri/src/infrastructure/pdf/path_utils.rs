use vello::kurbo::{BezPath, Point};

/// Convert a swash `Outline` into a vello `BezPath`.
pub fn outline_to_bez_path(outline: &swash::scale::outline::Outline) -> BezPath {
    use swash::zeno::Verb;
    let mut bez_path = BezPath::new();
    let mut points = outline.points().iter();
    for verb in outline.verbs() {
        match verb {
            Verb::MoveTo => {
                if let Some(p) = points.next() {
                    bez_path.move_to(Point::new(p.x as f64, p.y as f64));
                }
            }
            Verb::LineTo => {
                if let Some(p) = points.next() {
                    bez_path.line_to(Point::new(p.x as f64, p.y as f64));
                }
            }
            Verb::QuadTo => {
                if let (Some(c), Some(p)) = (points.next(), points.next()) {
                    bez_path.quad_to(
                        Point::new(c.x as f64, c.y as f64),
                        Point::new(p.x as f64, p.y as f64),
                    );
                }
            }
            Verb::CurveTo => {
                if let (Some(c1), Some(c2), Some(p)) = (points.next(), points.next(), points.next())
                {
                    bez_path.curve_to(
                        Point::new(c1.x as f64, c1.y as f64),
                        Point::new(c2.x as f64, c2.y as f64),
                        Point::new(p.x as f64, p.y as f64),
                    );
                }
            }
            Verb::Close => bez_path.close_path(),
        }
    }
    bez_path
}

/// Convert a `PathSegment` list into a vello `BezPath`.
pub fn path_segments_to_bez_path(
    segments: &[crate::infrastructure::pdf::models::PathSegment],
) -> BezPath {
    let mut bez_path = BezPath::new();
    for seg in segments {
        match seg.command.as_str() {
            "move" => {
                if let Some(p) = seg.points.get(0) {
                    bez_path.move_to(Point::new(p[0] as f64, p[1] as f64));
                }
            }
            "line" => {
                if let Some(p) = seg.points.get(0) {
                    bez_path.line_to(Point::new(p[0] as f64, p[1] as f64));
                }
            }
            "bezier" => {
                if seg.points.len() == 3 {
                    bez_path.curve_to(
                        Point::new(seg.points[0][0] as f64, seg.points[0][1] as f64),
                        Point::new(seg.points[1][0] as f64, seg.points[1][1] as f64),
                        Point::new(seg.points[2][0] as f64, seg.points[2][1] as f64),
                    );
                }
            }
            "close" => bez_path.close_path(),
            _ => {}
        }
    }
    bez_path
}
