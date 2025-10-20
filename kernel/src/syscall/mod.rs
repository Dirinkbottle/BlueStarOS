mod syscall;
pub const GET_TIME:usize=0;//获取系统时间
pub const SYS_WRITE:usize=1;//stdin write系统调用
pub const SYS_READ:usize=2;//stdin read系统调用



///id: 系统调用号
///args:接受1个usize参数
pub fn syscall_handler(id:usize,arg:[usize;3]){//目前只支持3个参数
    match id {
        GET_TIME=>{

        }
        SYS_WRITE=>{
        }
        SYS_READ=>{
            
        }
        _=>{panic!("Unknow Syscall type!");}
    }
}