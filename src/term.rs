use nix::errno::Errno;
use nix::sys::termios::{tcgetattr, tcsetattr, LocalFlags, SetArg, Termios};
use nix::unistd;
use std::io::Write;
use std::os::unix::prelude::RawFd;

pub struct CellSize {
    pub height: u32,
    pub width: u32,
}

const DEFAULT_CELL_SIZE: CellSize = CellSize {
    height: 20,
    width: 10,
};

const STDIN: RawFd = 0;
const STDOUT: RawFd = 1;

struct RawMode(Termios);

impl RawMode {
    fn new() -> nix::Result<Self> {
        let attrs = tcgetattr(STDOUT)?;

        let mut change = attrs.clone();
        change.local_flags.remove(LocalFlags::ICANON);
        change.local_flags.remove(LocalFlags::ECHO);

        tcsetattr(STDOUT, SetArg::TCSAFLUSH, &change)?;
        Ok(RawMode(attrs))
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = tcsetattr(STDOUT, SetArg::TCSADRAIN, &self.0);
    }
}

pub fn cell_size() -> CellSize {
    if unistd::isatty(STDOUT) != Ok(true) || unistd::isatty(STDIN) != Ok(true) {
        return DEFAULT_CELL_SIZE;
    }

    let term_mode = match RawMode::new() {
        Ok(m) => m,
        Err(_) => return DEFAULT_CELL_SIZE,
    };

    // Query the terminal and wait until we have at least the DA1 response.
    let mut write = &b"\x1B[14t\x1B[18t\x1B[c"[..];
    while !write.is_empty() {
        write = match unistd::write(STDOUT, write) {
            Ok(w) => &write[w..],
            Err(_) => return DEFAULT_CELL_SIZE,
        };
    }

    let mut win_width = 0;
    let mut win_height = 0;
    let mut rows = 0;
    let mut cols = 0;

    #[derive(PartialEq, Copy, Clone)]
    enum Parser {
        None,
        SquareBracket,
        Question,
        Esc,
        BeforeWinHeight,
        WinHeight,
        WinWidth,
        BeforeRows,
        Rows,
        Cols,
    }

    let mut state = Parser::None;

    'main: loop {
        let mut data = [0; 64];

        let read = match unistd::read(STDIN, &mut data) {
            Ok(n) => n,
            Err(Errno::EINTR) => continue,
            _ => return DEFAULT_CELL_SIZE,
        };

        for byte in &data[..read] {
            match *byte {
                b'c' => break 'main,

                0x1B => state = Parser::Esc,

                b'[' if state == Parser::Esc => state = Parser::SquareBracket,

                b'?' if state == Parser::SquareBracket => state = Parser::Question,

                b';' => {
                    state = match state {
                        Parser::BeforeWinHeight => Parser::WinHeight,
                        Parser::WinHeight => Parser::WinWidth,
                        Parser::BeforeRows => Parser::Rows,
                        Parser::Rows => Parser::Cols,
                        _ => return DEFAULT_CELL_SIZE,
                    };
                }

                b'0'..=b'9' => match (u32::from(*byte - b'0'), state) {
                    (4, Parser::SquareBracket) => state = Parser::BeforeWinHeight,

                    (8, Parser::SquareBracket) => state = Parser::BeforeRows,

                    (n, Parser::WinHeight) => win_height = win_height * 10 + n,

                    (n, Parser::WinWidth) => win_width = win_width * 10 + n,

                    (n, Parser::Rows) => rows = rows * 10 + n,

                    (n, Parser::Cols) => cols = cols * 10 + n,

                    (_, Parser::Question) => (),

                    _ => return DEFAULT_CELL_SIZE,
                },

                _ => (),
            }
        }
    }

    drop(term_mode);

    if win_width == 0 || win_height == 0 || rows == 0 || cols == 0 {
        return DEFAULT_CELL_SIZE;
    }

    CellSize {
        height: win_height / rows,
        width: win_width / cols,
    }
}

pub fn render(mut output: impl Write, img: &[u8]) -> std::io::Result<()> {
    output.write_all(b"\x1B]1337;File=inline=1:")?;

    let mut b64 = base64::write::EncoderWriter::new(&mut output, base64::STANDARD);
    b64.write_all(img)?;
    b64.finish()?;
    drop(b64);

    output.write_all(b"\x07")?;

    Ok(())
}
