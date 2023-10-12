use std::thread::scope;
use std::thread::available_parallelism;

type Task = fn();

#[derive(Clone)]
pub(crate) struct ThreadDispatcher {
    tasks: Vec<Task>,
}

impl ThreadDispatcher {
    pub(crate) fn new () -> Self {
        Self {
            tasks: Vec::new(),
        }
    }

    pub(crate) fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub(crate) fn remove_task(&mut self, task: Task) {
        self.tasks.retain(|t| t != &task);
    }

    pub(crate) fn purge_tasks(&mut self) {
        self.tasks.clear();
    }

    pub(super) fn get_tasks(&self) -> &Vec<Task> {
        &self.tasks
    }

    pub(crate) fn get_task_count(&self) -> usize {
        self.tasks.len()
    }

    pub(crate) fn run(&mut self) {
        // todo - do this lol
        panic!("uhh don't run this yet!")
    }
}