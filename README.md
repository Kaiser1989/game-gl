# game-gl
Multiplatform game loop with OpenGL context. This Repository is a try to create some multiplatform game loop, initializing OpenGles context and handling input events. As this project is based on Winit and Glutin (both still having bugs running on android), the existing repositories were forked and bug fixed to make it work for this project.

<br>

## How to run?
All initialization is done for you. All you need to do is starting the game loop, and providing implementations for game loop callbacks:
```
pub fn main() {
    let mut game_loop = GameLoop::new(ExampleRunner{});
    game_loop.run();
}
```

In this case `ExampleRunner` implements the `Runner`-Trait, to provide all needed functions:
```
pub trait Runner {

    fn init(&mut self);

    fn cleanup(&mut self);

    fn pause(&mut self);

    fn resume(&mut self);

    fn input(&mut self, input_events: &[InputEvent]);

    fn update(&mut self, elapsed_time: f32);

    fn render(&mut self, gl: &Gl);

    fn create_device(&mut self, gl: &Gl);

    fn destroy_device(&mut self, gl: &Gl);

    fn resize_device(&mut self, gl: &Gl, width: u32, height: u32);
}
```

All needed types and structs, can be found directly located under `game_gl` crate:
```
use game_gl::{ GameLoop, Runner, gl, Gl, InputEvent};
```

<br>

## The game loop
The game loop is responsible to keep your app running. Usually a frame repeatly does:
* handle input events
* update world and entities
* render screen

where rendering should only be called, if our render context is initialized. Implementing the `Runner`-Trait ensures all this:
### init()
Init is called once right after the app is started and before any render context is created. This is the place to init all basic structs, allocate memory and prepare for running the loop.
### cleanup()
Cleanup is called once right before the app is destroyed. In early C++ days this was the place to free all the allocated memory. As Rust is doing memory handling for you automatically, you can use this so save some states for next game start.
### input()
Input is part of the loop functions. It provides a reference to a slice with all input events that occured in this frame. `InputEvent` is an enum with following types:
* `Cursor(CursorEvent)` -> contains current cursor location (only desktop)
* `Mouse(MouseEvent)` -> contains click information (only desktop)
* `Touch(TouchEvent)` -> contains touch information like location, pressed, moved (only android)
* `Keyboard(KeyboardEvent)` -> contains keyboard information for pressed keys
You don't need to process all events. If you miss an event, it's lost. Events are polled internally for every frame.
### update(elapsed_time: f32)
Update is part of the loop functions. It provides a variable given the elapsed time since the last frame. This is the place to update your entities movement, animations and all the stuff related to games.
### render(gl: &GL)
Render is part of the loop functions. It provides the OpenGles context, based on the gl_generator crate. Use this context to render your entities to the screen. The swap buffer method is called internally after rendering is done. The method is only called if a valid render context is available, otherwise this function will be skipped in this frame.
### pause()
Pause tells you that your app has been paused by the OS.
### resume()
Resume tells you that your app has been resumed by the OS.
### create_device(gl: &GL)
CreateDevice funtion is called whenever a render context is created. This is the place to initialize your constantly used graphics resources like textures, buffers and other OpenGL stuff. <br>
ATTENTION: Desktop apps will create the context right after calling the `init` function. The context will be available for the complete lifetime of your app. Android does some special context handling. Current issues from Glutin and Winit mention that render context is only available between Android's `resumed` and `suspended` methods. Sending your app to the background is destroying the context, resuming it will create one again. This is handled internally for your. If you have massive loads of graphics resources, this can result to a bad user experience as every resource needs to be uploaded to the OpenGL context again.
### destroy_device(gl: &GL)
DestroyDevice function is called whenever a render context is destroyed. This is the place to release all your graphics resources. <br>
ATTENTION: Desktop apps will destroy the context right before calling the `cleanup` function. As already mentioned, Android destroyes your context when sending the app to background and recreates it (create_device is called) when app is brought back to foreground again.
### resize_device(gl: &GL)
ResizeDevice function is called whenever a render context changes its size (resolution). This is the place to adjust your resolution dependend resources, e.g. an additional framebuffer. This funtion is also called once right after create_device to give the resolution of your window.

<br>

## Installing cargo apk and android targets
To install the build pipeline and android targets, follow the installation guidline on: 
https://github.com/rust-windowing/android-rs-glue

<br>

## Dependencies
This project is based on bug fixed forks of:
* glutin 0.28
* winit 0.26

All these projects were forked to my github. Therefore only the project only consists of git dependencies to my local git repositiories.
### Why don't you create pull requests in the original projects:
By following the isses on these crates, i noticed, that developing a working android solution doesn't have any priority (as of today: 17.07.2020). The changes i did are not a valid contribution to make these crates any better or to provide stable solutions for mobile platforms, they are just hacks and workarounds to make the current projects work for Android.

All you need to do, to get your app run on Windows and Android (not testes for IOS) is using these dependencies:
```
[dependencies]
game-gl = { git = "https://github.com/Kaiser1989/game-gl" }

[target.'cfg(target_os = "android")'.dependencies]
ndk-glue = "0.5.0"

[package.metadata.android]
build_targets = [ "armv7-linux-androideabi", "aarch64-linux-android", "i686-linux-android", "x86_64-linux-android" ]
```
where all sub dependencies are replaces to my bug fixed forks. There is no dependency left to original glutin or winit.

<br>

## Example
A running example of this game loop crate can be found at: https://github.com/Kaiser1989/rust-android-example. This example creates a simple game loop rendering a our beautiful lena. It's tested for windows and android.

<br>

## Special Thanks
Special thanks go to @Katyo who started fixing the glutin crate to make it build and of course to the developer of glutin, winit and android-ndk-rs. They do a good job. And the working solution is so close. When they got their issues fixed, this project here is no longer needed.

Feel free to do whatever you want with this ;)
