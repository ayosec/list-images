use crate::images::Thumbnail;
use crate::term::Term;

use std::io::{self, Write};
use std::path::Path;

pub struct Renderer<'a, T> {
    output: T,
    term: Term,
    args: &'a crate::Args,
    row_height: u32,
    row_offset_x: u32,
}

impl<'a> Renderer<'a, ()> {
    pub fn new(term: Term, args: &'a crate::Args) -> Renderer<'a, impl Write> {
        let stdout = std::io::BufWriter::new(std::io::stdout().lock());
        Renderer::new_with_output(stdout, term, args)
    }
}

impl<'a, T: Write> Renderer<'a, T> {
    pub fn new_with_output(output: T, term: Term, args: &'a crate::Args) -> Self {
        Renderer {
            output,
            term,
            args,
            row_height: 0,
            row_offset_x: 0,
        }
    }

    pub fn render(&mut self, path: &Path, img: &Thumbnail) -> io::Result<()> {
        // Compute size in cells.
        let width = img.width / self.term.cell_width;
        let height = img.height / self.term.cell_height;

        if self.row_height == 0 || self.row_offset_x + width > self.term.columns {
            self.start_row()?;
        }

        if self.row_offset_x > 0 {
            write!(&mut self.output, "\x1B8\x1B[{}C", self.row_offset_x)?;
        }

        // Hyperlink to the path.
        if !self.args.no_hyperlinks {
            write!(
                &mut self.output,
                "\x1B[38;2;{};{};{}m",
                self.args.hyperlink_color[0],
                self.args.hyperlink_color[1],
                self.args.hyperlink_color[2]
            )?;

            write!(&mut self.output, "\x1B]8;;{}\x07", path.display())?;
        }

        // Send the thumbnail using iTerm2 protocol.
        self.output.write_all(b"\x1B]1337;File=inline=1:")?;

        let mut b64 = base64::write::EncoderWriter::new(&mut self.output, base64::STANDARD);
        b64.write_all(&img.pixels)?;
        b64.finish()?;
        drop(b64);

        // Finish both image and hyperlink.
        self.output.write_all(b"\x07")?;

        if !self.args.no_hyperlinks {
            self.output.write_all(b"\x1B]8;;\x07\x1B[m")?;
        }

        // Update row position.

        self.row_offset_x += width + 1;

        if height > self.row_height {
            self.row_height = height;
        }

        Ok(())
    }

    fn start_row(&mut self) -> io::Result<()> {
        if self.row_height > 0 {
            write!(&mut self.output, "\x1B8\x1B[{}B", self.args.thumbnail_size)?;
        }

        self.row_offset_x = 0;
        self.row_height = 0;

        for _ in 0..=self.args.thumbnail_size {
            self.output.write_all(b"\n")?;
        }

        write!(&mut self.output, "\x1B[{}A\x1B7", self.args.thumbnail_size)
    }
}
