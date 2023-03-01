//! Load images with ffmpeg.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

/// Default seek to generate thumbnails from a video.
const DEFAULT_THUMBNAIL_SEEK: f64 = 10.;

pub fn get_frame(path: &Path) -> anyhow::Result<Vec<u8>> {
    // Get duration of the stream.
    let duration = run(Command::new("ffprobe")
        .args(["-loglevel", "error"])
        .args(["-select_streams", "v:0"])
        .args(["-show_entries", "stream=duration"])
        .args(["-of", "flat"])
        .arg(path))?;

    let duration = 'duration: {
        // Find a line like `streams.stream.0.duration="123.45"`
        for line in std::str::from_utf8(&duration)?.split('\n') {
            let mut parts = line.splitn(2, '=');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                if key.contains("duration") {
                    let value: f64 = value
                        .strip_prefix('"')
                        .unwrap_or(value)
                        .strip_suffix('"')
                        .unwrap_or(value)
                        .parse()?;

                    break 'duration value;
                }
            }
        }

        anyhow::bail!("can't find duration from ffprobe");
    };

    // Launch ffmpeg to get a frame from the file at the `seek_percent`
    // position.
    //
    // Frame is encoded as PPM (lossless, uncompressed) to reduce
    // processing time.
    let data = run(Command::new("ffmpeg")
        .args(["-loglevel", "error"])
        .arg("-ss")
        .arg(format!("{}", duration * DEFAULT_THUMBNAIL_SEEK / 100.))
        .arg("-i")
        .arg(path)
        .args(["-vframes", "1"])
        .args(["-c:v", "ppm"])
        .args(["-f", "image2"])
        .arg("-"))?;

    Ok(data)
}

/// Run a command and returns its output if the process terminates successfully.
fn run(cmd: &mut Command) -> anyhow::Result<Vec<u8>> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn()?;
    let mut data = Vec::with_capacity(4096);
    child.stdout.as_mut().unwrap().read_to_end(&mut data)?;

    if child.wait()?.success() {
        return Ok(data);
    }

    anyhow::bail!("child failed");
}
