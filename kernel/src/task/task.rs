use core::arch::global_asm;
use core::panicking::panic;

use alloc::collections::vec_deque::VecDeque;
use lazy_static::lazy_static;
use riscv::register::sstatus;
use riscv::register::sstatus::SPP;
use crate::__kernel_refume;
use crate::config::*;
use crate::memory::*;
use crate::task::file_loader;
use log::debug;
use crate::trap::{app_entry_point, kernel_trap_handler};
///任务上下文
use crate::{ sync::UPSafeCell, trap::TrapContext};
global_asm!(include_str!("_switch.S"));

#[repr(C)]
pub struct TaskContext{
     ra:usize, //offset 0
     sp:usize, //offser 8
     ///s0-s11 被调用者保存寄存器 switch保存
     calleed_register:[usize;12]//offset 16-..
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
    trap_context_ppn:usize,//陷阱上下文物理帧
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


impl TaskContext {
    /// 创建任务上下文，跳转到 app_entry_point
    /// 注意：kernel_sp 是内核栈指针，不是用户栈！
    /// app_entry_point 是内核函数，需要内核栈来执行
    fn return_trap_new(kernel_sp: usize) -> Self {
       TaskContext { ra: app_entry_point as usize, sp: kernel_sp, calleed_register: [0;12] }
    }
///零初始化
    pub fn zero_init()->Self{
        TaskContext { ra: 0, sp: 0, calleed_register: [0;12] }
    }
}

impl TaskControlBlock {
    ///创建第一个任务 appid用于创建内核栈的，目前为为1,承担trapcontext初始化任务
    fn new(app_id: usize)->Self{
        let elf_data=file_loader();
        let (mut memset,elf_entry,user_ap,kernel_sp) = MapSet::from_elf(app_id,elf_data);
        let task_cx = TaskContext::return_trap_new(kernel_sp);
        let kernel_satp=KERNEL_SPACE.lock().table.satp_token();
        let trap_cx_ppn = memset.table.translate_byvpn(VirAddr(TRAP_CONTEXT_ADDR).strict_into_virnum()).expect("trap ppn transalte failed");
        let task_control_block=TaskControlBlock{
            memory_set:memset,
            task_statut:TaskStatus::Ready,
            task_context:task_cx,
            trap_context_ppn:trap_cx_ppn.0,
            pass:0,
            stride:BIG_INT/TASK_TICKET,
            ticket:TASK_TICKET
        };
        let trap_cx_point:*mut TrapContext = (trap_cx_ppn.0 * PAGE_SIZE) as *mut TrapContext;
        // 设置trap地址
        unsafe {
            //traphandler应该传trapline高地址
            *trap_cx_point=TrapContext::init_app_trap_context(elf_entry, kernel_satp,kernel_trap_handler as usize , kernel_sp,user_ap.0)
        }
        task_control_block
    }
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

    #[no_mangle]
    pub fn run_first_task(&self) -> ! {
      let mut inner=self.task_que_inner.lock();//记得drop
      let curren_task_index=inner.current;
      let mut task = &mut inner.task_queen[curren_task_index];
      let task_cx_ptr = &mut task.task_context as *mut TaskContext;
      let kernel_task_cx=TaskContext::zero_init();
      drop(inner);//越早越好
      // 调用 __switch 切换到第一个任务
      // __switch 会：
      // 1. 保存 _unused 的上下文（虽然我们不会再用到）
      // 2. 恢复 next_task_cx_ptr 指向的上下文
      // 3. 跳转到 task.task_context.ra，即 app_entry_point
      unsafe {
        __switch(&kernel_task_cx as *const TaskContext, task_cx_ptr);
      }
      
      panic!("unreachable in run_first_task!");
    }

    ///获取当前任务的页表stap
    pub fn get_current_stap(&self)->usize{
        let mut inner= self.task_que_inner.lock();
        let current_task:usize=inner.current;
        let  task_memset = &mut inner.task_queen[current_task].memory_set;
        let stap = task_memset.get_table().satp_token();
        stap
    }

}


impl TaskContext {
    ///ra设置为trap refume地址，sp为用户栈指针，callee_register初始化0
    pub fn trapnew_init(sp:usize)->Self{
       TaskContext { ra: __kernel_refume as usize, sp: sp, calleed_register: [0;12] }
    }
}

///全局任务管理器 初始只有一个任务
lazy_static!{
     pub static ref TASK_MANAER:TaskManager=unsafe {
        debug!("Initializing TASK_MANAGER...");
        let mut task_deque=VecDeque::new();
        let task=TaskControlBlock::new(1);//初始化第一个任务block，app_id=1
        task_deque.push_back(task);
        debug!("First task added to queue");
        TaskManager{
            task_que_inner:UPSafeCell::new(
            TaskManagerInner{
                task_queen:task_deque,
                current:0//初始化为第一个任务
              }
            )
        }   
     };
}
///返回单个app的内核栈地址（在内核地址空间）
pub fn getapp_kernel_sapce()->usize{
    // 现在内核栈在内核空间，需要返回第0个任务的内核栈顶
    let app_id = 0;  // 第一个任务
    let kernel_stack_bottom = TRAP_BOTTOM_ADDR - (app_id + 1) * (KERNEL_STACK_SIZE + PAGE_SIZE);
    kernel_stack_bottom + KERNEL_STACK_SIZE  // 返回栈顶
}


pub fn run_first_task()->!{
    TASK_MANAER.run_first_task();
}