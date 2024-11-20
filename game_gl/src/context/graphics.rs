//////////////////////////////////////////////////
// Using

use image::imageops::{resize, FilterType};
use image::{GrayImage, Luma, RgbaImage};
use nalgebra_glm::*;
use rusttype::{point, Font, Scale};

use crate::opengl::{gl, gl::types::*, Gl, GlTexture};

//////////////////////////////////////////////////
// Definition

#[derive(Debug, Default)]
pub struct RawGraphicsContext {
    gl: Option<Gl>,
    resolution: Vec2,
}

unsafe impl Sync for RawGraphicsContext {}
unsafe impl Send for RawGraphicsContext {}

//////////////////////////////////////////////////
// Implementation

impl RawGraphicsContext {
    //////////////////////////////////////////////////
    // Device functions

    pub fn create(&mut self, gl: &Gl) {
        // set default bindings
        unsafe {
            // culling
            gl.Enable(gl::CULL_FACE);
            gl.CullFace(gl::BACK);

            // blending
            gl.Enable(gl::BLEND);
            gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            // depth
            gl.Disable(gl::DEPTH_TEST);
            gl.DepthMask(gl::FALSE);
            //gl.DepthFunc(gl::LESS);
        }

        // set context
        self.gl = Some(gl.clone());
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        // update screen size
        self.resolution = vec2(width as f32, height as f32);
        println!("RESIZE: {:?}", &self.resolution);

        // update viewport
        let gl = self.gl.as_ref().expect("Missing OpenGL context");
        unsafe {
            gl.Viewport(0, 0, self.resolution.x as GLsizei, self.resolution.y as GLsizei);
        }
    }

    pub fn destroy(&mut self) {
        // clear context
        self.gl = None;
    }

    //////////////////////////////////////////////////
    // Getter

    pub fn resolution(&self) -> Vec2 {
        self.resolution
    }

    //////////////////////////////////////////////////
    // Clear functions

    pub fn clear(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe {
                gl.ClearColor(1.0, 0.2, 0.3, 1.0);
                gl.ClearDepthf(1.0);
                gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            }
        }
    }

    pub fn clear_depth(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe {
                gl.ClearDepthf(1.0);
                gl.Clear(gl::DEPTH_BUFFER_BIT);
            }
        }
    }

    //////////////////////////////////////////////////
    // Texture functions

    pub fn create_texture(&self, gl: &Gl, textures: &[&[u8]]) -> GlTexture {
        // OLD FILE LOAD:
        // &ctx.files().load_bytes(path).expect(&format!("Failed to load file {}", path))
        let images: Vec<RgbaImage> = textures.iter().map(|buffer| image::load_from_memory(buffer).expect("Failed to read memory").to_rgba8()).collect();
        GlTexture::new(gl, &images)
    }

    pub fn create_font_texture(&self, gl: &Gl, font: &[u8], font_size: u32) -> GlTexture {
        // create font
        let font = Font::try_from_bytes(font).expect("Error constructing Font");
        let text: String = (0..128 as u8).map(|c| c as char).collect();
        let scale = Scale::uniform(font_size as f32);
        let v_metrics = font.v_metrics(scale);
        let glyphs = font.layout(&text, scale, point(0.0, v_metrics.ascent));
        // generate glyph images
        let images: Vec<GrayImage> = glyphs
            .map(|glyph| {
                // get bounding
                if let Some(bounding_box) = glyph.pixel_bounding_box() {
                    let glyph_width = (bounding_box.max.x - bounding_box.min.x) as u32;
                    let offset = (font_size - glyph_width).max(0) / 2;
                    // create new image to render glyph
                    let mut image = GrayImage::new(glyph_width.max(font_size) as u32, font_size as u32);
                    // Draw the glyph into the image per-pixel by using the draw closure
                    glyph.draw(|x, y, v| {
                        image.put_pixel(
                            // Offset the position by the glyph bounding box
                            x + offset as u32,
                            y + bounding_box.min.y as u32,
                            // Turn the coverage into an alpha value
                            Luma([(v * 255.0) as u8]),
                        )
                    });
                    // Save the image to a png file
                    resize(&image, font_size, font_size, FilterType::CatmullRom)
                } else {
                    GrayImage::new(font_size, font_size)
                }
            })
            .collect();
        GlTexture::new(gl, &images)
    }
}
