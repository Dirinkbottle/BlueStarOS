use alloc::collections::vec_deque::VecDeque;
use lazy_static::lazy_static;
use crate::__kernel_refume;
///任务上下文
use crate::{memory::MapSet, sync::UPSafeCell, trap::TrapContext};
pub struct TaskContext{
     ra:usize,
     sp:usize,
     ///s0-s11 被调用者保存寄存器 switch保存
     calleed_register:[usize;12]
}

enum TaskStatus {
    UnInit,
    Runing,
    Zombie,
    Blocking,
    Ready,
    Exit,
}

pub struct TaskControlBlock{
    memory_set:MapSet,//程序地址空间
    task_statut:TaskStatus,//程序运行状态
    task_context:TaskContext,//任务上下文
    trap_context:TrapContext,//陷阱上下文
    pass:usize,//行程
    stride:usize,//步长
    ticket:usize,//权重
}



struct TaskManagerInner{
    task_queen:VecDeque<TaskControlBlock>,//任务队列
    current:usize//当前任务
}

///任务管理器
pub struct TaskManager{//单核环境目前无竞争
    task_que_inner:UPSafeCell<TaskManagerInner>,//内部可变性
}




impl TaskManager {//全局唯一
    ///添加任务队列或者归队
    pub fn add_task(&mut self,task:TaskControlBlock){
        self.task_que_inner.lock().task_queen.push_back(task);
    }
    ///从队列移除任务
    pub fn remove_task(&mut self){

    }
    ///根据Stride挑选下个要运行的任务
    pub fn task_next_task(&mut self)->Option<TaskControlBlock>{
       match self.task_que_inner.lock().task_queen.iter().enumerate().min_by_key(|(index,block)|{block.pass}){
            Some((index,_))=>{
                Some(self.task_que_inner.lock().task_queen.swap_remove_back(index).expect("No task But matched!!!")) 
            }
            None=>{
                return None;
            }
       }
       

    }


    pub fn run_first_task(&mut self){

    }



}


impl TaskContext {
    ///ra设置为trap refume地址，sp为用户栈指针，callee_register初始化0
    pub fn trapnew_init(sp:usize)->Self{
       TaskContext { ra: __kernel_refume as usize, sp: sp, calleed_register: [0;12] }
    }
}

///全局任务管理器
lazy_static!{
     pub static ref TASK_MANAER:TaskManager=unsafe {
        TaskManager{
            task_que_inner:UPSafeCell::new(
            TaskManagerInner{
                task_queen:VecDeque::new(),
                current:0
            }
        )
    }
        
     };
}