use crate::{
    op::{tests::TestContext, WrapContext},
    task::TaskStatus,
};

use std::{collections::HashMap, iter::repeat_with, sync::Arc};
use tracing_test::traced_test;

use super::{AlwayDoNothing, AlwayFailed, AlwaySuccess, Plan, StateNode, TaskNode};

enum TestTask {
    Success,
    DoNothing,
    Failure,
}
async fn test_plan_op<'a, 'b: 'a>(tasks: &[(&str, &'a [&'b str], TestTask, TaskStatus)]) {
    let tasks = tasks
        .iter()
        .map(|(id, next, task, status)| {
            let next = next.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            (
                id.to_string(),
                next.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                match task {
                    TestTask::Success => AlwaySuccess.into(),
                    TestTask::DoNothing => AlwayDoNothing.into(),
                    TestTask::Failure => AlwayFailed.into(),
                },
                status,
            )
        })
        .collect::<Vec<_>>();
    let topo = tasks
        .iter()
        .map(|(id, next, ..)| (id, &next[..]))
        .collect::<Vec<_>>();
    let to_nodes = |id, task| {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = TaskNode::new(id, task, rx);
        let state = Arc::new(StateNode::new(tx));
        (task, state)
    };
    let id = String::from("test");
    let dummy_task = AlwaySuccess.into();
    let nodes = tasks.iter().map(|(id, _, task, _)| to_nodes(id, task));
    let context = Arc::new(TestContext::default().wrap());
    let plan = Plan::<TestContext>::new(
        repeat_with(|| to_nodes(&id, &dummy_task))
            .take(2)
            .chain(nodes),
        topo,
    );
    let res = plan
        .run(context)
        .await
        .into_iter()
        .collect::<HashMap<_, _>>();

    for (id, .., status) in tasks.iter() {
        assert_eq!(res[&id], **status);
    }
}
#[tokio::test]
#[traced_test]
async fn test_plan_op_case1() {
    test_plan_op(&[
        ("1", &["2"], TestTask::Success, TaskStatus::Success),
        ("2", &["3"], TestTask::Success, TaskStatus::Success),
        ("3", &[], TestTask::Success, TaskStatus::Success),
    ])
    .await;
}
#[tokio::test]
#[traced_test]
async fn test_plan_op_case2() {
    test_plan_op(&[
        ("1", &["2"], TestTask::Success, TaskStatus::Success),
        ("2", &["3"], TestTask::Failure, TaskStatus::Failed),
        ("3", &[], TestTask::Success, TaskStatus::Failed),
    ])
    .await;
}
#[tokio::test]
#[traced_test]
async fn test_plan_op_case3() {
    test_plan_op(&[
        ("1", &["2"], TestTask::Success, TaskStatus::Success),
        ("2", &["3"], TestTask::DoNothing, TaskStatus::DoNothing),
        ("3", &[], TestTask::Success, TaskStatus::DoNothing),
    ])
    .await;
}
#[tokio::test]
#[traced_test]
async fn test_plan_op_case4() {
    test_plan_op(&[
        ("1", &["3"], TestTask::Success, TaskStatus::Success),
        ("2", &["3"], TestTask::Success, TaskStatus::Success),
        ("3", &[], TestTask::Success, TaskStatus::Success),
    ])
    .await;
}
#[tokio::test]
#[traced_test]
async fn test_plan_op_case5() {
    test_plan_op(&[
        ("1", &["3"], TestTask::Success, TaskStatus::Success),
        ("2", &["3"], TestTask::DoNothing, TaskStatus::DoNothing),
        ("3", &[], TestTask::Success, TaskStatus::Success),
    ])
    .await;
}
#[tokio::test]
#[traced_test]
async fn test_plan_op_case6() {
    test_plan_op(&[
        ("1", &["3"], TestTask::DoNothing, TaskStatus::DoNothing),
        ("2", &["3"], TestTask::DoNothing, TaskStatus::DoNothing),
        ("3", &[], TestTask::Success, TaskStatus::DoNothing),
    ])
    .await;
}
