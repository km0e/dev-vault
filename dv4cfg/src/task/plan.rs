use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU32, Ordering},
    },
};

use async_trait::async_trait;
use tokio::sync::oneshot;
use tracing::{debug, error, info, trace};

use crate::{
    error::Result,
    into_boxed_task,
    op::{Context, ContextImpl},
};

use super::core::{BoxedTask, Target, Task, TaskStatus};

#[derive(Debug, Default)]
pub struct AlwaySuccess;

#[async_trait]
impl<I: ContextImpl> Task<I> for AlwaySuccess {
    async fn exec(&self, _target: &Target, _context: Arc<Context<I>>) -> Result<TaskStatus>
    where
        I: 'async_trait,
    {
        Ok(TaskStatus::Success)
    }
}

#[derive(Debug, Default)]
pub struct AlwayDoNothing;

#[async_trait]
impl<I: ContextImpl> Task<I> for AlwayDoNothing {
    async fn exec(&self, _target: &Target, _context: Arc<Context<I>>) -> Result<TaskStatus>
    where
        I: 'async_trait,
    {
        Ok(TaskStatus::DoNothing)
    }
}

#[derive(Debug, Default)]
pub struct AlwayFailed;

#[async_trait]
impl<I: ContextImpl> Task<I> for AlwayFailed {
    async fn exec(&self, _target: &Target, _context: Arc<Context<I>>) -> Result<TaskStatus>
    where
        I: 'async_trait,
    {
        Ok(TaskStatus::Failed)
    }
}

into_boxed_task!(AlwaySuccess, AlwayDoNothing, AlwayFailed);

#[derive(Debug)]
pub struct Plan<'a, I: ContextImpl> {
    pub final_: TaskNode<'a, I>,
    pub tasks: Vec<TaskNode<'a, I>>,
    pub start: Arc<StateNode>,
}

impl<'a, I: ContextImpl> Plan<'a, I> {
    /// first is start task, second is final task, rest is task
    pub fn new<'b, 'c>(
        nodes: impl IntoIterator<Item = (TaskNode<'a, I>, Arc<StateNode>)>,
        topo: impl IntoIterator<Item = (&'b String, &'b [String])>,
    ) -> Self {
        let mut iter = nodes.into_iter();
        let (mut start_task, start_state) = iter.next().unwrap();
        let (final_task, final_state) = iter.next().unwrap();
        let (mut tasks, mut states): (HashMap<_, _>, HashMap<_, _>) = iter
            .map(|(task, state)| {
                let id = task.id;
                info!("Add task {}", id);
                ((task.id, task), (id, state))
            })
            .unzip();
        for (id, next_tasks) in topo.into_iter() {
            if next_tasks.is_empty() {
                tasks.get_mut(id).unwrap().next.push(final_state.add_dep());
            }
            for next in next_tasks {
                let next_state_node = states
                    .get_mut(next)
                    .ok_or_else(|| format!("{} not found", next))
                    .unwrap();
                tasks
                    .get_mut(id)
                    .unwrap()
                    .next
                    .push(next_state_node.add_dep());
            }
        }
        for (id, state) in states {
            if state.free() {
                info!("Add start task {}", id);
                start_task.next.push(state.add_dep());
            }
        }
        let mut tasks = tasks.into_values().collect::<Vec<_>>();
        tasks.push(start_task);
        Self {
            final_: final_task,
            tasks,
            start: start_state,
        }
    }
    pub async fn run(self, context: Arc<Context<I>>) -> Vec<(&'a String, TaskStatus)> {
        let mut js = vec![];
        debug!("try spawn {} task", self.tasks.len());
        for task in self.tasks {
            let ctx = context.clone();
            js.push(task.exec(ctx));
        }

        Arc::try_unwrap(self.start)
            .unwrap()
            .tx
            .send(TaskStatus::Success as u8)
            .unwrap();

        let res = futures::future::join_all(js).await;
        debug!("all task join");
        res
    }
}

#[derive(Debug)]
pub struct TaskNode<'a, I: ContextImpl> {
    pub id: &'a String,
    pub target: Target,
    task: &'a BoxedTask<I>,
    pub next: Vec<Arc<StateNode>>,
    rx: oneshot::Receiver<u8>,
}

impl<'a, I: ContextImpl> TaskNode<'a, I> {
    pub fn new<'b: 'a>(id: &'a String, task: &'b BoxedTask<I>, rx: oneshot::Receiver<u8>) -> Self {
        Self {
            id,
            target: Target::default(),
            task,
            next: Vec::default(),
            rx,
        }
    }
    async fn exec(self, ctx: Arc<Context<I>>) -> (&'a String, TaskStatus) {
        ctx.get_interactor()
            .log(&format!("[Task] [Ready] {}", self.id))
            .await;
        trace!("[{}] wait for dep", self.id);
        let status = self
            .rx
            .await
            .map(|s| s.try_into().unwrap())
            .inspect_err(|e| {
                error!("[{}] dep error: {:?}", self.id, e);
            })
            .unwrap_or(TaskStatus::Failed);
        let status = match status {
            TaskStatus::Success => {
                ctx.get_interactor()
                    .log(&format!("[Task] [Exec ] {}", self.id))
                    .await;
                debug!("[{}] exec", self.id);
                self.task
                    .exec(&self.target, ctx.clone())
                    .await
                    .inspect_err(|e| {
                        error!("[{}] dep error: {:?}", self.id, e);
                    })
                    .unwrap_or(TaskStatus::Failed)
            }
            TaskStatus::Failed => {
                ctx.get_interactor()
                    .log(&format!("[Task] [Failed] {}", self.id))
                    .await;
                TaskStatus::Failed
            }
            TaskStatus::DoNothing => TaskStatus::DoNothing,
        };
        ctx.get_interactor()
            .log(&format!("[Task] [Over ] {} {}", self.id, &status))
            .await;
        for next in self.next {
            next.try_exec(&status);
        }
        debug!("enable next over");
        (self.id, status)
    }
}

#[derive(Debug)]
pub struct StateNode {
    status: AtomicU8,
    dep_count: AtomicU32,
    tx: oneshot::Sender<u8>,
}
impl StateNode {
    pub fn new(tx: oneshot::Sender<u8>) -> Self {
        Self {
            status: AtomicU8::new(0),
            dep_count: AtomicU32::new(0),
            tx,
        }
    }
    pub fn try_exec(self: Arc<Self>, s: &TaskStatus) {
        self.status.fetch_max((*s) as u8, Ordering::Release);
        let fetch_sub = self.dep_count.fetch_sub(1, Ordering::Release);
        if fetch_sub == 1 {
            let status = self.status.load(Ordering::Acquire);
            Arc::try_unwrap(self).unwrap().tx.send(status).unwrap();
        }
    }
    pub fn add_dep(self: &Arc<Self>) -> Arc<Self> {
        self.dep_count.fetch_add(1, Ordering::Release);
        self.clone()
    }
    pub fn free(&self) -> bool {
        self.dep_count.load(Ordering::Relaxed) == 0
    }
}

// #[cfg(test)]
// mod tests;
