use macroquad::prelude::*;
use macroquad::material::{load_material, Material};
use macroquad::texture::{render_target, FilterMode};

fn lerp(sphere_position: Vec3, target_position: Vec3) -> Vec3 {
    sphere_position + (target_position - sphere_position) * 0.2
}

fn trigger_jump(is_jumping: &mut bool, velocity_y: &mut f32) {
    if !*is_jumping {
        *is_jumping = true;
        *velocity_y = 0.4; // Initial jump velocity
    }
}

#[macroquad::main("3D")]
async fn main() {
    let render_target = render_target(screen_width() as u32, screen_height() as u32);
    render_target.texture.set_filter(FilterMode::Nearest);

    let material = load_material(
        ShaderSource::Glsl {
            vertex: CRT_VERTEX_SHADER,
            fragment: CRT_FRAGMENT_SHADER,
        },
        Default::default(),
    )
    .unwrap();

    let blur_material = load_material(
        ShaderSource::Glsl {
            vertex: BLUR_VERTEX_SHADER,
            fragment: BLUR_FRAGMENT_SHADER,
        },
        Default::default(),
    )
    .unwrap();

    let mut sphere_position = vec3(-8., 0.5, 0.);
    let mut target_position = vec3(-8., 0.5, 0.);
    let mut camera_position = vec3(-20., 15., 0.);

    let mut is_jumping = false;
    let mut velocity_y = 0.0;
    let gravity = -0.02;

    loop {
        set_camera(&Camera3D {
            position: camera_position,
            up: vec3(0., 1., 0.),
            render_target: Some(render_target.clone()),
            target: sphere_position,
            fovy: 19.5,
            ..Default::default()
        });

        clear_background(LIGHTGRAY);

        draw_grid(20, 1., BLACK, GRAY);

        draw_cube_wires(vec3(0., 1., -6.), vec3(2., 2., 2.), DARKGREEN);
        draw_cube_wires(vec3(0., 1., 6.), vec3(2., 2., 2.), DARKBLUE);
        draw_cube_wires(vec3(2., 1., 2.), vec3(2., 2., 2.), YELLOW);

        draw_cube(vec3(2., 0., -2.), vec3(0.4, 0.4, 0.4), None, BLACK);

        // Add controls for the sphere
        if is_key_down(KeyCode::W) {
            target_position.x += 0.1;
        }
        if is_key_down(KeyCode::S) {
            target_position.x -= 0.1;
        }
        if is_key_down(KeyCode::A) {
            target_position.z -= 0.1;
        }
        if is_key_down(KeyCode::D) {
            target_position.z += 0.1;
        }
        if is_key_down(KeyCode::Space) {
            trigger_jump(&mut is_jumping, &mut velocity_y);
        }

        // Apply gravity and update position
        if is_jumping {
            velocity_y += gravity;
            target_position.y += velocity_y;

            if target_position.y <= 0.5 {
                is_jumping = false;
                velocity_y = 0.0;
            }
        }

        sphere_position = lerp(sphere_position, target_position);
        camera_position = lerp(camera_position, vec3(sphere_position.x - 20., 15., sphere_position.z));
        draw_sphere(sphere_position, 1., None, BLUE);

        // Back to screen space, render some text
        set_default_camera();
        
        draw_text("WELCOME TO 3D WORLD", 10.0, 20.0, 30.0, BLACK);

        gl_use_material(&material);
        draw_texture_ex(
            &render_target.texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                flip_y: true,
                ..Default::default()
            },
        );

        gl_use_material(&blur_material);
        draw_texture_ex(
            &render_target.texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                flip_y: true,
                ..Default::default()
            },
        );
        gl_use_default_material();

        next_frame().await;
    }
}

const CRT_FRAGMENT_SHADER: &'static str = r#"#version 100
precision lowp float;

varying vec4 color;
varying vec2 uv;

uniform sampler2D Texture;

// https://www.shadertoy.com/view/XtlSD7

vec2 CRTCurveUV(vec2 uv)
{
    uv = uv * 2.0 - 1.0;
    vec2 offset = abs( uv.yx ) / vec2( 6.0, 4.0 );
    uv = uv + uv * offset * offset;
    uv = uv * 0.5 + 0.5;
    return uv;
}

void DrawVignette( inout vec3 color, vec2 uv )
{
    float vignette = uv.x * uv.y * ( 1.0 - uv.x ) * ( 1.0 - uv.y );
    vignette = clamp( pow( 16.0 * vignette, 0.3 ), 0.0, 1.0 );
    color *= vignette;
}


void DrawScanline( inout vec3 color, vec2 uv )
{
    float iTime = 0.1;
    float scanline  = clamp( 0.95 + 0.05 * cos( 3.14 * ( uv.y + 0.008 * iTime ) * 240.0 * 1.0 ), 0.0, 1.0 );
    float grille  = 0.85 + 0.15 * clamp( 1.5 * cos( 3.14 * uv.x * 640.0 * 1.0 ), 0.0, 1.0 );
    color *= scanline * grille * 1.2;
}

void main() {
    vec2 crtUV = CRTCurveUV(uv);
    vec3 res = texture2D(Texture, uv).rgb * color.rgb;
    if (crtUV.x < 0.0 || crtUV.x > 1.0 || crtUV.y < 0.0 || crtUV.y > 1.0)
    {
        res = vec3(0.0, 0.0, 0.0);
    }
    DrawVignette(res, crtUV);
    // DrawScanline(res, uv);
    gl_FragColor = vec4(res, 1.0);
}
"#;

const CRT_VERTEX_SHADER: &'static str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
";

const BLUR_FRAGMENT_SHADER: &'static str = r#"#version 100
precision mediump float;

uniform sampler2D Texture;
varying vec2 uv;

void main() {
    vec4 color = vec4(0.0);

    // Perform a simple blur by sampling surrounding pixels
    float blurSize = (0.5 - uv.y) / 300.;

    for(float x = -4.0; x <= 4.0; x++) {
        for(float y = -4.0; y <= 4.0; y++) {
            color += texture2D(Texture, uv + vec2(x, y) * blurSize) / 81.0;
        }
    }

    gl_FragColor = color;
}
"#;

const BLUR_VERTEX_SHADER: &'static str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
";
