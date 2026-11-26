use std::io::{self, Read, Write};

#[cfg(unix)]
pub struct RawModeGuard {
    fd: i32,
    original: libc::termios,
}

#[cfg(unix)]
impl RawModeGuard {
    pub fn new() -> io::Result<Self> {
        unsafe {
            let fd = 0;
            let mut original: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut original) != 0 {
                return Err(io::Error::last_os_error());
            }
            let mut raw = original;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
            raw.c_iflag &= !(libc::IXON | libc::ICRNL);
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(fd, libc::TCSANOW, &raw) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self { fd, original })
        }
    }
}

#[cfg(unix)]
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

/// Read a single UTF-8 character from stdin (raw mode, byte at a time).
fn read_char(stdin: &mut io::Stdin) -> io::Result<Option<char>> {
    let mut byte = [0u8; 1];
    if stdin.read(&mut byte)? == 0 {
        return Ok(None);
    }
    let b = byte[0];
    if b < 0x80 {
        return Ok(Some(b as char));
    }
    let len = if b >= 0xF0 {
        4
    } else if b >= 0xE0 {
        3
    } else if b >= 0xC0 {
        2
    } else {
        // stray continuation byte: skip it
        return Ok(None);
    };
    let mut buf = vec![b];
    for _ in 1..len {
        if stdin.read(&mut byte)? == 0 {
            break;
        }
        buf.push(byte[0]);
    }
    Ok(String::from_utf8(buf).ok().and_then(|s| s.chars().next()))
}

/// Visible width of a string, ignoring ANSI SGR escape sequences.
fn visible_len(s: &str) -> usize {
    let mut n = 0usize;
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            for c2 in it.by_ref() {
                if c2 == 'm' {
                    break;
                }
            }
        } else {
            n += 1;
        }
    }
    n
}

#[cfg(unix)]
fn term_width() -> usize {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(0, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            return (ws.ws_col as usize).saturating_sub(2).clamp(40, 110);
        }
    }
    80
}

#[cfg(not(unix))]
fn term_width() -> usize {
    80
}

fn matches<'a>(buf: &'a str, commands: &'a [(&'a str, &'a str)]) -> Vec<&'a str> {
    if !buf.starts_with('/') {
        return vec![];
    }
    commands
        .iter()
        .map(|(c, _)| *c)
        .filter(|c| c.starts_with(buf))
        .collect()
}

fn command_desc<'a>(commands: &'a [(&'a str, &'a str)], name: &str) -> &'a str {
    commands
        .iter()
        .find(|(c, _)| *c == name)
        .map(|(_, d)| *d)
        .unwrap_or("")
}

/// Draw the boxed input UI. Leaves the cursor on the input line.
/// Returns the number of menu lines drawn (0 when the menu is hidden).
fn draw_box(
    stdout: &mut io::Stdout,
    prompt: &str,
    buf: &str,
    cursor: usize,
    selected: usize,
    commands: &[(&str, &str)],
    rendered: &mut bool,
) -> io::Result<usize> {
    let m = matches(buf, commands);
    let show_menu = !m.is_empty() && buf.starts_with('/');
    let menu_count = if show_menu { m.len() } else { 0 };

    if *rendered {
        // cursor is on the input line; go to the top border and clear down
        write!(stdout, "\x1b[1A\r\x1b[J")?;
    }
    *rendered = true;

    let w = term_width();
    let border_h = format!("╭{}╮", "─".repeat(w - 2));
    let border_b = format!("╰{}╯", "─".repeat(w - 2));

    write!(stdout, "{}\r\n", border_h)?;

    // input line: "│ " + prompt + buf + padding + " │"
    let text = format!("{}{}", prompt, buf);
    let vis = visible_len(&text);
    let pad = (w as isize - 4 - vis as isize).max(0) as usize;
    write!(stdout, "│ {}{}{} │\r\n", text, " ".repeat(pad), "")?;

    // menu lines
    if show_menu {
        for (i, c) in m.iter().enumerate() {
            let label = {
                let l = format!("{}  {}", c, command_desc(commands, c));
                if l.chars().count() > w - 4 {
                    l.chars().take(w - 4).collect::<String>()
                } else {
                    l
                }
            };
            if i == selected {
                write!(
                    stdout,
                    "│ \x1b[7m {:<width$} \x1b[0m│\r\n",
                    label,
                    width = w - 4
                )?;
            } else {
                write!(stdout, "│ {:<width$} │\r\n", label, width = w - 4)?;
            }
        }
    }

    write!(stdout, "{}", border_b)?;

    // back to the input line and restore cursor column
    let up = menu_count + 1;
    write!(stdout, "\x1b[{}A\r", up)?;
    let target = 2 + visible_len(prompt) + buf[..cursor].chars().count();
    write!(stdout, "\x1b[{}C", target)?;
    stdout.flush()?;
    Ok(menu_count)
}

