/// Lightweight SQLite connection pool backed purely by rusqlite + std::sync.
///
/// Up to `max_size` connections are opened lazily and returned to the pool on drop.
/// This avoids the rusqlite version conflict that external crates (r2d2_sqlite,
/// deadpool-sqlite) introduce.
use rusqlite::Connection;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

const POOL_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

struct Inner {
    idle: Vec<Connection>,
    size: usize,
    max_size: usize,
}

/// A shareable SQLite connection pool.
#[derive(Clone)]
pub(crate) struct DbPool {
    inner: Arc<(Mutex<Inner>, Condvar)>,
    path: PathBuf,
    init_sql: &'static str,
}

impl DbPool {
    pub(crate) fn new(
        path: PathBuf,
        max_size: usize,
        init_sql: &'static str,
    ) -> Result<Self, String> {
        let pool = DbPool {
            inner: Arc::new((
                Mutex::new(Inner {
                    idle: Vec::with_capacity(max_size),
                    size: 0,
                    max_size,
                }),
                Condvar::new(),
            )),
            path,
            init_sql,
        };
        // Eagerly open one connection to validate the path/schema.
        let _conn = pool.get()?;
        Ok(pool)
    }

    fn open_conn(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.path).map_err(|e| e.to_string())?;
        conn.execute_batch(self.init_sql)
            .map_err(|e| e.to_string())?;
        Ok(conn)
    }

    pub(crate) fn get(&self) -> Result<PooledConnection, String> {
        let (lock, cvar) = &*self.inner;
        let conn = {
            let mut guard = lock.lock().map_err(|e| e.to_string())?;
            loop {
                if let Some(c) = guard.idle.pop() {
                    break c;
                }
                if guard.size < guard.max_size {
                    guard.size += 1;
                    drop(guard);
                    match self.open_conn() {
                        Ok(c) => break c,
                        Err(e) => {
                            // Roll back the size increment on failure.
                            let mut g = lock.lock().map_err(|e| e.to_string())?;
                            g.size -= 1;
                            return Err(e);
                        }
                    }
                } else {
                    let (g, timed_out) = cvar
                        .wait_timeout(guard, POOL_WAIT_TIMEOUT)
                        .map_err(|e| e.to_string())?;
                    guard = g;
                    if timed_out.timed_out() {
                        return Err("db pool timeout: no connection available after 5s".to_string());
                    }
                }
            }
        };
        Ok(PooledConnection {
            conn: Some(conn),
            pool: self.clone(),
        })
    }

    pub(crate) fn cache_key(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }

    fn return_conn(&self, conn: Connection) {
        let (lock, cvar) = &*self.inner;
        if let Ok(mut guard) = lock.lock() {
            guard.idle.push(conn);
        }
        cvar.notify_one();
    }
}

/// An RAII guard that returns the connection to the pool on drop.
pub(crate) struct PooledConnection {
    conn: Option<Connection>,
    pool: DbPool,
}

impl Deref for PooledConnection {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("connection already returned")
    }
}

impl DerefMut for PooledConnection {
    fn deref_mut(&mut self) -> &mut Connection {
        self.conn.as_mut().expect("connection already returned")
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(c) = self.conn.take() {
            self.pool.return_conn(c);
        }
    }
}
