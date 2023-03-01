mod archives;
mod ffmpeg;
mod images;
mod imgcache;
mod render;
mod term;

use clap::Parser;
use images::{Source, Thumbnail};

use std::path::PathBuf;
use std::sync::Arc;

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

    let cache = Arc::new(imgcache::Cache::new(args.thumbnail_size));

    // Launch multiple threads to create the thumbnails.

    let (pending_tx, pending_rx) = crossbeam_channel::unbounded::<Job>();

    for _ in 0..num_cpus::get() {
        let rx = pending_rx.clone();
        let cache = Arc::clone(&cache);
        let thumbnail_size = args.thumbnail_size;
        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                process_job(job, Option::as_ref(&cache), &term, thumbnail_size);
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
        while let Ok((path, thumbnail)) = job.recv() {
            match thumbnail {
                Ok(img) => renderer.render(&path, &img)?,
                Err(e) => failed.push((path, e)),
            }
        }
    }

    renderer.finish()?;

    for (path, err) in failed {
        eprintln!("{}: {}", path.display(), err);
    }

    Ok(())
}

fn process_job(job: Job, cache: Option<&imgcache::Cache>, term: &term::Term, thumbnail_size: u32) {
    // Try to open the file as an archive.
    if let Ok(archive) = archives::open(&job.path) {
        for entry in archive {
            let path = job.path.join(entry.name);
            render_file(
                Source::Mem(&entry.data, path),
                &job.tx,
                cache,
                term,
                thumbnail_size,
            );
        }

        return;
    }

    let Job { path, tx } = job;
    render_file(Source::Path(path), &tx, cache, term, thumbnail_size);
}

fn render_file(
    source: Source,
    tx: &crossbeam_channel::Sender<(PathBuf, anyhow::Result<Thumbnail>)>,
    cache: Option<&imgcache::Cache>,
    term: &term::Term,
    thumbnail_size: u32,
) {
    let thumbnail = cache
        .and_then(|c| c.get(source.path()).map(Ok))
        .unwrap_or_else(|| {
            let thumbnail = images::thumbnail(
                &source,
                term.cell_height * thumbnail_size,
                term.cell_width * thumbnail_size * 2,
            );

            if let (Some(cache), Ok(thumbnail)) = (cache.as_ref(), &thumbnail) {
                cache.store(source.path(), thumbnail);
            }

            thumbnail
        });

    tx.send((source.into_path_buf(), thumbnail)).unwrap();
}
