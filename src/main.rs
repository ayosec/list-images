mod images;
mod term;

fn main() {
    let mut failed = Vec::new();

    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());

    for file in std::env::args_os().skip(1) {
        let file = std::path::PathBuf::from(file);
        match images::thumbnail(&file, 150) {
            Ok(img) => term::render(&mut stdout, img.as_ref()).expect("Render image"),
            Err(e) => failed.push((file, e)),
        }
    }

    // Release lock on stdout.
    drop(stdout);

    for (file, err) in failed {
        eprintln!("{}: {}", file.display(), err);
    }
}
