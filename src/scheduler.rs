pub const MAX_TASKS: usize = 64;
pub const STACK_SIZE: usize = 64 * 1024; // 64 KiB

pub const NICE_0_WEIGHT: u64 = 1024;


// Totally didnt steal ts from linux :P
pub static PRIO_WEIGHTS: [u64; 40] = [
    88761, 71755, 56483, 46273, 36291,
    29154, 23254, 18705, 14949, 11916,
     9548,  7620,  6100,  4904,  3906,
     3121,  2501,  1991,  1586,  1277,
     1024,   820,   655,   526,   423,
      335,   272,   215,   172,   137,
      110,    87,    70,    56,    45,
       36,    29,    23,    18,    15,
];

#[derive(Clone, Copy, PartialEq)]
pub enum TaskState {
    Running,
    Runnable,
    Blocked,
    Dead,
}

#[repr(C)]
pub struct Context {
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub rbx: u64, pub rbp: u64,
    pub rsp: u64,
}

pub struct Task {
    pub ctx:        Context,
    pub stack:      [u8; STACK_SIZE],
    pub state:      TaskState,
    pub id:         u32,
    pub nice:       i8,
    pub vruntime:   u64,
    pub weight:     u64,
}

impl Task {
    pub const fn new(id: u32) -> Self {
        Task {
            ctx:        Context { r15:0, r14:0, r13:0, r12:0, rbx:0, rbp:0, rsp:0},
            stack:      [0u8; STACK_SIZE],
            state:      TaskState::Runnable,
            id,
            nice:       0,
            vruntime:   0,
            weight:     NICE_0_WEIGHT,
        }
    }
}

