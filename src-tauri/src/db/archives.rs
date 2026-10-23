use crate::db::storage::open_database;
#[cfg(test)]
use crate::db::storage::prepare_connection;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;

const CREATOR_PASSWORD_LIMIT: u32 = 10;

pub struct ArchivePasswordRepository {
    connection: Mutex<Connection>,
}

impl ArchivePasswordRepository {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            connection: Mutex::new(open_database()?),
        })
    }

    #[cfg(test)]
    fn in_memory() -> Result<Self, String> {
        let mut connection = Connection::open_in_memory().map_err(|e| e.to_string())?;
        prepare_connection(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn password_for_archive(&self, archive_id: &str) -> Result<Option<String>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                "SELECT password FROM archive_passwords WHERE archive_id=?1",
                [archive_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn source_for_archive(&self, archive_id: &str) -> Result<Option<String>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                "SELECT source FROM archive_passwords WHERE archive_id=?1",
                [archive_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map(Option::flatten)
            .map_err(|e| e.to_string())
    }

    pub fn creator_passwords(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<String>, String> {
        // Keyless Mega links store their keys without a creator; never offer those as passwords.
        if creator_id.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT password FROM archive_passwords
                 WHERE service=?1 AND creator_id=?2
                 GROUP BY password
                 ORDER BY MAX(updated_at) DESC
                 LIMIT ?3",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(
                params![service, creator_id, CREATOR_PASSWORD_LIMIT],
                |row| row.get::<_, String>(0),
            )
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn remember(
        &self,
        archive_id: &str,
        service: &str,
        creator_id: &str,
        password: &str,
        source: &str,
    ) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO archive_passwords(archive_id, service, creator_id, password, source)
                 VALUES(?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(archive_id) DO UPDATE SET
                   service=excluded.service, creator_id=excluded.creator_id,
                   password=excluded.password, source=excluded.source,
                   updated_at=strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
                params![archive_id, service, creator_id, password, source],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creator_passwords_are_distinct_and_newest_first() {
        let repo = ArchivePasswordRepository::in_memory().unwrap();
        for (archive, service, password) in [
            ("a", "patreon", "old"),
            ("b", "patreon", "new"),
            ("c", "patreon", "new"),
            ("d", "fanbox", "other"),
        ] {
            repo.remember(archive, service, "1", password, "").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert_eq!(
            repo.creator_passwords("patreon", "1").unwrap(),
            vec!["new".to_string(), "old".to_string()]
        );
    }
}
