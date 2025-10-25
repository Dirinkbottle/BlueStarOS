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
    /// 创建新任务
    /// app_id: 应用程序ID（从0开始，用于加载不同的ELF文件）
    /// kernel_stack_id: 内核栈ID（从1开始，用于分配不同的内核栈空间）
    fn new(app_id: usize, kernel_stack_id: usize) -> Self {
        debug!("Creating task for app_id: {}, kernel_stack_id: {}", app_id, kernel_stack_id);
        
        let elf_data = file_loader(app_id);
        let (mut memset, elf_entry, user_sp, kernel_sp) = MapSet::from_elf(kernel_stack_id, elf_data);
        let task_cx = TaskContext::return_trap_new(kernel_sp);
        let kernel_satp = KERNEL_SPACE.lock().table.satp_token();
        let trap_cx_ppn = memset.table
            .translate_byvpn(VirAddr(TRAP_CONTEXT_ADDR).strict_into_virnum())
            .expect("trap ppn translate failed");
        
        let task_control_block = TaskControlBlock {
            memory_set: memset,
            task_statut: TaskStatus::Ready,
            task_context: task_cx,
            trap_context_ppn: trap_cx_ppn.0,
            pass: 0,
            stride: BIG_INT / TASK_TICKET,
            ticket: TASK_TICKET
        };
        
        // 初始化 TrapContext
        let trap_cx_point: *mut TrapContext = (trap_cx_ppn.0 * PAGE_SIZE) as *mut TrapContext;
        unsafe {
            *trap_cx_point = TrapContext::init_app_trap_context(
                elf_entry,
                kernel_satp,
                kernel_trap_handler as usize,
                kernel_sp,
                user_sp.0
            );
        }
        
        debug!("Task created successfully: entry={:#x}, user_sp={:#x}", elf_entry, user_sp.0);
        task_control_block
    }
}


impl TaskManager {//全局唯一
    ///添加任务队列或者归队
    pub fn add_task(&mut self,task:TaskControlBlock){
        self.task_que_inner.lock().task_queen.push_back(task);
    }
    ///从队列移除任务,应该由aplication的exit系统调用来执行
    pub fn remove_task(&mut self,app_index:usize){

    }
    ///根据Stride挑选下个要运行的READY任务,挂起当前任务,把current设置为下个任务的index,然后运行下一个任务
    pub fn suspend_and_run_task(&self){
        let mut inner  =self.task_que_inner.lock();
        let current=inner.current;
        //标记当前任务为BLOCK
        inner.task_queen[current].task_statut=TaskStatus::Blocking;

        let task_index=match inner.task_queen.
        iter().
        enumerate().
        filter(|(_,block)|{if let TaskStatus::Ready=block.task_statut {true}else {
            false
        }}).
        min_by_key(|(_,block)|{
            block.pass
        }){
            Some((index,_))=>{
                index//返回任务的下标索引
            }
            None=>{
                panic!("No task can run!");
            }
        };
        //标记这个任务为run
        let swaped_task_cx=&inner.task_queen[current].task_context as *const TaskContext;
        let task=&mut inner.task_queen[task_index];
        task.task_statut=TaskStatus::Runing;
        let need_swap_in = &mut task.task_context as *mut TaskContext;
        debug!("current:{} Next task:{}",inner.current,task_index);
        inner.current=task_index;//更新任务指针
        drop(inner);//drop inner
        unsafe {
         __switch(swaped_task_cx , need_swap_in);
        }
        panic!("unreachable!!");

        //设置下一个任务的运行状态
    }


    ///运行第一个任务
    pub fn run_first_task(&self) -> ! {
      let mut inner=self.task_que_inner.lock();//记得drop
      let curren_task_index=inner.current;
      let task = &mut inner.task_queen[curren_task_index];
      let task_cx_ptr = &mut task.task_context as *mut TaskContext;
      let kernel_task_cx=TaskContext::zero_init();
      //标记为running
      task.task_statut=TaskStatus::Runing;
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
        drop(inner);
        stap
    }

    ///获取当前任务的陷阱上下文可变引用
    pub fn get_current_trapcx(&self)->&mut TrapContext{
        let inner =self.task_que_inner.lock();
        let curren_task_index=inner.current;
        let task_trap_ppn = inner.task_queen[curren_task_index].trap_context_ppn;
        let origin_phyaddr =( task_trap_ppn*PAGE_SIZE) as *mut TrapContext;
        let trap_context =unsafe {
            &mut *origin_phyaddr
        };
        drop(inner);
        trap_context
    }

}


impl TaskContext {
    ///ra设置为trap refume地址，sp为用户栈指针，callee_register初始化0
    pub fn trapnew_init(sp:usize)->Self{
       TaskContext { ra: __kernel_refume as usize, sp: sp, calleed_register: [0;12] }
    }
}

/// 全局任务管理器，加载所有应用程序
lazy_static! {
    pub static ref TASK_MANAER: TaskManager = unsafe {
        debug!("Initializing TASK_MANAGER...");
        let app_count = crate::task::get_app_count();
        debug!("Found {} applications to load", app_count);
        
        let mut task_deque = VecDeque::new();
        
        // 加载所有应用程序
        for app_id in 0..app_count {
            debug!("Loading application {}...", app_id);
            // app_id 从 0 开始，kernel_stack_id 从 1 开始
            let mut task = TaskControlBlock::new(app_id, app_id + 1);
            //task.task_statut=TaskStatus::Ready; 在new已经设置为ready
            task_deque.push_back(task);
            debug!("Application {} loaded successfully", app_id);
        }
        
        debug!("All {} applications loaded into task queue", app_count);
        
        TaskManager {
            task_que_inner: UPSafeCell::new(TaskManagerInner {
                task_queen: task_deque,
                current: 0  // 初始化为第一个任务
            })
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