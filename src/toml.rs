use std::io::{Write, Read, BufReader, BufWriter, ErrorKind};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::error::Error;

use serde::{Serialize, Deserialize};

use tempfile::NamedTempFile;

type Result<T> = std::result::Result<T, TomlError>;

#[derive(Debug)]
pub struct TomlError {
    msg: String,
    path: PathBuf,
}

impl std::fmt::Display for TomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TOML file \"{}\": {}", self.path.display(), self.msg)
    }
}

impl Error for TomlError {
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
    where E: Error
{
    fn context(self, path: &Path) -> Result<T> {
        self.map_err(|e| TomlError {
            path: path.to_path_buf(),
            msg: e.to_string(),
        })
    }
}

pub trait Context<T, E> {
    fn context(self, path: &Path) -> Result<T>;
}

fn e(msg: &str, path: &Path) -> TomlError {
    TomlError {
        path: path.to_path_buf(),
        msg: msg.to_string(),
    }
}

/**
 * Read a TOML-formatted file into an optional object via serde deserialisation.
 * If the requested file does not exist, None will be returned.
 */
pub fn read_file<T, P: AsRef<Path>>(p: P) -> Result<Option<T>>
where
    for<'de> T: Deserialize<'de>
{
    let p = p.as_ref();

    let f = match File::open(p) {
        Err(e) => match e.kind() {
            ErrorKind::NotFound => return Ok(None),
            _ => return Err(e).context(p),
        }
        Ok(f) => f,
    };
    let mut r = BufReader::new(f);
    let mut buf = Vec::<u8>::new();

    r.read_to_end(&mut buf).context(p)?;

    Ok(Some(toml::from_slice(&buf).context(p)?))
}

/**
 * Write a TOML-formatted file from an object via serde serialisation.
 */
pub fn write_file<T, P: AsRef<Path>>(p: P, o: &T) -> Result<()>
where
    T: Serialize
{
    let p = p.as_ref();
    let o = toml::to_vec(o).context(p)?;

    /*
     * In order to safely and atomically update the file with the new contents,
     * we will first write a temporary file.  If this write fails, we will not
     * have damaged the original.
     */
    let dir = p.parent().expect("no parent directory?!");
    let tf = NamedTempFile::new_in(dir).context(p)?;

    {
        let mut w = BufWriter::new(tf.as_file());
        w.write_all(&o).context(p)?;
        w.flush().context(p)?;
    }

    #[cfg(unix)]
    {
        /*
         * Use fsync(2) to ensure the file is completely written to the backing
         * store.
         */
        use std::os::unix::io::AsRawFd;
        let fd = tf.as_raw_fd();
        if unsafe { libc::fsync(fd) } != 0 {
            return Err(e("fsync failure", p));
        }
    }

    /*
     * Move the temporary file to the target name, replacing any existing file
     * atomically.
     * XXX We should probably fsync(2) the directory after we perform the
     * rename.
     */
    tf.persist(p).context(p)?;
    Ok(())
}
