fn catmull_rom(p0: vec4<f32>, p1: vec4<f32>, p2: vec4<f32>, p3: vec4<f32>, t: f32) -> vec4<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    return 0.5 * (
        (2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
    );
}

fn mitchell_weight(x: f32) -> f32 {
    let b = 1.0 / 3.0;
    let c = 1.0 / 3.0;
    let ax = abs(x);
    if (ax < 1.0) {
        return ((12.0 - 9.0 * b - 6.0 * c) * ax * ax * ax
            + (-18.0 + 12.0 * b + 6.0 * c) * ax * ax
            + (6.0 - 2.0 * b)) / 6.0;
    }
    if (ax < 2.0) {
        return ((-b - 6.0 * c) * ax * ax * ax
            + (6.0 * b + 30.0 * c) * ax * ax
            + (-12.0 * b - 48.0 * c) * ax
            + (8.0 * b + 24.0 * c)) / 6.0;
    }
    return 0.0;
}

fn texel_fetch(tex: texture_2d<f32>, coord: vec2<i32>, size: vec2<i32>) -> vec4<f32> {
    let max_coord = max(vec2<i32>(0, 0), size - vec2<i32>(1, 1));
    let c = clamp(coord, vec2<i32>(0, 0), max_coord);
    let uv = (vec2<f32>(c) + vec2<f32>(0.5, 0.5)) / vec2<f32>(size);
    return textureSampleLevel(tex, s_linear, uv, 0.0);
}

fn texel_fetch_clamped(
    tex: texture_2d<f32>,
    coord: vec2<i32>,
    clamp_min: vec2<i32>,
    clamp_max: vec2<i32>,
    size: vec2<i32>,
) -> vec4<f32> {
    let c = clamp(coord, clamp_min, clamp_max);
    let uv = (vec2<f32>(c) + vec2<f32>(0.5, 0.5)) / vec2<f32>(size);
    return textureSampleLevel(tex, s_linear, uv, 0.0);
}

fn sample_mitchell(tex: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    let size_u = textureDimensions(tex);
    let size_i = vec2<i32>(size_u);
    let size_f = vec2<f32>(size_u);
    let uv_clamped = clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    let texel = uv_clamped * size_f - vec2<f32>(0.5, 0.5);
    let base_f = floor(texel);
    let base = vec2<i32>(base_f);
    let f = texel - base_f;

    let wx0 = mitchell_weight(1.0 + f.x);
    let wx1 = mitchell_weight(f.x);
    let wx2 = mitchell_weight(1.0 - f.x);
    let wx3 = mitchell_weight(2.0 - f.x);
    let wy0 = mitchell_weight(1.0 + f.y);
    let wy1 = mitchell_weight(f.y);
    let wy2 = mitchell_weight(1.0 - f.y);
    let wy3 = mitchell_weight(2.0 - f.y);

    let x0 = base.x - 1;
    let x1 = base.x;
    let x2 = base.x + 1;
    let x3 = base.x + 2;
    let y0 = base.y - 1;
    let y1 = base.y;
    let y2 = base.y + 1;
    let y3 = base.y + 2;

    var sum = vec4<f32>(0.0);
    sum = sum + texel_fetch(tex, vec2<i32>(x0, y0), size_i) * (wx0 * wy0);
    sum = sum + texel_fetch(tex, vec2<i32>(x1, y0), size_i) * (wx1 * wy0);
    sum = sum + texel_fetch(tex, vec2<i32>(x2, y0), size_i) * (wx2 * wy0);
    sum = sum + texel_fetch(tex, vec2<i32>(x3, y0), size_i) * (wx3 * wy0);

    sum = sum + texel_fetch(tex, vec2<i32>(x0, y1), size_i) * (wx0 * wy1);
    sum = sum + texel_fetch(tex, vec2<i32>(x1, y1), size_i) * (wx1 * wy1);
    sum = sum + texel_fetch(tex, vec2<i32>(x2, y1), size_i) * (wx2 * wy1);
    sum = sum + texel_fetch(tex, vec2<i32>(x3, y1), size_i) * (wx3 * wy1);

    sum = sum + texel_fetch(tex, vec2<i32>(x0, y2), size_i) * (wx0 * wy2);
    sum = sum + texel_fetch(tex, vec2<i32>(x1, y2), size_i) * (wx1 * wy2);
    sum = sum + texel_fetch(tex, vec2<i32>(x2, y2), size_i) * (wx2 * wy2);
    sum = sum + texel_fetch(tex, vec2<i32>(x3, y2), size_i) * (wx3 * wy2);

    sum = sum + texel_fetch(tex, vec2<i32>(x0, y3), size_i) * (wx0 * wy3);
    sum = sum + texel_fetch(tex, vec2<i32>(x1, y3), size_i) * (wx1 * wy3);
    sum = sum + texel_fetch(tex, vec2<i32>(x2, y3), size_i) * (wx2 * wy3);
    sum = sum + texel_fetch(tex, vec2<i32>(x3, y3), size_i) * (wx3 * wy3);

    return sum;
}

