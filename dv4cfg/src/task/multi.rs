mod app;
pub use app::*;
mod auto;
pub use auto::*;
mod copy;
pub use copy::*;
mod exec;
pub use exec::*;

use super::into_boxed_task;

use dev::{BoxedTask, ContextImpl};
into_boxed_task!(
    AppTask,
    DryRunAppTask,
    AutoTask,
    DryRunAutoTask,
    CopyTask,
    DryRunCopyTask,
    ExecTask,
    DryRunExecTask
);

mod dev {
    pub use std::sync::Arc;

    pub use async_trait::async_trait;

    pub use super::super::core::*;
    pub use crate::{
        adapter::TaskParts,
        config::{TaskAttr, TaskComplete},
        error::Result,
        op::{Context, ContextImpl},
    };
    pub use serde::{Deserialize, Serialize};
    pub use snafu::whatever;
}

pub mod config {
    pub use super::app::AppTaskConfig;
    pub use super::auto::AutoTaskConfig;
    pub use super::copy::CopyTaskConfig;
    pub use super::exec::ExecTaskConfig;
}
