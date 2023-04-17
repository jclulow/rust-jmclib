use std::ffi::OsStr;
use std::io::Result;
use std::path::Component;
use std::path::{Path, PathBuf};

fn discard_n(pb: &mut PathBuf, n: u32) {
    for _ in 0..n {
        assert!(pb.pop());
    }
}

fn pc(s: &str) -> Component {
    Component::Normal(OsStr::new(s))
}

fn ioe<T, S: AsRef<str>>(s: S) -> Result<T> {
    Err(std::io::Error::new(std::io::ErrorKind::Other, s.as_ref()))
}

/**
 * Obtain a fully qualified path to the project root directory.
 *
 * If the program is running from a Cargo target directory, e.g.,
 * "/code/blah/target/release/program", then the project root directory is
 * "/code/blah".
 *
 * If running from an apparent deployed location, e.g.,
 * "/opt/software/bin/program", then the project root directory is one level up
 * from "bin", e.g., "/opt/software".
 */
pub fn rootdir() -> Result<PathBuf> {
    let mut exe = std::env::current_exe()?;

    /*
     * Discard the last component of the path (the executable file) so that we
     * can reason about the directory structure in which it is found.
     */
    let last: Vec<Component> = exe.components().rev().skip(1).collect();

    if last.len() >= 2 {
        if (last[0] == pc("debug") || last[0] == pc("release"))
            && last[1] == pc("target")
        {
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

    ioe(format!(
        "could not determine data directory relative to executive path \"{}\"",
        exe.display(),
    ))
}

/**
 * Obtain a fully qualified path to a project root-relative path.
 *
 * For example, if the program is "/opt/software/bin/program", and the provided
 * path is "etc/config.toml", the result will be
 * "/opt/software/etc/config.toml".
 *
 * See rootdir() for more details on project root directories.
 */
pub fn rootpath<P: AsRef<Path>>(p: P) -> Result<PathBuf> {
    let p = p.as_ref();
    if !p.is_relative() {
        return ioe(format!("path \"{}\" is not relative", p.display()));
    }

    let mut path = rootdir()?;
    path.push(p);
    Ok(path)
}
