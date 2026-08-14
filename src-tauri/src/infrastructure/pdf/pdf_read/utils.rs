use lopdf::Object;

pub fn operands_to_f32(ops: &[Object]) -> Result<Vec<f32>, String> {
    let mut res = Vec::new();
    for op in ops {
        if let Ok(f) = op.as_float() {
            res.push(f);
        } else if let Ok(i) = op.as_i64() {
            res.push(i as f32);
        }
    }
    Ok(res)
}

pub fn multiply_matrices(current: [f32; 6], new: [f32; 6]) -> [f32; 6] {
    let (a1, b1, c1, d1, e1, f1) = (new[0], new[1], new[2], new[3], new[4], new[5]);
    let (a2, b2, c2, d2, e2, f2) = (
        current[0], current[1], current[2], current[3], current[4], current[5],
    );
    [
        a1 * a2 + b1 * c2,
        a1 * b2 + b1 * d2,
        c1 * a2 + d1 * c2,
        c1 * b2 + d1 * d2,
        e1 * a2 + f1 * c2 + e2,
        e1 * b2 + f1 * d2 + f2,
    ]
}
