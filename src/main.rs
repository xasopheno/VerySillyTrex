use raw_window_handle::HasRawWindowHandle;
use raw_window_handle::RawWindowHandle;
use std::env;
use std::path::Path;
use std::process;
use std::sync::{Arc, Mutex};
use vst::host::{Host, HostBuffer, PluginLoader};
use vst::plugin::Plugin;
use winit::window::Window;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use vst::buffer::SendEventBuffer;

#[allow(dead_code)]
struct SampleHost;

impl Host for SampleHost {
    fn automate(&self, index: i32, value: f32) {
        println!("Parameter {} had its value changed to {}", index, value);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: simple_host path/to/vst");
        process::exit(1);
    }

    let path = Path::new(&args[1]);

    // Create the host
    let host = Arc::new(Mutex::new(SampleHost));

    println!("Loading {}...", path.to_str().unwrap());

    // Load the plugin
    let mut loader = PluginLoader::load(path, Arc::clone(&host))
        .unwrap_or_else(|e| panic!("Failed to load plugin: {}", e));

    // Create an instance of the plugin
    let mut instance = loader.instance().unwrap();

    // Get the plugin information
    let info = instance.get_info();

    println!(
        "Loaded '{}':\n\t\
         Vendor: {}\n\t\
         Presets: {}\n\t\
         Parameters: {}\n\t\
         VST ID: {}\n\t\
         Version: {}\n\t\
         Initial Delay: {} samples",
        info.name,
        info.vendor,
        info.presets,
        info.parameters,
        info.unique_id,
        info.version,
        info.initial_delay
    );
    dbg!(&info);

    let editor = instance.get_editor();
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let handle = match window.raw_window_handle() {
        RawWindowHandle::MacOS(h) => h.ns_view,
        _ => panic!(),
    };
    // Initialize the instance
    editor.unwrap().open(handle);
    println!("Initialized instance!");

    let inputs = vec![vec![0.0; 1000]; 2];
    let mut outputs = vec![vec![0.0; 1000]; 32];
    let mut host_buffer: HostBuffer<f32> = HostBuffer::new(2, 32);

    event_loop.run(move |event, _, control_flow| {
        let mut audio_buffer = host_buffer.bind(&inputs, &mut outputs);
        let mut b = vst::buffer::SendEventBuffer::new(1024);
        b.send_events_to_plugin(
            vec![vst::event::MidiEvent {
                data: [1, 100, 100],
                delta_frames: 0,
                live: true,
                note_length: Some(40),
                note_offset: None,
                detune: 0,
                note_off_velocity: 50,
            }],
            &mut instance,
        );
        instance.process(&mut audio_buffer);
        let s: f32 = outputs[0].iter().sum();
        if s > 0.0 {
            dbg!(&outputs[0]);
        }

        *control_flow = ControlFlow::Poll;
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {}
            _ => (),
        }
    });

    println!("Closing instance...");
    // Close the instance. This is not necessary as the instance is shut down when
    // it is dropped as it goes out of scope.
    // drop(instance);
}
