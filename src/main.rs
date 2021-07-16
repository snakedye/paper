mod app;
mod environment;
use environment::Environment;
use wayland_client::{Attached, Display};
use smithay_client_toolkit::shm::AutoMemPool;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;

fn main() {
    // Command line arguments
    let mut args = std::env::args();
    let mut paper = app::Paper::default();
    args.next();
    loop {
        match args.next() {
            Some(flag) => match flag.as_str() {
                "-c" | "--color" => {
                    let res = u32::from_str_radix(&args.next().unwrap().trim_start_matches("#"), 16);
                    if let Ok(color) = res {
                        paper.style(app::Style::Color(color));
                    }
                }
                "-i" | "--image" => {
                    if let Some(path) = args.next() {
                        paper.style(app::Style::Image(path));
                    }
                }
                "-t" | "--tiled" => {
                    if let Some(path) = args.next() {
                        paper.style(app::Style::Tiled(path));
                    }
                }
                "-b" | "--border" => {
                    if let Some(gap) = args.next() {
                        if let Ok(gap) = gap.parse::<u32>() {
                            let res = u32::from_str_radix(&args.next().unwrap().trim_start_matches("#"), 16);
                            if let Ok(color) = res {
                                paper.border(gap, color);
                            }
                        }
                    }
                }
                "-h" | "--help" => {
                    print!("Usage: paper [option]\n\n");
                    print!("  -c | --color 		 	#AARRGGBB\n");
                    print!("  -i | --image 		 	/path/to/image\n");
                    print!("  -t | --tile 		 	/path/to/image\n");
                    println!("  -b | --border		 	border_size #AARRGGBB\n");
                }
                _ => break
            }
            None => break
        }
    }

    if paper.is_some() {
        let display = Display::connect_to_env().unwrap();
        let mut event_queue = display.create_event_queue();
        let environment = Environment::new(&display, &mut event_queue);

        let attached = Attached::from(environment.shm.clone().expect("No shared memory pool"));

        for output in &environment.outputs {
            let mempool = AutoMemPool::new(attached.clone()).unwrap();
            let surface = environment.get_surface();
            let layer_surface = environment
                .layer_shell
                .as_ref()
                .expect("Compositor doesn't implement the LayerShell protocol")
                .get_layer_surface(&surface, Some(&output.wl_output), Layer::Background, String::from("wallpaper"));
            surface.set_buffer_scale(output.scale);
            layer_surface.set_anchor(Anchor::all());
            let snape = app::Snape::new(
                output.width,
                output.height,
                surface,
                layer_surface,
                mempool
            );
            snape.dispatch_surface(paper.clone());
        }

        loop {
            event_queue
                .dispatch(&mut (), |event, object, _| {
                    panic!(
                        "[callop] Encountered an orphan event: {}@{}: {}",
                        event.interface,
                        object.as_ref().id(),
                        event.name
                    );
                })
                .unwrap();
        }
    }
}
