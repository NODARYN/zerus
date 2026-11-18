//! Exact exit lookup and advisory archive evidence for the two Teams readers.
use super::*;
use std::time::{Duration, Instant};

fn check_root(state: &Path) -> Result<()> {
    if crate::platform::canonical(state).map_err(|e| e.to_string())?
        != crate::platform::canonical(root()).map_err(|e| e.to_string())?
    {
        return Err("Archive lookup belongs to another state root".into());
    }
    Ok(())
}

fn canonical_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|uuid| !uuid.is_nil() && uuid.to_string() == id)
}

/// A standing confirmation can use only the existing complete, unique run
/// lookup. Keep the result unknown on this pass even when repair was admitted.
pub(crate) fn find_run(state: &Path, name: &str, run: &str) -> Result<Option<String>> {
    check_root(state)?;
    let result = find_run_locked(name, run);
    if result.as_ref().is_err_and(|error| incomplete(error)) {
        refresh::request_for_team();
    }
    result
}

fn find_run_locked(name: &str, run: &str) -> Result<Option<String>> {
    let _guard =
        storage::writer_until(Instant::now() + Duration::from_millis(300))?.ok_or(INCOMPLETE)?;
    let db = pin(refresh::catalog_without_request()?)?;
    if refresh::proof(&db)? != db.directory {
        return Err(INCOMPLETE.into());
    }
    let mut query = sql(db.prepare(
        "SELECT substr(id,1,65) FROM archive_identities WHERE valid=1 AND name=? AND run=? ORDER BY id LIMIT 2",
    ))?;
    let ids = sql(query.query_map(params![name, run], |row| row.get::<_, String>(0)))?;
    let ids = sql(ids.collect::<rusqlite::Result<Vec<_>>>())?;
    if ids.len() > 1 || ids.iter().any(|id| !canonical_id(id)) {
        return Err("No exact unique exit archive".into());
    }
    if let Some(id) = ids.first() {
        let record = archive::read_archive(name, id)?;
        if record["run_id"] != run {
            return Err("The exact archive identity changed".into());
        }
    }
    current_proof(&db)?;
    Ok(ids.into_iter().next())
}

/// A sighting on disk is enough to explain a missing row to the person. It
/// conveys no authority to resume. A cold read has its own small inspection
/// budget and requests the worker even if no matching body fits that budget.
pub(crate) fn member_archive_pending(state: &Path, session: &str, launch: &str) -> Result<bool> {
    check_root(state)?;
    if !canonical_id(session) || !canonical_id(launch) {
        return Err(INCOMPLETE.into());
    }
    if refresh::catalog_without_request().is_ok() {
        return Ok(false);
    }
    refresh::request_for_team();
    native_archive_pending(session, launch)
}

fn native_archive_pending(session: &str, launch: &str) -> Result<bool> {
    const ENTRIES: usize = 64;
    const BYTES: u64 = 4 << 20;
    let until = Instant::now() + Duration::from_millis(25);
    let directory = refresh::directory()?;
    if directory == "absent" {
        return Ok(false);
    }
    let db = refresh::existing().ok();
    let mut remaining = BYTES;
    let mut complete = true;
    for (examined, entry) in fs::read_dir(root().join("archives"))
        .map_err(|e| e.to_string())?
        .enumerate()
    {
        if examined == ENTRIES || Instant::now() >= until || remaining == 0 {
            return Err(INCOMPLETE.into());
        }
        let path = entry.map_err(|e| e.to_string())?.path();
        let Some(id) = path
            .file_stem()
            .and_then(|id| id.to_str())
            .filter(|id| path.extension().is_some_and(|ext| ext == "json") && canonical_id(id))
        else {
            continue;
        };
        let Ok(stamp) = archive_references::current_stamp(&path) else {
            complete = false;
            continue;
        };
        if db
            .as_ref()
            .is_some_and(|db| refresh::has_identity(db, id, &stamp))
        {
            continue;
        }
        let size = archive_references::stamped_bytes(&stamp)?;
        if size > remaining {
            complete = false;
            continue;
        }
        remaining -= size;
        let Some(raw) = labels::read_file(&path, size).ok().flatten() else {
            complete = false;
            continue;
        };
        let Ok(record) = serde_json::from_slice::<Value>(&raw) else {
            complete = false;
            continue;
        };
        if archive::validate_record(string(&record, "name"), id, &record).is_err()
            || archive_references::current_stamp(&path).as_ref() != Ok(&stamp)
        {
            complete = false;
            continue;
        }
        if record["session_id"] == session
            && record["launch_id"] == launch
            && record["state"] == "archived"
            && refresh::directory()? == directory
        {
            return Ok(true);
        }
    }
    if complete && refresh::directory()? == directory {
        Ok(false)
    } else {
        Err(INCOMPLETE.into())
    }
}
