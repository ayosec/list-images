mod images;
mod render;
mod term;

use images::Thumbnail;
use std::path::PathBuf;

const THUMBNAIL_SIZE: u32 = 5;

struct Job {
    path: PathBuf,
    tx: crossbeam_channel::Sender<(PathBuf, anyhow::Result<Thumbnail>)>,
}

fn main() -> anyhow::Result<()> {
    let term = term::Term::new()?;

    // Launch multiple threads to create the thumbnails.

    let (pending_tx, pending_rx) = crossbeam_channel::unbounded::<Job>();

    for _ in 0..num_cpus::get() {
        let rx = pending_rx.clone();
        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                let thumbnail = images::thumbnail(
                    &job.path,
                    term.cell_height * THUMBNAIL_SIZE,
                    term.cell_width * THUMBNAIL_SIZE * 2,
                );
                job.tx.send((job.path, thumbnail)).unwrap();
            }
        });
    }

    let jobs: Vec<_> = std::env::args_os()
        .skip(1)
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
    let mut renderer = render::Renderer::new(term, THUMBNAIL_SIZE);

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
