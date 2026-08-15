use crate::infrastructure::pdf::models::PathSegment;

pub fn simplify_path_segments(segments: Vec<PathSegment>, epsilon: f32) -> Vec<PathSegment> {
    if segments.is_empty() {
        return segments;
    }
    let mut result = Vec::with_capacity(segments.len());
    let mut current_poly: Vec<[f32; 2]> = Vec::new();

    let flush_poly = |poly: &mut Vec<[f32; 2]>, res: &mut Vec<PathSegment>| {
        if poly.is_empty() {
            return;
        }
        if poly.len() > 2 {
            let simplified = simplify_points(poly, epsilon);
            for (i, pt) in simplified.into_iter().enumerate() {
                res.push(PathSegment {
                    command: if i == 0 { "move".into() } else { "line".into() },
                    points: vec![pt],
                });
            }
        } else {
            for (i, pt) in poly.drain(..).enumerate() {
                res.push(PathSegment {
                    command: if i == 0 { "move".into() } else { "line".into() },
                    points: vec![pt],
                });
            }
        }
        poly.clear();
    };

    for seg in segments {
        if seg.command == "move" || seg.command == "line" {
            current_poly.push(seg.points[0]);
        } else {
            flush_poly(&mut current_poly, &mut result);
            result.push(seg);
        }
    }
    flush_poly(&mut current_poly, &mut result);
    result
}

fn simplify_points(points: &[[f32; 2]], epsilon: f32) -> Vec<[f32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut dmax = 0.0;
    let mut index = 0;
    let end = points.len() - 1;
    for i in 1..end {
        let d = perpendicular_distance(points[i], points[0], points[end]);
        if d > dmax {
            index = i;
            dmax = d;
        }
    }
    if dmax > epsilon {
        let mut res1 = simplify_points(&points[0..=index], epsilon);
        let mut res2 = simplify_points(&points[index..=end], epsilon);
        res1.pop();
        res1.append(&mut res2);
        res1
    } else {
        vec![points[0], points[end]]
    }
}

fn perpendicular_distance(p: [f32; 2], p1: [f32; 2], p2: [f32; 2]) -> f32 {
    let [x, y] = p;
    let [x1, y1] = p1;
    let [x2, y2] = p2;
    let dx = x2 - x1;
    let dy = y2 - y1;
    let den = (dy * dy + dx * dx).sqrt();
    if den < 0.0001 {
        return ((x - x1).powi(2) + (y - y1).powi(2)).sqrt();
    }
    (dy * x - dx * y + x2 * y1 - y2 * x1).abs() / den
}
