use crate::db::DbPool;

pub struct AppState {
    pub(crate) db: DbPool,
}
