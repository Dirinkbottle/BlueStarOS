use core::arch::global_asm;
use core::panicking::panic;

use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use log::error;
use log::trace;
use riscv::register::sstatus;
use riscv::register::sstatus::SPP;
use crate::__kernel_refume;
use crate::config::*;
use crate::memory::*;
use crate::sbi::shutdown;
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


///进程id 需要实现回收 rail自动分配
pub struct  ProcessId(usize);

///进程id分配器 需要实现分配 [start,end)
pub struct ProcessIdAlloctor{
    current:usize,//当前的pid
    end:usize,//最高限制的pid，可选
    id_pool:Vec<ProcessId>
}

pub struct TaskControlBlock{
        pub pid:ProcessId,//进程id
        pub memory_set:MapSet,//程序地址空间
        task_statut:TaskStatus,//程序运行状态
        task_context:TaskContext,//任务上下文
        trap_context_ppn:usize,//陷阱上下文物理帧
        pass:usize,//行程
        stride:usize,//步长
        ticket:usize,//权重
}

//暂留，后期再重构
pub struct TaskControlBlockInner{

}



pub struct TaskManagerInner{
    pub task_queen:VecDeque<TaskControlBlock>,//任务队列
    pub current:usize//当前任务
}

///任务管理器
pub struct TaskManager{//单核环境目前无竞争
    ///注意释放时机
   pub task_que_inner:UPSafeCell<TaskManagerInner>,//内部可变性 
}

impl  ProcessIdAlloctor{
    ///初始化进程id分配器 start:起始分配pid end:限制最大的pid
    pub fn initial_processid_alloctor(start:usize,end:usize)->Self{
        let id_pool :Vec<ProcessId>= Vec::new();
            ProcessIdAlloctor { current: start, end ,id_pool:id_pool}
    }

    ///分配进程id
    pub fn alloc_id(&mut self)->Option<ProcessId>{
        //首先检查pool是否有可用process
        if !self.id_pool.is_empty(){
          return self.id_pool.pop();
        }
        //检查边界 ，先把currentid+1，然后返回
        if self.current < self.end{
            self.current+=1;
           return Some(ProcessId(self.current-1));
        }

        None
    }
}


impl Drop for ProcessId {
    fn drop(&mut self) {
        ///进程id自动回收 rail思想 需要先初始化全局processidalloctor
        ProcessId_ALLOCTOR.lock().id_pool.push(ProcessId(self.0));//实际只需要保存id号
        trace!("Process Id :{} recycled!",self.0)
    }
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
            pid:ProcessId_ALLOCTOR.lock().alloc_id().expect("No Process ID Can use"),
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
    ///从队列移除当前任务,应该由aplication的exit系统调用来执行 之后必须执行下一个任务 bug修复：应该同时移动指针到任意一个ready的任务
    pub fn remove_current_task(&self){
        let mut inner=self.task_que_inner.lock();
        
        // 先保存要删除的任务索引
        let task_to_remove = inner.current;
        debug!("Removing task at index: {}, queue length before removal: {}", task_to_remove, inner.task_queen.len());
        
        // 删除任务
        inner.task_queen.remove(task_to_remove).expect("Remove Task Control Block Failed!");
        
        // 删除后更新current指针
        // VecDeque.remove(i) 会删除索引i的元素，后面的元素索引都会减1
        // 删除后，如果还有任务，我们需要将current设置为一个有效的任务索引
        if !inner.task_queen.is_empty() {
            // 如果删除的是最后一个任务（task_to_remove == 原队列长度-1）
            // 则删除后 task_to_remove >= 新队列长度，需要回绕到开头
            if task_to_remove >= inner.task_queen.len() {
                inner.current = 0;
            } else {
                // 否则，保持current在原位置
                // 此时current指向的是原来task_to_remove+1位置的任务
                inner.current = task_to_remove;
            }
            debug!("After removal: current set to {}, queue length: {}", inner.current, inner.task_queen.len());
        }
        
        drop(inner);
        if self.task_queen_is_empty() {
            panic!("Remove Last Task");
        }
    }
    ///根据Stride挑选下个要运行的READY任务,挂起当前任务,把current设置为下个任务的index,然后运行下一个任务 Stride算法：增加运行任务的步长
    pub fn suspend_and_run_task(&self){ //首先应该检查任务是否为空

        //任务列表是否为空?
        if self.task_queen_is_empty(){
                panic!("Task Queen is empty!");
        }


        let mut inner  =self.task_que_inner.lock();
        let current=inner.current;
        //标记当前任务为BLOCK
        inner.task_queen[current].task_statut=TaskStatus::Ready;

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
                error!("No task can select");
                shutdown();
                
            }
        };
        
        debug!("current:{} Next task:{}",inner.current,task_index);
        
        //如果切换到同一个任务，直接返回 _switch耗费上下文资源
        //这可以防止在持有用户态锁时发生任务切换导致的死锁问题（全局锁）
        if current == task_index {
            // 重新标记为运行状态，增加步长
            inner.task_queen[task_index].task_statut = TaskStatus::Runing;
            inner.task_queen[task_index].pass += inner.task_queen[task_index].stride;
            drop(inner);
            debug!("Same task, skip __switch");
            return; // 直接返回，不需要切换
        }
        
        //标记这个任务为run
        let swaped_task_cx=&inner.task_queen[current].task_context as *const TaskContext;
        let task=&mut inner.task_queen[task_index];
        task.task_statut=TaskStatus::Runing;
        //增加步长
        task.pass+=task.stride;
        let need_swap_in = &mut task.task_context as *mut TaskContext;
        inner.current=task_index;//更新任务指针
        drop(inner);//drop inner
        unsafe {
         __switch(swaped_task_cx , need_swap_in);
        }

        //任务从这里返回
    }

    pub fn task_queen_is_empty(&self)->bool{
        let inner=self.task_que_inner.lock();
        let result= inner.task_queen.is_empty();
        drop(inner);
        debug!("task queen empty?:{}",result);
        result
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
      //增加步长
      task.pass+=task.stride;
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


    ///kail当前任务，内核有权调用 调用栈顶必须为TrapHandler! 调用它的地方考虑是否直接return
    pub fn kail_current_task_and_run_next(&self){
        self.remove_current_task();//删除对应任务块
        self.suspend_and_run_task();//调度下一个stride最小的任务
        error!("Task Kailed!");
    }



}


impl TaskContext {
    ///ra设置为trap refume地址，sp为用户栈指针，callee_register初始化0
    pub fn trapnew_init(sp:usize)->Self{
       TaskContext { ra: __kernel_refume as usize, sp: sp, calleed_register: [0;12] }
    }
}


///全局进程id分配器
lazy_static!{
    pub static ref ProcessId_ALLOCTOR:UPSafeCell<ProcessIdAlloctor>=UPSafeCell::new(ProcessIdAlloctor::initial_processid_alloctor(0, 10_000_000));
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