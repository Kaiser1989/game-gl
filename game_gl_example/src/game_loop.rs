//////////////////////////////////////////////////
// Using

use std::mem::size_of;

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

    fn init(&mut self, _ctx: &mut GameContext) {
        log::debug!("init");
    }

    fn cleanup(&mut self, _ctx: &mut GameContext) {
        log::debug!("cleanup");
    }

    fn update(&mut self, _ctx: &mut GameContext, _elapsed_time: f32) {
        //log::debug!("update");
    }

    fn input(&mut self, ctx: &mut GameContext, input_events: &[InputEvent]) {
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
                    ctx.exit();
                }
                _ => {}
            },
        });
    }

    fn render(&mut self, _ctx: &mut GameContext, gl: &Gl) {
        //log::debug!("render");
        unsafe {
            gl.ClearColor(1.0, 0.0, 0.0, 1.0);
            gl.ClearDepthf(1.0);
            gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            self.vao.bind();
            self.ibo.bind();

            self.texture.bind(1);
            self.ubo.bind(1);

            self.shader.bind();
            self.shader.link_texture(1, "t_Sampler");
            self.shader.link_uniform(1, "Settings");

            gl.Viewport(0, 0, self.resolution.0, self.resolution.1);
            // gl.Disable(gl::CULL_FACE);
            // gl.Disable(gl::DEPTH_TEST);
            // gl.Enable(gl::DEPTH_TEST);
            // gl.DepthMask(gl::TRUE);
            // gl.DepthFunc(gl::LESS);

            self.shader.draw_elements(gl::TRIANGLE_STRIP, self.ibo.count());

            self.shader.unbind();

            self.ubo.unbind();
            self.texture.unbind();

            self.ibo.unbind();
            self.vao.unbind();
        }
    }

    fn create_device(&mut self, ctx: &mut GameContext, gl: &Gl) {
        log::debug!("create_device");

        // create resources
        self.vao = GlVertexArrayObject::new(gl);

        self.vbo = GlVertexBuffer::new(gl, gl::STATIC_DRAW, &[[0.0; 4]; 4]);
        self.vbo.update(&[[-0.5, -0.5, 0.0, 1.0], [-0.5, 0.5, 0.0, 0.0], [0.5, -0.5, 1.0, 1.0], [0.5, 0.5, 1.0, 0.0]]);

        self.ibo = GlIndexBuffer::new(gl, gl::STATIC_DRAW, &[0; 4]);
        self.ibo.update(&[0, 1, 2, 3]);

        self.ubo = GlUniformBuffer::new(gl, gl::DYNAMIC_DRAW, &(0.0, 0.0, 0.0, 0.0));
        self.ubo.update(&(0.5, 0.9, 0.9, 1.0));

        let buffer = ctx.files().load_bytes("lena.png").unwrap();
        let image = image::load_from_memory(&buffer).unwrap().to_rgba8();
        self.texture = GlTexture::new(gl, &[image]);

        self.shader = GlShader::new(gl, VS, FS);

        // bind buffers to vao
        self.vao.bind();
        self.vao.bind_attrib(&self.vbo, 0, 2, gl::FLOAT, gl::FALSE, 0, 4 * size_of::<f32>(), 0);
        self.vao.bind_attrib(&self.vbo, 1, 2, gl::FLOAT, gl::FALSE, 2 * size_of::<f32>(), 4 * size_of::<f32>(), 0);
        self.vao.unbind();
    }

    fn destroy_device(&mut self, _ctx: &mut GameContext, _gl: &Gl) {
        log::debug!("destroy_device");

        self.vao.release();
        self.vbo.release();
        self.ibo.release();
        self.ubo.release();
        self.texture.release();
        self.shader.release();
    }

    fn resize_device(&mut self, _ctx: &mut GameContext, _gl: &Gl, width: u32, height: u32) {
        log::debug!("resize_device ({} x {})", width, height);
        self.resolution = (width as GLsizei, height as GLsizei);
    }
}
