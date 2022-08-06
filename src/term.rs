use std::io::Write;

pub fn render(mut output: impl Write, img: &[u8]) -> std::io::Result<()> {
    output.write_all(b"\x1B]1337;File=inline=1:")?;

    let mut b64 = base64::write::EncoderWriter::new(&mut output, base64::STANDARD);
    b64.write_all(img)?;
    b64.finish()?;
    drop(b64);

    output.write_all(b"\x07")?;

    Ok(())
}