fn sample_bicubic(tex: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    let size_u = textureDimensions(tex);
    let size_i = vec2<i32>(size_u);
    let size_f = vec2<f32>(size_u);
    let uv_clamped = clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    let texel = uv_clamped * size_f - vec2<f32>(0.5, 0.5);
    let base_f = floor(texel);
    let base = vec2<i32>(base_f);
    let f = texel - base_f;

    let x0 = base.x - 1;
    let x1 = base.x;
    let x2 = base.x + 1;
    let x3 = base.x + 2;
    let y0 = base.y - 1;
    let y1 = base.y;
    let y2 = base.y + 1;
    let y3 = base.y + 2;

    let r0 = catmull_rom(
        texel_fetch(tex, vec2<i32>(x0, y0), size_i),
        texel_fetch(tex, vec2<i32>(x1, y0), size_i),
        texel_fetch(tex, vec2<i32>(x2, y0), size_i),
        texel_fetch(tex, vec2<i32>(x3, y0), size_i),
        f.x
    );
    let r1 = catmull_rom(
        texel_fetch(tex, vec2<i32>(x0, y1), size_i),
        texel_fetch(tex, vec2<i32>(x1, y1), size_i),
        texel_fetch(tex, vec2<i32>(x2, y1), size_i),
        texel_fetch(tex, vec2<i32>(x3, y1), size_i),
        f.x
    );
    let r2 = catmull_rom(
        texel_fetch(tex, vec2<i32>(x0, y2), size_i),
        texel_fetch(tex, vec2<i32>(x1, y2), size_i),
        texel_fetch(tex, vec2<i32>(x2, y2), size_i),
        texel_fetch(tex, vec2<i32>(x3, y2), size_i),
        f.x
    );
    let r3 = catmull_rom(
        texel_fetch(tex, vec2<i32>(x0, y3), size_i),
        texel_fetch(tex, vec2<i32>(x1, y3), size_i),
        texel_fetch(tex, vec2<i32>(x2, y3), size_i),
        texel_fetch(tex, vec2<i32>(x3, y3), size_i),
        f.x
    );

    return catmull_rom(r0, r1, r2, r3, f.y);
}

fn sample_bicubic_clamped(
    tex: texture_2d<f32>,
    uv: vec2<f32>,
    clamp_min: vec2<i32>,
    clamp_max: vec2<i32>,
) -> vec4<f32> {
    let size_u = textureDimensions(tex);
    let size_i = vec2<i32>(size_u);
    let size_f = vec2<f32>(size_u);
    let uv_clamped = clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    let texel = uv_clamped * size_f - vec2<f32>(0.5, 0.5);
    let base_f = floor(texel);
    let base = vec2<i32>(base_f);
    let f = texel - base_f;

    let x0 = base.x - 1;
    let x1 = base.x;
    let x2 = base.x + 1;
    let x3 = base.x + 2;
    let y0 = base.y - 1;
    let y1 = base.y;
    let y2 = base.y + 1;
    let y3 = base.y + 2;

    let r0 = catmull_rom(
        texel_fetch_clamped(tex, vec2<i32>(x0, y0), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x1, y0), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x2, y0), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x3, y0), clamp_min, clamp_max, size_i),
        f.x
    );
    let r1 = catmull_rom(
        texel_fetch_clamped(tex, vec2<i32>(x0, y1), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x1, y1), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x2, y1), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x3, y1), clamp_min, clamp_max, size_i),
        f.x
    );
    let r2 = catmull_rom(
        texel_fetch_clamped(tex, vec2<i32>(x0, y2), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x1, y2), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x2, y2), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x3, y2), clamp_min, clamp_max, size_i),
        f.x
    );
    let r3 = catmull_rom(
        texel_fetch_clamped(tex, vec2<i32>(x0, y3), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x1, y3), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x2, y3), clamp_min, clamp_max, size_i),
        texel_fetch_clamped(tex, vec2<i32>(x3, y3), clamp_min, clamp_max, size_i),
        f.x
    );

    return catmull_rom(r0, r1, r2, r3, f.y);
}