/// Clear the box and print the submitted line as plain output.
fn finish_line(stdout: &mut io::Stdout, prompt: &str, buf: &str, rendered: bool) -> io::Result<()> {
    if rendered {
        write!(stdout, "\x1b[1A\r\x1b[J")?;
    }
    writeln!(stdout, "{}{}", prompt, buf)?;
    stdout.flush()
}

/// Read a line with a boxed input UI + slash-command menu (auto-show).
/// `commands` is a list of (command, description) pairs.
pub fn read_line_interactive(
    prompt: &str,
    commands: &[(&str, &str)],
) -> io::Result<Option<String>> {
    let _guard = RawModeGuard::new()?;
    let mut stdout = io::stdout();
    let mut stdin = io::stdin();
    let mut buf = String::new();
    let mut cursor = 0usize; // byte index, always on a char boundary
    let mut selected: usize = 0;
    let mut rendered = false;

    draw_box(
        &mut stdout,
        prompt,
        &buf,
        cursor,
        selected,
        commands,
        &mut rendered,
    )?;

    loop {
        let Some(ch) = read_char(&mut stdin)? else {
            // EOF
            finish_line(&mut stdout, prompt, &buf, rendered)?;
            if buf.is_empty() {
                return Ok(None);
            } else {
                return Ok(Some(buf));
            }
        };
        match ch {
            '\r' | '\n' => {
                // if menu visible and selected, accept selection
                let m = matches(&buf, commands);
                if !m.is_empty() && buf.starts_with('/') && rendered {
                    // menu was visible on last draw; accept current selection
                    buf = m[selected].to_string();
                }
                finish_line(&mut stdout, prompt, &buf, rendered)?;
                return Ok(Some(buf));
            }
            '\u{3}' => {
                // Ctrl-C
                if rendered {
                    write!(stdout, "\x1b[1A\r\x1b[J")?;
                }
                writeln!(stdout, "{}^C", prompt)?;
                stdout.flush()?;
                return Ok(None);
            }
            '\u{4}' => {
                // Ctrl-D
                if buf.is_empty() {
                    if rendered {
                        write!(stdout, "\x1b[1A\r\x1b[J")?;
                    }
                    writeln!(stdout)?;
                    stdout.flush()?;
                    return Ok(None);
                }
            }
            '\u{7f}' | '\u{8}' => {
                // Backspace
                if cursor > 0 {
                    let ch_len = buf[..cursor]
                        .chars()
                        .next_back()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    cursor -= ch_len;
                    buf.remove(cursor);
                    selected = 0;
                    draw_box(
                        &mut stdout,
                        prompt,
                        &buf,
                        cursor,
                        selected,
                        commands,
                        &mut rendered,
                    )?;
                }
            }
            '\t' => {
                // Tab: cycle selection
                let m = matches(&buf, commands);
                if !m.is_empty() {
                    selected = (selected + 1) % m.len();
                    buf = m[selected].to_string();
                    cursor = buf.len();
                    draw_box(
                        &mut stdout,
                        prompt,
                        &buf,
                        cursor,
                        selected,
                        commands,
                        &mut rendered,
                    )?;
                }
            }
            '\u{1b}' => {
                // escape sequence: read two more bytes
                let mut seq = [0u8; 2];
                let _ = stdin.read(&mut seq);
                if seq[0] == b'[' {
                    match seq[1] {
                        b'A' => {
                            // up
                            let m = matches(&buf, commands);
                            if !m.is_empty() && selected > 0 {
                                selected -= 1;
                                draw_box(
                                    &mut stdout,
                                    prompt,
                                    &buf,
                                    cursor,
                                    selected,
                                    commands,
                                    &mut rendered,
                                )?;
                            }
                        }
                        b'B' => {
                            // down
                            let m = matches(&buf, commands);
                            if !m.is_empty() && selected + 1 < m.len() {
                                selected += 1;
                                draw_box(
                                    &mut stdout,
                                    prompt,
                                    &buf,
                                    cursor,
                                    selected,
                                    commands,
                                    &mut rendered,
                                )?;
                            }
                        }
                        _ => {}
                    }
                }
            }
            c if !c.is_control() => {
                buf.insert_str(cursor, &c.to_string());
                cursor += c.len_utf8();
                selected = 0;
                draw_box(
                    &mut stdout,
                    prompt,
                    &buf,
                    cursor,
                    selected,
                    commands,
                    &mut rendered,
                )?;
            }
            _ => {}
        }
    }
}
