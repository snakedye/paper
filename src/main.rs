mod app;
mod environment;
use std::path::Path;
use snui::widgets::*;
use environment::Environment;
use wayland_client::{Display};

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
                        paper.style(app::Style::Image(Image::new(Path::new(&path))));
                    }
                }
                "-d" | "--dir" => {
                    if let Some(path) = args.next() {
                        paper.style(app::Style::Directory(path));
                    }
                }
                "-o" | "--output" =>  paper.output = args.next(),
                "-t" | "--tiled" => {
                    if let Some(path) = args.next() {
                        paper.style(app::Style::Tiled(Image::new(Path::new(&path))));
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
                    print!("  -t | --tile 		 	/path/to/image\n");
                    print!("  -i | --image 		 	/path/to/image\n");
                    print!("  -d | --dir 		 	/path/to/directory\n");
                    print!("  -o | --output			the name of your output\n");
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
        let mut environment = Environment::new(&display, &mut event_queue, paper);

        loop {
            event_queue
                .dispatch(&mut environment, |event, object, _| {
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
