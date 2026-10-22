use crate::db::storage::open_database;
#[cfg(test)]
use crate::db::storage::prepare_connection;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::Mutex;

pub struct ContentIndexRepository {
    connection: Mutex<Connection>,
}

pub struct IndexCandidate {
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
    pub snapshot_json: String,
    pub sources: Option<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IndexedFormats {
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
    pub formats: Vec<String>,
}

impl ContentIndexRepository {
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

    pub fn candidates(&self, limit: u32) -> Result<Vec<IndexCandidate>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT post.service, post.creator_id, post.post_id, post.snapshot_json,
                        known.sources, known.complete
                 FROM posts post
                 LEFT JOIN post_inner_formats known
                   ON known.service=post.service AND known.creator_id=post.creator_id
                  AND known.post_id=post.post_id
                 WHERE (
                   EXISTS (SELECT 1 FROM collection_posts item
                           WHERE item.service=post.service AND item.creator_id=post.creator_id
                             AND item.post_id=post.post_id)
                   OR EXISTS (SELECT 1 FROM content_pins pin
                              WHERE pin.entity_kind='post' AND pin.reason='favorite'
                                AND pin.service=post.service AND pin.creator_id=post.creator_id
                                AND pin.post_id=post.post_id)
                 )
                 AND (
                   known.post_id IS NULL
                   OR datetime(post.cached_at) > datetime(known.checked_at)
                   OR (known.complete=0
                       AND datetime(known.checked_at) < datetime('now', '-1 day'))
                 )
                 LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([limit], |row| {
                Ok(IndexCandidate {
                    service: row.get(0)?,
                    creator_id: row.get(1)?,
                    post_id: row.get(2)?,
                    snapshot_json: row.get(3)?,
                    sources: row.get(4)?,
                    complete: row.get::<_, Option<bool>>(5)?.unwrap_or(false),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    pub fn record(
        &self,
        candidate: &IndexCandidate,
        sources: &str,
        formats: &[String],
        complete: bool,
    ) -> Result<(), String> {
        let formats = serde_json::to_string(formats).map_err(|e| e.to_string())?;
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO post_inner_formats(service, creator_id, post_id, sources, formats, complete)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(service, creator_id, post_id) DO UPDATE SET
                   sources=excluded.sources, formats=excluded.formats,
                   complete=excluded.complete, checked_at=CURRENT_TIMESTAMP",
                params![
                    candidate.service,
                    candidate.creator_id,
                    candidate.post_id,
                    sources,
                    formats,
                    complete
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn touch(&self, candidate: &IndexCandidate) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "UPDATE post_inner_formats SET checked_at=CURRENT_TIMESTAMP
                 WHERE service=?1 AND creator_id=?2 AND post_id=?3",
                params![candidate.service, candidate.creator_id, candidate.post_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<IndexedFormats>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT service, creator_id, post_id, formats FROM post_inner_formats
                 WHERE formats <> '[]'",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                let formats: String = row.get(3)?;
                Ok(IndexedFormats {
                    service: row.get(0)?,
                    creator_id: row.get(1)?,
                    post_id: row.get(2)?,
                    formats: serde_json::from_str(&formats).unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(repository: &ContentIndexRepository) {
        let connection = repository.connection.lock().unwrap();
        connection
            .execute_batch(
                "INSERT INTO creators(service, creator_id, name, snapshot_json) VALUES('patreon', 'c1', 'c', '{}');
                 INSERT INTO posts(service, creator_id, post_id, title, snapshot_json, cached_at)
                   VALUES('patreon', 'c1', 'saved', 't', '{}', '2026-01-01 00:00:00'),
                         ('patreon', 'c1', 'loose', 't', '{}', '2026-01-01 00:00:00');
                 INSERT INTO collection_posts(collection_id, service, creator_id, post_id, operation_id)
                   VALUES('00000000-0000-0000-0000-000000000001', 'patreon', 'c1', 'saved', 'op');",
            )
            .unwrap();
    }

    #[test]
    fn only_saved_posts_are_candidates_until_recorded() {
        let repository = ContentIndexRepository::in_memory().unwrap();
        seed(&repository);
        let candidates = repository.candidates(10).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].post_id, "saved");

        repository
            .record(&candidates[0], "a.zip", &["video".to_string()], true)
            .unwrap();
        assert!(repository.candidates(10).unwrap().is_empty());
        assert_eq!(
            repository.list().unwrap(),
            vec![IndexedFormats {
                service: "patreon".into(),
                creator_id: "c1".into(),
                post_id: "saved".into(),
                formats: vec!["video".into()],
            }]
        );
    }

    #[test]
    fn a_newer_snapshot_makes_the_post_a_candidate_again() {
        let repository = ContentIndexRepository::in_memory().unwrap();
        seed(&repository);
        let candidate = repository.candidates(10).unwrap().remove(0);
        repository.record(&candidate, "", &[], true).unwrap();
        repository
            .connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE posts SET cached_at=datetime('now', '+1 minute') WHERE post_id='saved'",
                [],
            )
            .unwrap();
        let again = repository.candidates(10).unwrap();
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].sources.as_deref(), Some(""));
        assert!(again[0].complete);
    }
}
