///
/// 进程管理调度


use alloc::sync::Arc;
use crate::task::TaskControlBlock;





 /**
  * 进程管理器
  */
struct Processer{
    task:Arc<TaskControlBlock>,
    current_task:usize
}