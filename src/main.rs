mod app;
mod environment;
use environment::Environment;
use wayland_client::{Attached, Display};
use smithay_client_toolkit::shm::AutoMemPool;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;

fn main() {
    // Command line arguments
    let mut args = std::env::args();
    let mut paper = app::Paper::None;
    args.next();
    loop {
        match args.next() {
            Some(flag) => match flag.as_str() {
                "-c" | "--color" => {
                    let color = u32::from_str_radix(&args.next().unwrap().trim_start_matches("#"), 16);
                    paper = app::Paper::Color(color.unwrap());
                }
                "-i" | "--image" => {
                    let path = args.next().unwrap();
                    paper = app::Paper::Image(path);
                }
                "-t" | "--tiled" => {
                    let path = args.next().unwrap();
                    paper = app::Paper::Tiled(path);
                }
                "-b" | "--border" => {
                    let path = args.next().unwrap();
                    let gap = if let Some(gap) = args.next() {
                        if let Ok(gap) = gap.parse::<u32>() {
                            gap
                        } else { 0 }
                    } else { 0 };
                    let color = u32::from_str_radix(&args.next().unwrap().trim_start_matches("#"), 16);
                    paper = app::Paper::Border(path, gap, color.unwrap());
                }
                "-tb" | "--tiled-bordered" => {
                    let path = args.next().unwrap();
                    let gap = if let Some(gap) = args.next() {
                        if let Ok(gap) = gap.parse::<u32>() {
                            gap
                        } else { 0 }
                    } else { 0 };
                    let color = u32::from_str_radix(&args.next().unwrap().trim_start_matches("#"), 16);
                    paper = app::Paper::TiledBorder(path, gap, color.unwrap());
                }
                "-h" | "--help" => {
                    print!("Usage: paper [option]\n\n");
                    print!("  -c | --color 		 		#AARRGGBB\n");
                    print!("  -i | --image 		 		/path/to/image\n");
                    print!("  -t | --tile 		 		/path/to/image\n");
                    print!("  -b | --border		 		/path/to/image border_size #AARRGGBB\n");
                    print!("  -tb | --tiled-bordered 	/path/to/image border_size #AARRGGBB\n");
                    println!();
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
                .get_layer_surface(&surface, Some(&output.wl_output), Layer::Background, String::from("paper"));
            surface.set_buffer_scale(output.scale);
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
