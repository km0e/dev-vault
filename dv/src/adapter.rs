use std::{collections::HashMap, iter::repeat_with, sync::Arc};

use dev_vault::{
    op::ContextImpl,
    task::{AlwaySuccess, BoxedTask, Plan, StateNode, TaskNode},
    user::UserFilter,
};
use tokio::sync::oneshot;
use tracing::info;

use crate::config::{Cite, Target};

#[derive(Debug)]
pub struct TaskParts<I> {
    pub id: String,
    pub next: Vec<String>,
    pub target: Target,
    pub task: BoxedTask<I>,
}

impl<I> TaskParts<I> {
    pub fn new(id: impl Into<String>, task: BoxedTask<I>) -> Self {
        Self {
            id: id.into(),
            target: Target::default(),
            next: Vec::default(),
            task,
        }
    }
    fn to_nodes(&self) -> (TaskNode<I>, Arc<StateNode>)
    where
        I: ContextImpl,
    {
        let (tx, rx) = oneshot::channel();
        let mut task = TaskNode::new(&self.id, &self.task, rx);
        task.target <<= self.target.cast();
        let state = StateNode::new(tx);
        let state = Arc::new(state);
        (task, state)
    }
}

pub struct GroupParts<I: ContextImpl> {
    dummy: TaskParts<I>,
    pub tasks: Vec<TaskParts<I>>,
    pub cites: Vec<Cite>,
}

impl<I: ContextImpl> GroupParts<I> {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            dummy: TaskParts::new(id, AlwaySuccess.into()),
            tasks: Vec::new(),
            cites: Vec::new(),
        }
    }
    pub fn id(&self) -> &String {
        &self.dummy.id
    }
}

impl<I: ContextImpl> GroupParts<I> {
    pub fn cast<'a, 'b: 'a, 'c: 'a, 'd>(
        &'a self,
        groups: &'c HashMap<String, GroupParts<I>>,
        global_tasks: &'b HashMap<String, TaskParts<I>>,
        filter: &mut UserFilter,
    ) -> dev_vault::Result<Plan<'a, I>> {
        info!("cast group {}", self.dummy.id);
        let mut cite_task_nodes = Vec::new();
        let mut nodes = Vec::with_capacity(self.cites.len());
        for cite in self.cites.iter() {
            if let Some(tp) = global_tasks.get(&cite.id) {
                let (mut task, state) = tp.to_nodes();
                task.target <<= cite.target.cast();
                info!("Add cite task {} [{}]", cite.id, task.target);
                nodes.push((task, state))
            } else if let Some(plan) = groups.get(&cite.id) {
                let plan = plan.cast(groups, global_tasks, filter)?;
                cite_task_nodes.extend(plan.tasks.into_iter().map(|mut task| {
                    task.target <<= cite.target.cast();
                    info!("Add cite task {} [{}]", task.id, task.target);
                    task
                }));
                nodes.push((plan.final_, plan.start));
            } else {
                panic!("plan {} cite {} not found", self.dummy.id, cite.id);
            }
        }
        let nodes = self
            .tasks
            .iter()
            .map(|tp| tp.to_nodes())
            .inspect(|(task, _)| {
                info!("Add task {} [{}]", task.id, task.target);
            })
            .chain(nodes)
            .inspect(|(task, _)| {
                task.target.filter(filter);
            });
        let topo = self
            .tasks
            .iter()
            .map(|tp| (&tp.id, &tp.next[..]))
            .chain(self.cites.iter().map(|cite| (&cite.id, &cite.next[..])));
        let mut plan = Plan::new(
            repeat_with(|| self.dummy.to_nodes()).take(2).chain(nodes),
            topo,
        );
        plan.tasks.extend(cite_task_nodes);
        Ok(plan)
    }
}
