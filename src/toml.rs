use std::io::{Write, Read, BufReader, BufWriter};
use std::fs::File;
use std::path::Path;

use serde::{Serialize, Deserialize};

use tempfile::NamedTempFile;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/**
 * Read a TOML-formatted file into an object via serde deserialisation.
 */
pub fn read_file<T, P: AsRef<Path>>(p: P) -> Result<T>
where
    for<'de> T: Deserialize<'de>
{
    let f = File::open(p)?;
    let mut r = BufReader::new(f);
    let mut buf = Vec::<u8>::new();

    r.read_to_end(&mut buf)?;

    Ok(toml::from_slice(&buf)?)
}

/**
 * Write a TOML-formatted file from an object via serde serialisation.
 */
pub fn write_file<T, P: AsRef<Path>>(p: P, o: &T) -> Result<()>
where
    T: Serialize
{
    let p = p.as_ref();
    let o = toml::to_vec(o)?;

    /*
     * In order to safely and atomically update the file with the new contents,
     * we will first write a temporary file.  If this write fails, we will not
     * have damaged the original.
     */
    let dir = p.parent().expect("no parent directory?!");
    let tf = NamedTempFile::new_in(dir)?;
    println!("tempfile: {}", tf.path().display());

    {
        let mut w = BufWriter::new(tf.as_file());
        w.write_all(&o)?;
        w.flush()?;
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
            return Err("fsync failure".into());
        }
    }

    /*
     * Move the temporary file to the target name, replacing any existing file
     * atomically.
     * XXX We should probably fsync(2) the directory after we perform the
     * rename.
     */
    tf.persist(p)?;
    Ok(())
}
