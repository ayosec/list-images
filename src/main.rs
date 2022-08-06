mod images;
mod term;

use std::path::PathBuf;

struct Job {
    file: PathBuf,
    tx: crossbeam_channel::Sender<(PathBuf, anyhow::Result<Vec<u8>>)>,
}

fn main() {
    let cell_size = term::cell_size();

    // Launch multiple threads to create the thumbnails.

    let (pending_tx, pending_rx) = crossbeam_channel::unbounded::<Job>();

    for _ in 0..num_cpus::get() {
        let rx = pending_rx.clone();
        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                let thumbnail =
                    images::thumbnail(&job.file, cell_size.height * 5, cell_size.width * 10);
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
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());

    for job in jobs {
        let (file, thumbnail) = job.recv().unwrap();

        match thumbnail {
            Ok(img) => term::render(&mut stdout, img.as_ref()).expect("Render image"),
            Err(e) => failed.push((file, e)),
        }
    }

    drop(stdout);

    for (file, err) in failed {
        eprintln!("{}: {}", file.display(), err);
    }
}
