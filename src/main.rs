mod images;
mod render;
mod term;

use clap::Parser;
use images::Thumbnail;

use std::path::PathBuf;

#[derive(Parser)]
pub struct Args {
    /// Size, in cells, of the thumbnail.
    #[clap(short, long, default_value_t = 5)]
    thumbnail_size: u32,

    /// Don't add hyperlinks to every image.
    #[clap(short = 'N', long)]
    no_hyperlinks: bool,

    /// Images to render.
    images: Vec<PathBuf>,

    /// Color to set foreground for hyperlinks.
    #[clap(short = 'c', long, value_parser = parse_color, default_value = "FF7700")]
    hyperlink_color: [u8; 3],
}

fn parse_color(value: &str) -> Result<[u8; 3], &'static str> {
    if value.len() == 6 {
        if let Ok(v) = u32::from_str_radix(value, 16) {
            let v = v.to_be_bytes();
            return Ok([v[1], v[2], v[3]]);
        }
    }

    Err("Expected RRGGBB in hexadecimal digits.")
}

struct Job {
    path: PathBuf,
    tx: crossbeam_channel::Sender<(PathBuf, anyhow::Result<Thumbnail>)>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let term = term::Term::new()?;

    // Launch multiple threads to create the thumbnails.

    let (pending_tx, pending_rx) = crossbeam_channel::unbounded::<Job>();

    for _ in 0..num_cpus::get() {
        let rx = pending_rx.clone();
        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                let thumbnail = images::thumbnail(
                    &job.path,
                    term.cell_height * args.thumbnail_size,
                    term.cell_width * args.thumbnail_size * 2,
                );
                job.tx.send((job.path, thumbnail)).unwrap();
            }
        });
    }

    let jobs: Vec<_> = args
        .images
        .iter()
        .map(|file| {
            let path = PathBuf::from(file);
            let path = path.canonicalize().unwrap_or(path);

            let (tx, rx) = crossbeam_channel::unbounded();
            pending_tx.send(Job { path, tx }).unwrap();

            rx
        })
        .collect();

    // Collect results from the threads.

    let mut failed = Vec::new();
    let mut renderer = render::Renderer::new(term, &args);

    for job in jobs {
        let (path, thumbnail) = job.recv()?;

        match thumbnail {
            Ok(img) => renderer.render(&path, &img)?,
            Err(e) => failed.push((path, e)),
        }
    }

    drop(renderer);

    for (path, err) in failed {
        eprintln!("{}: {}", path.display(), err);
    }

    Ok(())
}