core::arch::global_asm!(r#"
.global switch_context
switch_context:
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    mov  qword ptr [rdi], rsp
    mov  rsp, rsi
    pop  r15
    pop  r14
    pop  r13
    pop  r12
    pop  rbx
    pop  rbp
    ret
"#);
 
extern "C" {
    fn switch_context(old_rsp: *mut u64, new_rsp: u64);
}
 
static NEXT_ID: AtomicU32 = AtomicU32::new(1);
 
fn alloc_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
 
pub struct Scheduler {
    tasks:        [Task; MAX_TASKS],
    current:      usize,
    ticks:        u64,
    hz:           u64,
    ns_per_tick:  u64,
    now_ns:       u64,
    min_vruntime: u64,
    num_tasks:    usize,
}
 
unsafe impl Send for Scheduler {}
 
impl Scheduler {
    const fn new() -> Self {
        Scheduler {
            tasks:        [const { Task::idle() }; MAX_TASKS],
            current:      0,
            ticks:        0,
            hz:           100,
            ns_per_tick:  10_000_000,
            now_ns:       0,
            min_vruntime: 0,
            num_tasks:    0,
        }
    }
 
    pub fn init(&mut self, hz: u64) {
        self.hz          = hz;
        self.ns_per_tick = 1_000_000_000 / hz;
 
        let t      = &mut self.tasks[0];
        t.id       = alloc_id();
        t.state    = TaskState::Running;
        t.nice     = 0;
        t.weight   = NICE_0_WEIGHT;
        t.vruntime = 0;
 
        self.current   = 0;
        self.num_tasks = 1;
    }
 
    pub fn spawn(&mut self, entry: fn() -> !, nice: i8) -> Option<u32> {
        let slot = self.free_slot()?;
        let id   = alloc_id();
 
        let t      = &mut self.tasks[slot];
        t.id       = id;
        t.state    = TaskState::Runnable;
        t.nice     = nice;
        t.weight   = nice_to_weight(nice);
        t.vruntime = self.min_vruntime;
 
        let stack_end = t.stack.as_mut_ptr() as usize + STACK_SIZE;
        let aligned   = stack_end & !0xF;
        let frame_top = aligned - 8;
        let rsp       = aligned - 56;
 
        unsafe {
            (frame_top as *mut u64).write(entry as u64);
            let regs = rsp as *mut u64;
            for i in 0..6usize { regs.add(i).write(0); }
        }
 
        t.rsp = rsp as u64;
        self.num_tasks += 1;
        Some(id)
    }
 
    pub fn tick(&mut self) {
        self.ticks  += 1;
        self.now_ns += self.ns_per_tick;
 
        if let Some(cur) = self.running_task_mut() {
            let delta = self.ns_per_tick * NICE_0_WEIGHT / cur.weight;
            cur.vruntime = cur.vruntime.saturating_add(delta);
        }
 
        self.update_min_vruntime();
 
        let cur_vrt = self.tasks[self.current].vruntime;
 
        if let Some((next_idx, next_vrt)) = self.find_best_runnable() {
            if next_idx != self.current && cur_vrt > next_vrt + MIN_GRANULARITY_NS {
                self.do_switch(next_idx);
            }
        }
    }
 
    pub fn block_current(&mut self) {
        if let Some(t) = self.running_task_mut() {
            t.state = TaskState::Blocked;
        }
        if let Some((next, _)) = self.find_best_runnable() {
            self.do_switch(next);
        }
    }
 
    pub fn wake(&mut self, id: u32) {
        let min_vrt = self.min_vruntime;
        if let Some(t) = self.task_by_id_mut(id) {
            if t.state == TaskState::Blocked || t.state == TaskState::Dead {
                t.vruntime = t.vruntime.max(min_vrt);
                t.state    = TaskState::Runnable;
            }
        }
    }
 
    pub fn yield_current(&mut self) {
        let boost   = self.ns_per_tick * NICE_0_WEIGHT / self.tasks[self.current].weight;
        let new_vrt = self.min_vruntime + boost;
        self.tasks[self.current].vruntime = self.tasks[self.current].vruntime.max(new_vrt);
 
        if let Some((next, _)) = self.find_best_runnable() {
            if next != self.current {
                self.do_switch(next);
            }
        }
    }
 
    pub fn renice(&mut self, id: u32, nice: i8) {
        if let Some(t) = self.task_by_id_mut(id) {
            t.nice   = nice;
            t.weight = nice_to_weight(nice);
        }
    }
 
    pub fn exit_current(&mut self) -> ! {
        self.tasks[self.current].state = TaskState::Dead;
        self.num_tasks = self.num_tasks.saturating_sub(1);
 
        match self.find_best_runnable() {
            Some((next, _)) => {
                self.do_switch(next);
                loop { x86_64::instructions::hlt(); }
            }
            None => loop { x86_64::instructions::hlt(); }
        }
    }
 
    pub fn current_id(&self) -> u32 {
        self.tasks[self.current].id
    }
 
    pub fn num_tasks(&self) -> usize {
        self.num_tasks
    }
 
    pub fn task_list(&self, out: &mut [(u32, u64, i8, TaskState)]) -> usize {
        let mut n = 0;
        for t in self.tasks.iter() {
            if n >= out.len() { break; }
            if t.state != TaskState::Dead {
                out[n] = (t.id, t.vruntime, t.nice, t.state);
                n += 1;
            }
        }
        n
    }
 
    fn free_slot(&self) -> Option<usize> {
        self.tasks.iter().position(|t| t.state == TaskState::Dead)
    }
 
    fn running_task_mut(&mut self) -> Option<&mut Task> {
        let t = &mut self.tasks[self.current];
        if t.state == TaskState::Running { Some(t) } else { None }
    }
 
    fn task_by_id_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }
 
    fn find_best_runnable(&self) -> Option<(usize, u64)> {
        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.state == TaskState::Runnable)
            .min_by_key(|(_, t)| t.vruntime)
            .map(|(i, t)| (i, t.vruntime))
    }
 
    fn update_min_vruntime(&mut self) {
        let new_min = self.tasks
            .iter()
            .filter(|t| t.state == TaskState::Running || t.state == TaskState::Runnable)
            .map(|t| t.vruntime)
            .min()
            .unwrap_or(self.min_vruntime);
 
        if new_min > self.min_vruntime {
            self.min_vruntime = new_min;
        }
    }
 
    fn do_switch(&mut self, next: usize) {
        let prev = self.current;
        if prev == next { return; }
 
        if self.tasks[prev].state == TaskState::Running {
            self.tasks[prev].state = TaskState::Runnable;
        }
        self.tasks[next].state             = TaskState::Running;
        self.tasks[next].last_scheduled_ns = self.now_ns;
        self.current                       = next;
 
        let old_rsp_ptr: *mut u64 =
            unsafe { &mut (*(&mut self.tasks[prev] as *mut Task)).rsp };
        let new_rsp: u64 = self.tasks[next].rsp;
 
        unsafe { switch_context(old_rsp_ptr, new_rsp); }
    }
}
 
pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
 
pub fn init(hz: u64) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        SCHEDULER.lock().init(hz);
    });
}
 
pub fn spawn(entry: fn() -> !, nice: i8) -> Option<u32> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        SCHEDULER.lock().spawn(entry, nice)
    })
}
 
pub fn block_current() {
    SCHEDULER.lock().block_current();
}
 
pub fn wake(id: u32) {
    SCHEDULER.lock().wake(id);
}
 
pub fn yield_now() {
    SCHEDULER.lock().yield_current();
}
 
pub fn on_timer_tick() {
    if let Some(mut sched) = SCHEDULER.try_lock() {
        sched.tick();
    }
}
