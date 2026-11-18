//! The evidence beside a task's decision (zerus-5gcbh): the task's latest
//! report and its stages, newest first, each with its latest verdict, read
//! from the rows the report and verdict writers stored. Bounded: one report
//! and at most [`MAX_STAGES`] stages, with `more_stages` beyond them.
use super::*;
use crate::teams::ops::ReportNote;
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

/// The most stages one read returns (rounds times stages of a pipeline).
pub const MAX_STAGES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportEvidence {
    pub generation: u32,
    pub text: String,
    pub commit: String,
    pub notes: Vec<ReportNote>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictEvidence {
    pub generation: u32,
    /// The verdict's writer as the store encoded it (a proved stage holder).
    pub actor: Value,
    pub pass: bool,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEvidence {
    pub node: Node,
    pub report: Option<ReportEvidence>,
    /// Stage nodes of the task, newest first, with each one's latest verdict.
    pub stages: Vec<(Node, Option<VerdictEvidence>)>,
    pub more_stages: bool,
}

impl Store {
    /// A task's evidence in one read transaction. A stage node has no
    /// evidence of its own: its verdict is read with its task.
    pub fn task_evidence(&self, team: &str, id: NodeId) -> Result<TaskEvidence> {
        self.active()?;
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| Problem::new(code::NOT_FOUND, "Team does not exist."))?;
        let tx = db.unchecked_transaction().map_err(storage)?;
        let node = rows::node(&tx, team, id)?
            .ok_or_else(|| Problem::new(code::NOT_FOUND, "Task does not exist in this team."))?;
        if node.kind == NodeKind::Stage {
            return Err(Problem::new(
                code::INVALID_STATE,
                "A stage's verdict is read with its task.",
            ));
        }
        let report = tx
            .query_row(
                "SELECT generation,text,commit_id,notes FROM reports WHERE team=?1 AND node=?2 ORDER BY generation DESC LIMIT 1",
                params![team, id],
                |r| {
                    Ok(ReportEvidence {
                        generation: r.get(0)?,
                        text: r.get(1)?,
                        commit: r.get(2)?,
                        notes: sql_json(r, 3)?,
                    })
                },
            )
            .optional()
            .map_err(storage)?;
        let mut statement = tx
            .prepare("SELECT node FROM stage_tasks WHERE team=?1 AND target=?2 ORDER BY node DESC LIMIT ?3")
            .map_err(storage)?;
        let ids = statement
            .query_map(params![team, id, MAX_STAGES as i64 + 1], |r| {
                r.get::<_, u32>(0)
            })
            .map_err(storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(storage)?;
        drop(statement);
        let more_stages = ids.len() > MAX_STAGES;
        let mut stages = Vec::with_capacity(ids.len().min(MAX_STAGES));
        for stage in ids.into_iter().take(MAX_STAGES) {
            // A wanted stage task has no node yet: only created stages show.
            let Some(stage_node) = rows::node(&tx, team, stage)? else {
                continue;
            };
            let verdict = tx
                .query_row(
                    "SELECT generation,actor,pass,text FROM verdicts WHERE team=?1 AND node=?2 ORDER BY generation DESC LIMIT 1",
                    params![team, stage],
                    |r| {
                        Ok(VerdictEvidence {
                            generation: r.get(0)?,
                            actor: sql_json(r, 1)?,
                            pass: r.get(2)?,
                            text: r.get(3)?,
                        })
                    },
                )
                .optional()
                .map_err(storage)?;
            stages.push((stage_node, verdict));
        }
        tx.commit().map_err(storage)?;
        Ok(TaskEvidence {
            node,
            report,
            stages,
            more_stages,
        })
    }
}
