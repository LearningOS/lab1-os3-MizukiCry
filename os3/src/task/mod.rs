mod context;
mod switch;
mod task;

use crate::{
    config::{MAX_APP_NUM, MAX_SYSCALL_NUM, CLOCK_FREQ},
    loader::{get_num_app, init_app_cx},
    sync::UPSafeCell, timer::get_time_ms,
};
use alloc::{vec::Vec, boxed::Box};
use lazy_static::lazy_static;
pub use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};
pub use context::TaskContext;

struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks: Vec<TaskControlBlock> = Vec::with_capacity(MAX_APP_NUM);
        for _ in 0..MAX_APP_NUM {
            tasks.push(TaskControlBlock {
                task_cx: TaskContext::zero_init(),
                task_status: TaskStatus::UnInit,
                syscall_times: Box::new([0; MAX_SYSCALL_NUM]),
                start_time: None,
            });
        }
        for (i, t) in tasks.iter_mut().enumerate().take(num_app) {
            t.task_cx = TaskContext::goto_restore(init_app_cx(i));
            t.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: UPSafeCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
            }),
        }
    };
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        task0.start_time = Some(get_time_ms());
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_curren_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            if inner.tasks[next].start_time.is_none() {
                inner.tasks[next].start_time = Some(get_time_ms());
            }
            drop(inner);
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        }
        else {
            panic!("All applications completed!");
        }
    }

    fn count_syscall(&self, syscall_id: usize) {
        if syscall_id < MAX_SYSCALL_NUM {
            let mut inner = TASK_MANAGER.inner.exclusive_access();
            let current_task = inner.current_task;
            inner.tasks[current_task].syscall_times[syscall_id] += 1;
        }
    }

    fn get_syscall_times(&self) -> [u32; MAX_SYSCALL_NUM] {
        let inner = TASK_MANAGER.inner.exclusive_access();
        *inner.tasks[inner.current_task].syscall_times
    }

    fn get_current_run_time(&self) -> usize {
        let inner = TASK_MANAGER.inner.exclusive_access();
        get_time_ms() - inner.tasks[inner.current_task].start_time.unwrap()
    }

    fn get_current_task_status(&self) -> TaskStatus {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].task_status
    }
    // My Job: Try to implement my function to update or get task info!
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_curren_exited();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

pub fn count_syscall(syscall_id: usize) {
    TASK_MANAGER.count_syscall(syscall_id);
}

pub fn get_syscall_times() -> [u32; MAX_SYSCALL_NUM] {
    TASK_MANAGER.get_syscall_times()
}

pub fn get_current_run_time() -> usize {
    TASK_MANAGER.get_current_run_time()
}

pub fn get_current_task_status() -> TaskStatus {
    TASK_MANAGER.get_current_task_status()
}

// My Job: Public functions implemented here provide interfaces
// I may use TASK_MANAGER member functions to handle requests.