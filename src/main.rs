mod images;
mod render;
mod term;

use images::Thumbnail;
use std::path::PathBuf;

const THUMBNAIL_SIZE: u32 = 5;

struct Job {
    file: PathBuf,
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
                    &job.file,
                    term.cell_height * THUMBNAIL_SIZE,
                    term.cell_width * THUMBNAIL_SIZE * 2,
                );
                job.tx.send((job.file, thumbnail)).unwrap();
            }
        });
    }

    let jobs: Vec<_> = std::env::args_os()
        .skip(1)
        .map(|file| {
            let file = PathBuf::from(file);
            let (tx, rx) = crossbeam_channel::unbounded();
            pending_tx.send(Job { file, tx }).unwrap();
            rx
        })
        .collect();

    // Collect results from the threads.

    let mut failed = Vec::new();
    let mut renderer = render::Renderer::new(term, THUMBNAIL_SIZE);

    for job in jobs {
        let (file, thumbnail) = job.recv()?;

        match thumbnail {
            Ok(img) => renderer.render(&img)?,
            Err(e) => failed.push((file, e)),
        }
    }

    drop(renderer);

    for (file, err) in failed {
        eprintln!("{}: {}", file.display(), err);
    }

    Ok(())
}
