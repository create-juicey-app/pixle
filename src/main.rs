mod app;
mod canvas;
mod commands;
mod packages;
mod scripting; // <--- ADDED

use app::AppState;
use winit::{event::*, event_loop::EventLoop, window::WindowBuilder};

fn main() {
    // ... (Same as before) ...
    // ... Copy the main function from the previous response ...
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Untitled - Pixle 0.1.2")
        .build(&event_loop)
        .unwrap();

    let mut state = pollster::block_on(AppState::new(&window));

    let _ = event_loop.run(move |event, target| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            state.handle_window_event(&window, event);
            match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(physical_size) => state.resize(*physical_size),
                WindowEvent::RedrawRequested => {
                    state.update();
                    match state.render(&window) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                _ => {}
            }
        }
        Event::AboutToWait => window.request_redraw(),
        _ => {}
    });
}