fn sample_mitchell_clamped(
    tex: texture_2d<f32>,
    uv: vec2<f32>,
    clamp_min: vec2<i32>,
    clamp_max: vec2<i32>,
) -> vec4<f32> {
    let size_u = textureDimensions(tex);
    let size_i = vec2<i32>(size_u);
    let size_f = vec2<f32>(size_u);
    let uv_clamped = clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    let texel = uv_clamped * size_f - vec2<f32>(0.5, 0.5);
    let base_f = floor(texel);
    let base = vec2<i32>(base_f);
    let f = texel - base_f;

    let wx0 = mitchell_weight(1.0 + f.x);
    let wx1 = mitchell_weight(f.x);
    let wx2 = mitchell_weight(1.0 - f.x);
    let wx3 = mitchell_weight(2.0 - f.x);
    let wy0 = mitchell_weight(1.0 + f.y);
    let wy1 = mitchell_weight(f.y);
    let wy2 = mitchell_weight(1.0 - f.y);
    let wy3 = mitchell_weight(2.0 - f.y);

    let x0 = base.x - 1;
    let x1 = base.x;
    let x2 = base.x + 1;
    let x3 = base.x + 2;
    let y0 = base.y - 1;
    let y1 = base.y;
    let y2 = base.y + 1;
    let y3 = base.y + 2;

    var sum = vec4<f32>(0.0);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x0, y0), clamp_min, clamp_max, size_i) * (wx0 * wy0);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x1, y0), clamp_min, clamp_max, size_i) * (wx1 * wy0);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x2, y0), clamp_min, clamp_max, size_i) * (wx2 * wy0);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x3, y0), clamp_min, clamp_max, size_i) * (wx3 * wy0);

    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x0, y1), clamp_min, clamp_max, size_i) * (wx0 * wy1);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x1, y1), clamp_min, clamp_max, size_i) * (wx1 * wy1);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x2, y1), clamp_min, clamp_max, size_i) * (wx2 * wy1);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x3, y1), clamp_min, clamp_max, size_i) * (wx3 * wy1);

    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x0, y2), clamp_min, clamp_max, size_i) * (wx0 * wy2);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x1, y2), clamp_min, clamp_max, size_i) * (wx1 * wy2);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x2, y2), clamp_min, clamp_max, size_i) * (wx2 * wy2);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x3, y2), clamp_min, clamp_max, size_i) * (wx3 * wy2);

    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x0, y3), clamp_min, clamp_max, size_i) * (wx0 * wy3);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x1, y3), clamp_min, clamp_max, size_i) * (wx1 * wy3);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x2, y3), clamp_min, clamp_max, size_i) * (wx2 * wy3);
    sum = sum + texel_fetch_clamped(tex, vec2<i32>(x3, y3), clamp_min, clamp_max, size_i) * (wx3 * wy3);

    return sum;
}

fn sample_tex(tex: texture_2d<f32>, uv: vec2<f32>, mode: f32) -> vec4<f32> {
    if (mode > 2.5) {
        return textureSample(tex, s_nearest, uv);
    }
    if (mode > 1.5) {
        return sample_bicubic(tex, uv);
    }
    if (mode > 0.5) {
        return sample_mitchell(tex, uv);
    }
    return textureSample(tex, s_linear, uv);
}

fn sample_tex_clamped(
    tex: texture_2d<f32>,
    uv: vec2<f32>,
    mode: f32,
    clamp_min: vec2<i32>,
    clamp_max: vec2<i32>,
) -> vec4<f32> {
    if (mode > 2.5) {
        return textureSample(tex, s_nearest, uv);
    }
    if (mode > 1.5) {
        return sample_bicubic_clamped(tex, uv, clamp_min, clamp_max);
    }
    if (mode > 0.5) {
        return sample_mitchell_clamped(tex, uv, clamp_min, clamp_max);
    }
    return textureSample(tex, s_linear, uv);
}
