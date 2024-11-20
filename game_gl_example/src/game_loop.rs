//////////////////////////////////////////////////
// Using

use std::mem::size_of;

use game_gl::io::{InputEvent, Key, KeyState, KeyboardEvent};
use game_gl::opengl::{GlIndexBuffer, GlShader, GlTexture, GlUniformBuffer, GlVertexArrayObject, GlVertexBuffer};
use game_gl::prelude::*;

//////////////////////////////////////////////////
// Shader

const VS: &[u8] = b"#version 300 es
layout(location = 0) in vec2 a_Pos;
layout(location = 1) in vec2 a_TexCoord;

//in float a_TexSlot;

out vec3 v_TexCoord;

void main() {
    v_TexCoord = vec3(a_TexCoord, 0.0);
    gl_Position = vec4(a_Pos, 0.0, 1.0);
}
";

const FS: &[u8] = b"#version 300 es
precision mediump float;
precision mediump sampler2DArray;

in vec3 v_TexCoord;

uniform sampler2DArray t_Sampler;

layout(std140) uniform Settings {
    vec4 u_Color;
};

layout(location = 0) out vec4 target0;

void main() {
    target0 = texture(t_Sampler, v_TexCoord) * u_Color;
}
";

//////////////////////////////////////////////////
// Runner

#[derive(Debug, Default)]
pub struct ExampleGameLoop {
    ctx: Option<GameContext>,
    graphics: Graphics,
}

#[derive(Debug, Default)]
pub struct Graphics {
    vao: GlVertexArrayObject,
    vbo: GlVertexBuffer<[f32; 4]>,
    ibo: GlIndexBuffer,
    ubo: GlUniformBuffer<(f32, f32, f32, f32)>,
    texture: GlTexture,
    shader: GlShader,
    resolution: (GLsizei, GLsizei),
}

impl ExampleGameLoop {
    pub fn new() -> ExampleGameLoop {
        Default::default()
    }
}

impl GameLoop for ExampleGameLoop {
    fn title(&self) -> &str {
        "Test Example"
    }

    fn init(&mut self, ctx: GameContext) {
        log::debug!("init");
        self.ctx = Some(ctx);
    }

    fn cleanup(&mut self) {
        log::debug!("cleanup");
        self.ctx = None;
    }

    fn update(&mut self, _elapsed_time: f32) {
        //log::debug!("update");
    }

    fn input(&mut self, input_events: &[InputEvent]) {
        input_events.iter().for_each(|input_event| match input_event {
            InputEvent::Cursor(event) => {
                log::debug!("{:?}", event);
            }
            InputEvent::Mouse(event) => {
                log::debug!("{:?}", event);
            }
            InputEvent::Touch(event) => {
                log::debug!("{:?}", event);
            }
            InputEvent::Keyboard(KeyboardEvent { state, key }) => match (state, key) {
                (KeyState::Released, Key::Escape) => {
                    if let Some(ctx) = self.ctx.as_ref() {
                        ctx.write(|ctx| ctx.exit());
                    }
                }
                _ => {}
            },
        });
    }

    fn render(&mut self, gl: &Gl) {
        //log::debug!("render");
        unsafe {
            gl.ClearColor(1.0, 0.0, 0.0, 1.0);
            gl.ClearDepthf(1.0);
            gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            let fx = &mut self.graphics;
            fx.vao.bind();
            fx.ibo.bind();

            fx.texture.bind(1);
            fx.ubo.bind(1);

            fx.shader.bind();
            fx.shader.link_texture(1, "t_Sampler");
            fx.shader.link_uniform(1, "Settings");

            gl.Viewport(0, 0, fx.resolution.0, fx.resolution.1);
            // gl.Disable(gl::CULL_FACE);
            // gl.Disable(gl::DEPTH_TEST);
            // gl.Enable(gl::DEPTH_TEST);
            // gl.DepthMask(gl::TRUE);
            // gl.DepthFunc(gl::LESS);

            fx.shader.draw_elements(gl::TRIANGLE_STRIP, fx.ibo.count());

            fx.shader.unbind();

            fx.ubo.unbind();
            fx.texture.unbind();

            fx.ibo.unbind();
            fx.vao.unbind();
        }
    }

    fn create_device(&mut self, gl: &Gl) {
        log::debug!("create_device");

        // create resources
        let fx = &mut self.graphics;
        fx.vao = GlVertexArrayObject::new(gl);

        fx.vbo = GlVertexBuffer::new(gl, gl::STATIC_DRAW, &[[0.0; 4]; 4]);
        fx.vbo.update(&[[-0.5, -0.5, 0.0, 1.0], [-0.5, 0.5, 0.0, 0.0], [0.5, -0.5, 1.0, 1.0], [0.5, 0.5, 1.0, 0.0]]);

        fx.ibo = GlIndexBuffer::new(gl, gl::STATIC_DRAW, &[0; 4]);
        fx.ibo.update(&[0, 1, 2, 3]);

        fx.ubo = GlUniformBuffer::new(gl, gl::DYNAMIC_DRAW, &(0.0, 0.0, 0.0, 0.0));
        fx.ubo.update(&(0.5, 0.9, 0.9, 1.0));

        if let Some(ctx) = self.ctx.as_ref() {
            let files = ctx.read(|ctx| ctx.files());
            let buffer = files.load_bytes("lena.png").unwrap();
            let image = image::load_from_memory(&buffer).unwrap().to_rgba8();
            fx.texture = GlTexture::new(gl, &[image]);
        }

        fx.shader = GlShader::new(gl, VS, FS);

        // bind buffers to vao
        fx.vao.bind();
        fx.vao.bind_attrib(&fx.vbo, 0, 2, gl::FLOAT, gl::FALSE, 0, 4 * size_of::<f32>(), 0);
        fx.vao.bind_attrib(&fx.vbo, 1, 2, gl::FLOAT, gl::FALSE, 2 * size_of::<f32>(), 4 * size_of::<f32>(), 0);
        fx.vao.unbind();
    }

    fn destroy_device(&mut self, _gl: &Gl) {
        log::debug!("destroy_device");

        let fx = &mut self.graphics;
        fx.vao.release();
        fx.vbo.release();
        fx.ibo.release();
        fx.ubo.release();
        fx.texture.release();
        fx.shader.release();
    }

    fn resize_device(&mut self, _gl: &Gl, width: u32, height: u32) {
        log::debug!("resize_device ({} x {})", width, height);
        self.graphics.resolution = (width as GLsizei, height as GLsizei);
    }
}
