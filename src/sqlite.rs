use std::path::Path;

/*
 * Re-export the SQLite crate we use for convenience:
 */
pub use rusqlite;

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use slog::{info, Logger};

pub struct SqliteSetup {
    log: Option<Logger>,
    schema: Option<String>,
    cache_kb: Option<u32>,
    create: bool,
    check_integrity: bool,
}

impl SqliteSetup {
    pub fn new() -> SqliteSetup {
        SqliteSetup {
            log: None,
            schema: None,
            cache_kb: None,
            create: false,
            check_integrity: true,
        }
    }

    pub fn create(&mut self, create: bool) -> &mut Self {
        self.create = create;
        self
    }

    pub fn check_integrity(&mut self, check_integrity: bool) -> &mut Self {
        self.check_integrity = check_integrity;
        self
    }

    pub fn schema<S: ToString>(&mut self, schema: S) -> &mut Self {
        self.schema = Some(schema.to_string());
        self
    }

    pub fn cache_kb(&mut self, kb: u32) -> &mut Self {
        self.cache_kb = Some(kb);
        self
    }

    pub fn log(&mut self, log: Logger) -> &mut Self {
        self.log = Some(log);
        self
    }

    pub fn open<P: AsRef<Path>>(&self, path: P) -> Result<Connection> {
        let path = path.as_ref();
        let log = self
            .log
            .as_ref()
            .map(|log| log.clone())
            .unwrap_or_else(|| crate::log::discard());

        let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE;
        if self.create {
            flags |= OpenFlags::SQLITE_OPEN_CREATE;
        }

        info!(log, "opening database {:?}", path);
        let mut c = Connection::open_with_flags(path, flags)
            .context("opening database")?;

        if self.check_integrity {
            let integrity: String = c
                .query_row("PRAGMA integrity_check", [], |row| Ok(row.get(0)?))
                .context("integrity check")?;
            if integrity.to_ascii_uppercase() != "OK" {
                bail!("integrity check failure: {integrity:?}");
            }
            info!(log, "database integrity ok");
        }

        /*
         * Enable foreign key processing, which is off by default.  Without
         * enabling this, there is no referential integrity check between
         * primary and foreign keys in tables.
         */
        c.execute("PRAGMA foreign_keys = 'ON'", [])
            .context("enable foreign keys")?;

        /*
         * Enable the WAL.
         */
        let new_mode: String = c
            .query_row("PRAGMA journal_mode = 'WAL'", [], |row| Ok(row.get(0)?))
            .context("enable WAL mode")?;
        if new_mode.to_ascii_uppercase() != "WAL" {
            bail!("could not set journal mode to WAL (stuck in {new_mode:?})");
        }

        /*
         * If requested, set the page cache size to something other than the
         * default value of 2MB.
         */
        if let Some(kb) = self.cache_kb {
            c.execute(&format!("PRAGMA cache_size = -{kb}"), [])
                .context("set cache size")?;
        }

        if let Some(schema) = self.schema.as_deref() {
            /*
             * Take the schema file and split it on the special comments we use
             * to separate statements.
             */
            let mut steps: Vec<(u32, String)> = Vec::new();
            let mut version = None;
            let mut statement = String::new();

            for l in schema.lines() {
                if l.starts_with("-- v ") {
                    if let Some(version) = version.take() {
                        steps.push((version, statement.trim().to_string()));
                    }

                    version = Some(l.trim_start_matches("-- v ").parse()?);
                    statement.clear();
                } else {
                    statement.push_str(l);
                    statement.push('\n');
                }
            }
            if let Some(version) = version.take() {
                steps.push((version, statement.trim().to_string()));
            }

            /*
             * Get the current schema version before we start:
             */
            let uv: u32 = c
                .query_row("PRAGMA user_version", [], |row| Ok(row.get(0)?))
                .context("get user version")?;
            info!(log, "found user version {} in database", uv);

            for (version, statement) in steps {
                /*
                 * Do some whitespace normalisation.  We would prefer to keep
                 * the whitespace-heavy layout of the schema as represented in
                 * the file, as SQLite will preserve it in the ".schema" output.
                 * Unfortunately, there is no obvious way to ALTER TABLE ADD
                 * COLUMN in a way that similarly maintains the whitespace, so
                 * we will instead uniformly do without.
                 */
                let mut statement = statement.replace('\n', " ");
                while statement.contains("( ") {
                    statement = statement.trim().replace("( ", "(");
                }
                while statement.contains(" )") {
                    statement = statement.trim().replace(" )", ")");
                }
                while statement.contains("  ") {
                    statement = statement.trim().replace("  ", " ");
                }

                /*
                 * Perform the version check, statement execution, and version
                 * update inside a single transaction.  Not all of the things we
                 * could do in a statement are transactional, but if we are
                 * doing an INSERT SELECT to copy things from one table to
                 * another, we don't want that to conk out half way and run
                 * again.
                 */
                let tx = c.transaction_with_behavior(
                    TransactionBehavior::Immediate,
                )?;

                /*
                 * Determine the current user version.
                 */
                let uv: u32 = tx
                    .query_row("PRAGMA user_version", [], |row| Ok(row.get(0)?))
                    .context("get user version again")?;

                if version > uv {
                    info!(log, "apply version {}, run {}", version, statement);

                    let uc = tx
                        .execute(&statement, [])
                        .context("apply schema change")?;
                    info!(log, "updated {uc} rows");

                    /*
                     * Update the user version.
                     */
                    tx.execute(&format!("PRAGMA user_version = {version}"), [])
                        .context("set user version")?;
                    info!(log, "version {} ok", version);
                }

                tx.commit()?;
            }
        }

        Ok(c)
    }
}
