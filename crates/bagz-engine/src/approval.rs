//! The human-approval gate. Every agent returns a [`Pending`] proposal.
//! Only [`Pending::approve`] makes an [`Approved`] value, and it needs a
//! named human. Code that acts on agent output takes `Approved<T>`, so the
//! compiler refuses to act on anything a person has not signed off.
//!
//! [`Queue`] holds proposals between the agent run and the human decision,
//! with an audit log of every decision.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Agent output waiting for a human decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pending<T> {
    pub agent: String,
    pub summary: String,
    pub payload: T,
}

/// Agent output a named human approved. The fields are private, so this
/// type is only made by [`Pending::approve`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Approved<T> {
    agent: String,
    approver: String,
    payload: T,
}

impl<T> Pending<T> {
    pub fn new(agent: impl Into<String>, summary: impl Into<String>, payload: T) -> Self {
        Self {
            agent: agent.into(),
            summary: summary.into(),
            payload,
        }
    }

    /// A human approves. An empty approver name is refused.
    pub fn approve(self, approver: &str) -> Result<Approved<T>> {
        let approver = approver.trim();
        if approver.is_empty() {
            return Err(Error::Guard("approval needs a named human".into()));
        }
        Ok(Approved {
            agent: self.agent,
            approver: approver.to_string(),
            payload: self.payload,
        })
    }

    /// A human edits the payload before deciding.
    pub fn edit(mut self, f: impl FnOnce(&mut T)) -> Self {
        f(&mut self.payload);
        self
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Pending<U> {
        Pending {
            agent: self.agent,
            summary: self.summary,
            payload: f(self.payload),
        }
    }
}

impl<T> Approved<T> {
    pub fn agent(&self) -> &str {
        &self.agent
    }
    pub fn approver(&self) -> &str {
        &self.approver
    }
    pub fn payload(&self) -> &T {
        &self.payload
    }
    pub fn into_payload(self) -> T {
        self.payload
    }
}

/// Where a queued proposal stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Approved,
    Rejected,
}

/// One line of the decision log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub id: u64,
    pub agent: String,
    pub status: Status,
    pub by: String,
    pub note: String,
}

#[derive(Debug, Clone)]
struct Slot<T> {
    id: u64,
    status: Status,
    proposal: Option<Pending<T>>,
}

/// A review queue for proposals of one payload type.
#[derive(Debug, Clone)]
pub struct Queue<T> {
    next: u64,
    slots: Vec<Slot<T>>,
    log: Vec<Decision>,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self {
            next: 1,
            slots: Vec::new(),
            log: Vec::new(),
        }
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, p: Pending<T>) -> u64 {
        let id = self.next;
        self.next += 1;
        self.slots.push(Slot {
            id,
            status: Status::Pending,
            proposal: Some(p),
        });
        id
    }

    pub fn status(&self, id: u64) -> Option<Status> {
        self.slots.iter().find(|s| s.id == id).map(|s| s.status)
    }

    /// Ids still waiting for a human.
    pub fn pending_ids(&self) -> Vec<u64> {
        self.slots
            .iter()
            .filter(|s| s.status == Status::Pending)
            .map(|s| s.id)
            .collect()
    }

    pub fn peek(&self, id: u64) -> Option<&Pending<T>> {
        self.slots
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.proposal.as_ref())
    }

    fn take(&mut self, id: u64) -> Result<(usize, Pending<T>)> {
        let idx = self
            .slots
            .iter()
            .position(|s| s.id == id)
            .ok_or(Error::UnknownProposal(id))?;
        if self.slots[idx].status != Status::Pending {
            return Err(Error::NotPending(id));
        }
        let p = self.slots[idx]
            .proposal
            .take()
            .ok_or(Error::NotPending(id))?;
        Ok((idx, p))
    }

    /// Approves a pending proposal and hands back the approved value. An
    /// empty approver name is refused and the proposal stays pending.
    pub fn approve(&mut self, id: u64, approver: &str) -> Result<Approved<T>> {
        if approver.trim().is_empty() {
            return Err(Error::Guard("approval needs a named human".into()));
        }
        let (idx, p) = self.take(id)?;
        let a = p.approve(approver)?;
        self.slots[idx].status = Status::Approved;
        self.log.push(Decision {
            id,
            agent: a.agent.clone(),
            status: Status::Approved,
            by: a.approver.clone(),
            note: String::new(),
        });
        Ok(a)
    }

    /// Rejects a pending proposal. The payload is dropped.
    pub fn reject(&mut self, id: u64, by: &str, note: &str) -> Result<()> {
        let (idx, p) = self.take(id)?;
        self.slots[idx].status = Status::Rejected;
        self.log.push(Decision {
            id,
            agent: p.agent,
            status: Status::Rejected,
            by: by.trim().to_string(),
            note: note.to_string(),
        });
        Ok(())
    }

    pub fn log(&self) -> &[Decision] {
        &self.log
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}
