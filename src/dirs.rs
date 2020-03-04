use std::path::PathBuf;
use std::path::Component;
use std::ffi::OsStr;
use std::io::Result;

fn discard_n(pb: &mut PathBuf, n: u32) {
    for _ in 0..n {
        assert!(pb.pop());
    }
}

fn pc(s: &str) -> Component {
    Component::Normal(OsStr::new(s))
}

pub fn rootdir() -> Result<PathBuf> {
    let mut exe = std::env::current_exe()?;

    /*
     * Discard the last component of the path (the executable file) so that we
     * can reason about the directory structure in which it is found.
     */
    let last: Vec<Component> = exe.components().rev().skip(1).collect();

    if last.len() >= 2 {
        if (last[0] == pc("debug") || last[0] == pc("release")) &&
            last[1] == pc("target") {
            /*
             * Cargo target directory.
             */
            discard_n(&mut exe, 3);
            return Ok(exe);
        }
    }

    if last.len() >= 1 {
        if last[0] == pc("bin") {
            /*
             * Program shipped in bin/ directory.
             */
            discard_n(&mut exe, 2);
            return Ok(exe);
        }
    }

    Err(std::io::Error::new(std::io::ErrorKind::Other,
        format!("could not determine data directory relative to executive \
        path \"{}\"", exe.display())))
}
