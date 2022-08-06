use std::os::unix::prelude::RawFd;

use anyhow::bail;
use nix::errno::Errno;
use nix::sys::termios::{tcgetattr, tcsetattr, LocalFlags, SetArg, Termios};
use nix::unistd;

pub struct Term {
    pub columns: u32,
    pub cell_height: u32,
    pub cell_width: u32,
}

// Use stdout to read and write, so the program can work with no stdin.
const STDIO: RawFd = 1;

struct RawMode(Termios);

impl RawMode {
    fn new() -> nix::Result<Self> {
        let attrs = tcgetattr(STDIO)?;

        let mut change = attrs.clone();
        change.local_flags.remove(LocalFlags::ICANON);
        change.local_flags.remove(LocalFlags::ECHO);

        tcsetattr(STDIO, SetArg::TCSAFLUSH, &change)?;
        Ok(RawMode(attrs))
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = tcsetattr(STDIO, SetArg::TCSADRAIN, &self.0);
    }
}

impl Term {
    pub fn new() -> anyhow::Result<Self> {
        if unistd::isatty(STDIO) != Ok(true) {
            bail!("Not a TTY");
        }

        let term_mode = match RawMode::new() {
            Ok(m) => m,
            Err(_) => bail!("Can't set raw mode"),
        };

        // Query the terminal and wait until we have at least the DA1 response.
        let mut write = &b"\x1B[14t\x1B[18t\x1B[c"[..];
        while !write.is_empty() {
            write = match unistd::write(STDIO, write) {
                Ok(w) => &write[w..],
                Err(e) => bail!("Failed to query data: {}", e),
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

            let read = match unistd::read(STDIO, &mut data) {
                Ok(n) => n,
                Err(Errno::EINTR) => continue,
                Err(e) => bail!("Failed to read from TTY: {}", e),
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
                            _ => continue,
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

                        _ => continue,
                    },

                    _ => (),
                }
            }
        }

        drop(term_mode);

        if win_width == 0 || win_height == 0 || rows == 0 || cols == 0 {
            bail!("Missing some dimensions from terminal response.");
        }

        let term = Term {
            columns: cols,
            cell_height: win_height / rows,
            cell_width: win_width / cols,
        };

        Ok(term)
    }
}
