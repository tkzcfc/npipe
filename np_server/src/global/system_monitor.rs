use once_cell::sync::Lazy;
use std::sync::RwLock;
use std::time::Duration;
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};

/// 后台采样间隔。CPU 占用依赖两次采样的差值，间隔需要大于 sysinfo 的最小窗口。
const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub(crate) struct SystemSnapshot {
    pub host_name: String,
    pub os_name: String,
    pub kernel_version: String,
    pub uptime_secs: u64,
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub total_memory: u64,
    pub used_memory: u64,
    pub memory_usage: f32,
}

static SNAPSHOT: Lazy<RwLock<SystemSnapshot>> = Lazy::new(|| RwLock::new(initial_snapshot()));

fn initial_snapshot() -> SystemSnapshot {
    SystemSnapshot {
        host_name: System::host_name().unwrap_or_default(),
        os_name: System::name().unwrap_or_default(),
        kernel_version: System::kernel_version().unwrap_or_default(),
        uptime_secs: System::uptime(),
        cpu_usage: 0.0,
        cpu_cores: 0,
        total_memory: 0,
        used_memory: 0,
        memory_usage: 0.0,
    }
}

pub(crate) fn snapshot() -> SystemSnapshot {
    SNAPSHOT
        .read()
        .unwrap_or_else(|err| err.into_inner())
        .clone()
}

/// 只刷新 CPU 和内存，不枚举进程。
pub(crate) fn start() {
    tokio::spawn(async move {
        let mut system = System::new();
        system.refresh_cpu_usage();
        tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;

        loop {
            system.refresh_cpu_usage();
            system.refresh_memory();
            publish(&system);
            tokio::time::sleep(SAMPLE_INTERVAL).await;
        }
    });
}

fn publish(system: &System) {
    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let memory_usage = if total_memory > 0 {
        used_memory as f32 * 100.0 / total_memory as f32
    } else {
        0.0
    };

    let mut guard = SNAPSHOT.write().unwrap_or_else(|err| err.into_inner());
    guard.uptime_secs = System::uptime();
    guard.cpu_usage = system.global_cpu_usage();
    guard.cpu_cores = system.cpus().len();
    guard.total_memory = total_memory;
    guard.used_memory = used_memory;
    guard.memory_usage = memory_usage;
}
