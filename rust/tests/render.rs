//! GPU integration tests. Render shaders offscreen and check pixel output.
//!
//! Run with:
//!   cargo test --test render -- --ignored
//!
//! On Linux without hardware GPU, install lavapipe (mesa-vulkan-drivers) and
//! these tests will still pass.

use flutter_vulkan_native::{
    gpu_context::GpuContext,
    shader_pipeline::ShaderPipeline,
    uniform_queue::UniformValue,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn try_make_context() -> Option<GpuContext> {
    pollster::block_on(GpuContext::new_async()).ok()
}

// Full-screen triangle, the standard vertex shader used for ShaderToy rendering.
const VERT: &str = concat!(
    "#version 450\n",
    "void main() {\n",
    "    vec2 uv = vec2(float((gl_VertexIndex << 1) & 2),\n",
    "                   float( gl_VertexIndex        & 2));\n",
    "    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);\n",
    "}\n",
);

// Hash shader: per-pixel lowbias32 integer hash (Chris Wellons, nullprogram.com).
// Uses only integer bit-ops so the output is bit-exact across every GPU backend.
const FRAG_HASH: &str = concat!(
    "#version 450\n",
    "layout(set=0, binding=8) uniform PushConstants {\n",
    "    vec4  iMouse;\n",
    "    vec3  iResolution;\n",
    "    float iTime;\n",
    "} pc;\n",
    "layout(location=0) out vec4 fragColor;\n",
    "uint lowbias32(uint x) {\n",
    "    x ^= x >> 16u;\n",
    "    x *= 0x45d9f3bu;\n",
    "    x ^= x >> 16u;\n",
    "    return x;\n",
    "}\n",
    "void main() {\n",
    "    uint px = uint(gl_FragCoord.x);\n",
    "    uint py = uint(gl_FragCoord.y);\n",
    "    uint h  = lowbias32(px + py * uint(pc.iResolution.x));\n",
    "    fragColor = vec4(\n",
    "        float( h        & 0xffu) / 255.0,\n",
    "        float((h >>  8u)& 0xffu) / 255.0,\n",
    "        float((h >> 16u)& 0xffu) / 255.0,\n",
    "        1.0\n",
    "    );\n",
    "}\n",
);

fn lowbias32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x
}

fn expected_pixel(px: u32, py: u32, width: u32) -> [u8; 4] {
    let h = lowbias32(px + py * width);
    [(h & 0xff) as u8, ((h >> 8) & 0xff) as u8, ((h >> 16) & 0xff) as u8, 255]
}

fn make_hash_pipeline(ctx: &GpuContext, w: u32, h: u32) -> ShaderPipeline {
    let mut p = ShaderPipeline::new(&ctx.device, &ctx.queue, w, h);
    p.set_shaders_text(VERT, FRAG_HASH);
    p.get_uniforms_mut().add_uniform("iResolution", UniformValue::Vec3([w as f32, h as f32, 0.0]));
    p.get_uniforms_mut().add_uniform("iMouse",      UniformValue::Vec4([0.0; 4]));
    p.get_uniforms_mut().add_uniform("iTime",       UniformValue::Float(0.0));
    p
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// Renders a per-pixel hash and checks every pixel against the CPU reference.
// This covers the full pipeline: GLSL compile, wgpu pipeline creation, UBO
// upload, draw, readback, and row-stride stripping.
#[test]
#[ignore = "requires GPU adapter"]
fn hash_shader_matches_cpu_reference() {
    let ctx = match try_make_context() {
        Some(c) => c,
        None => { eprintln!("SKIP: no GPU adapter"); return; }
    };

    let (w, h) = (64, 64);
    let mut pipeline = make_hash_pipeline(&ctx, w, h);
    let err = pipeline.init_shader();
    assert!(err.is_empty(), "pipeline init: {err}");

    pipeline.draw_frame();
    let pixels = pipeline.read_pixels().expect("read_pixels");
    assert_eq!(pixels.len(), (w * h * 4) as usize);

    // Vulkan sets gl_FragCoord.y=0 at the top row, matching readback buffer order.
    let mut mismatches = 0usize;
    for py in 0..h {
        for px in 0..w {
            let off = ((py * w + px) * 4) as usize;
            let got = [pixels[off], pixels[off+1], pixels[off+2], pixels[off+3]];
            if got != expected_pixel(px, py, w) {
                mismatches += 1;
                if mismatches <= 8 {
                    eprintln!("({px},{py}): got {got:?}, want {:?}", expected_pixel(px, py, w));
                }
            }
        }
    }
    assert_eq!(mismatches, 0, "{mismatches} pixel(s) wrong");
}

// Writes iTime between draws and confirms the hash output (which ignores iTime)
// stays identical. Catches UBO writes that corrupt neighbouring fields.
#[test]
#[ignore = "requires GPU adapter"]
fn uniform_write_does_not_corrupt_output() {
    let ctx = match try_make_context() {
        Some(c) => c,
        None => { eprintln!("SKIP: no GPU adapter"); return; }
    };

    let (w, h) = (32, 32);
    let mut pipeline = make_hash_pipeline(&ctx, w, h);
    let err = pipeline.init_shader();
    assert!(err.is_empty(), "pipeline init: {err}");

    pipeline.draw_frame();
    let frame1 = pipeline.read_pixels().expect("read_pixels frame 1");

    pipeline.get_uniforms_mut().set_uniform_value("iTime", UniformValue::Float(99.0));
    pipeline.draw_frame();
    let frame2 = pipeline.read_pixels().expect("read_pixels frame 2");

    assert_eq!(frame1, frame2, "output changed after iTime write");

    for py in 0..h {
        for px in 0..w {
            let off = ((py * w + px) * 4) as usize;
            let got = [frame1[off], frame1[off+1], frame1[off+2], frame1[off+3]];
            assert_eq!(got, expected_pixel(px, py, w), "pixel ({px},{py})");
        }
    }
}
