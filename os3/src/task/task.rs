use alloc::boxed::Box;

use crate::config::MAX_SYSCALL_NUM;

use super::TaskContext;

#[derive(Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub syscall_times: Box<[u32; MAX_SYSCALL_NUM]>,
    pub start_time: Option<usize>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}