use crate::Result;
use crate::db;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::path::{Path, PathBuf};

const SYNC_LEASE_LOCK_NAME: &str = "sync";
const MAX_LEASE_DURATION_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub struct LeaderElection {
    db_path: PathBuf,
}

#[derive(Debug)]
struct LeaseRecord {
    holder_id: String,
    expires_at_ms: i64,
    epoch: i64,
}

impl LeaderElection {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }

    // T-4.1: RTM FR-30.1 / FR-30.2 / FR-30.3 / FR-30.4
    pub fn try_acquire(
        &self,
        holder_id: &str,
        lease_duration_secs: u64,
    ) -> Result<(bool, &'static str)> {
        let mut connection = db::open_connection(&self.db_path)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now_ms = current_time_ms();
        // T-4.6: RTM FR-4.2
        let expires_at_ms = now_ms.saturating_add(lease_duration_ms(lease_duration_secs));
        let current = load_current_lease(&tx)?;

        let outcome = match current {
            None => {
                tx.execute(
                    "INSERT INTO sync_leader_lease (
                         lock_name,
                         holder_id,
                         epoch,
                         lease_expires_at_ms,
                         updated_at_ms
                     ) VALUES (?1, ?2, 1, ?3, ?4)",
                    params![SYNC_LEASE_LOCK_NAME, holder_id, expires_at_ms, now_ms],
                )?;
                (true, "acquired")
            }
            Some(current) if current.expires_at_ms <= now_ms => {
                tx.execute(
                    "UPDATE sync_leader_lease
                    SET holder_id = ?2,
                         epoch = ?3,
                         lease_expires_at_ms = ?4,
                         updated_at_ms = ?5
                     WHERE lock_name = ?1",
                    params![
                        SYNC_LEASE_LOCK_NAME,
                        holder_id,
                        current.epoch.saturating_add(1),
                        expires_at_ms,
                        now_ms
                    ],
                )?;
                (true, "acquired")
            }
            Some(current) if current.holder_id == holder_id => {
                tx.execute(
                    "UPDATE sync_leader_lease
                     SET lease_expires_at_ms = ?3,
                         updated_at_ms = ?4
                     WHERE lock_name = ?1
                       AND holder_id = ?2",
                    params![SYNC_LEASE_LOCK_NAME, holder_id, expires_at_ms, now_ms],
                )?;
                (true, "renewed")
            }
            Some(_) => (false, "held_by_other"),
        };

        tx.commit()?;
        Ok(outcome)
    }

    // T-4.6: RTM FR-4.2
    pub fn force_acquire(
        &self,
        holder_id: &str,
        lease_duration_secs: u64,
    ) -> Result<(bool, &'static str)> {
        let mut connection = db::open_connection(&self.db_path)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now_ms = current_time_ms();
        let expires_at_ms = now_ms.saturating_add(lease_duration_ms(lease_duration_secs));
        let outcome = match load_current_lease(&tx)? {
            None => {
                tx.execute(
                    "INSERT INTO sync_leader_lease (
                         lock_name,
                         holder_id,
                         epoch,
                         lease_expires_at_ms,
                         updated_at_ms
                     ) VALUES (?1, ?2, 1, ?3, ?4)",
                    params![SYNC_LEASE_LOCK_NAME, holder_id, expires_at_ms, now_ms],
                )?;
                (true, "acquired")
            }
            Some(current) if current.holder_id == holder_id => {
                tx.execute(
                    "UPDATE sync_leader_lease
                     SET lease_expires_at_ms = ?3,
                         updated_at_ms = ?4
                     WHERE lock_name = ?1
                       AND holder_id = ?2",
                    params![SYNC_LEASE_LOCK_NAME, holder_id, expires_at_ms, now_ms],
                )?;
                (true, "renewed")
            }
            Some(current) => {
                tx.execute(
                    "UPDATE sync_leader_lease
                     SET holder_id = ?2,
                         epoch = ?3,
                         lease_expires_at_ms = ?4,
                         updated_at_ms = ?5
                     WHERE lock_name = ?1",
                    params![
                        SYNC_LEASE_LOCK_NAME,
                        holder_id,
                        current.epoch.saturating_add(1),
                        expires_at_ms,
                        now_ms
                    ],
                )?;
                (true, "force_acquired")
            }
        };
        tx.commit()?;
        Ok(outcome)
    }

    // T-4.1: RTM FR-30.5 / FR-30.6
    pub fn try_renew(&self, holder_id: &str, lease_duration_secs: u64) -> Result<bool> {
        let mut connection = db::open_connection(&self.db_path)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now_ms = current_time_ms();
        let expires_at_ms = now_ms.saturating_add(lease_duration_ms(lease_duration_secs));
        let renewed = match load_current_lease(&tx)? {
            Some(current) if current.holder_id == holder_id && current.expires_at_ms > now_ms => {
                tx.execute(
                    "UPDATE sync_leader_lease
                     SET lease_expires_at_ms = ?3,
                         updated_at_ms = ?4
                     WHERE lock_name = ?1
                       AND holder_id = ?2",
                    params![SYNC_LEASE_LOCK_NAME, holder_id, expires_at_ms, now_ms],
                )?;
                true
            }
            _ => false,
        };
        tx.commit()?;
        Ok(renewed)
    }

    // T-4.1: RTM FR-30.7
    pub fn release(&self, holder_id: &str) -> Result<()> {
        let mut connection = db::open_connection(&self.db_path)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            // T-4.6: RTM FR-30.7
            "DELETE FROM sync_leader_lease
              WHERE lock_name = ?1
                AND holder_id = ?2",
            params![SYNC_LEASE_LOCK_NAME, holder_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    // T-4.1: RTM FR-30.8
    pub fn is_leader(&self, holder_id: &str) -> Result<bool> {
        let connection = db::open_connection(&self.db_path)?;
        let now_ms = current_time_ms();
        let is_leader = connection.query_row(
            "SELECT EXISTS(
                 SELECT 1
                 FROM sync_leader_lease
                 WHERE lock_name = ?1
                   AND holder_id = ?2
                   AND lease_expires_at_ms > ?3
             )",
            params![SYNC_LEASE_LOCK_NAME, holder_id, now_ms],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(is_leader != 0)
    }
}

fn load_current_lease(connection: &rusqlite::Transaction<'_>) -> Result<Option<LeaseRecord>> {
    Ok(connection
        .query_row(
            "SELECT holder_id, lease_expires_at_ms, epoch
             FROM sync_leader_lease
             WHERE lock_name = ?1",
            params![SYNC_LEASE_LOCK_NAME],
            |row| {
                Ok(LeaseRecord {
                    holder_id: row.get(0)?,
                    expires_at_ms: row.get(1)?,
                    epoch: row.get(2)?,
                })
            },
        )
        .optional()?)
}

fn current_time_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn lease_duration_ms(lease_duration_secs: u64) -> i64 {
    i64::try_from(clamp_lease_duration_secs(lease_duration_secs))
        .unwrap_or(i64::MAX / 1000)
        .saturating_mul(1000)
}

fn clamp_lease_duration_secs(lease_duration_secs: u64) -> u64 {
    lease_duration_secs.min(MAX_LEASE_DURATION_SECS)
}
