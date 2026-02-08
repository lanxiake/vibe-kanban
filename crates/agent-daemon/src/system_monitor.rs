//! 系统监控模块
//!
//! 采集系统资源使用情况（CPU、内存、磁盘等）

use sysinfo::System;

use crate::protocol::{SystemInfo, SystemStats};

/// 系统监控器
pub struct SystemMonitor {
    system: System,
}

impl SystemMonitor {
    /// 创建新的系统监控器
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }

    /// 获取系统信息（静态信息）
    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            os: System::name().unwrap_or_else(|| "unknown".to_string()),
            arch: std::env::consts::ARCH.to_string(),
            cpu_cores: self.system.cpus().len() as i32,
            total_memory_gb: self.system.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
            disk_total_gb: self.get_total_disk_space(),
            hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
        }
    }

    /// 获取系统统计信息（动态信息）
    pub fn get_system_stats(&mut self) -> SystemStats {
        // 刷新系统信息
        self.system.refresh_all();

        // 计算 CPU 使用率
        let cpu_usage = self.system.global_cpu_usage() as f64;

        // 计算内存使用率
        let total_memory = self.system.total_memory() as f64;
        let used_memory = self.system.used_memory() as f64;
        let available_memory = self.system.available_memory() as f64;
        let memory_usage = if total_memory > 0.0 {
            (used_memory / total_memory) * 100.0
        } else {
            0.0
        };

        // 计算磁盘使用率
        let (disk_usage, disk_available) = self.get_disk_stats();

        // 获取负载平均值
        let load_avg = System::load_average();

        SystemStats {
            cpu_usage_percent: cpu_usage,
            memory_usage_percent: memory_usage,
            memory_available_gb: available_memory / 1024.0 / 1024.0 / 1024.0,
            disk_usage_percent: disk_usage,
            disk_available_gb: disk_available,
            load_average: [load_avg.one, load_avg.five, load_avg.fifteen],
        }
    }

    /// 获取总磁盘空间（GB）
    fn get_total_disk_space(&self) -> f64 {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let total: u64 = disks.iter().map(|d| d.total_space()).sum();
        total as f64 / 1024.0 / 1024.0 / 1024.0
    }

    /// 获取磁盘统计信息
    fn get_disk_stats(&self) -> (f64, f64) {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let total: u64 = disks.iter().map(|d| d.total_space()).sum();
        let available: u64 = disks.iter().map(|d| d.available_space()).sum();

        let usage = if total > 0 {
            ((total - available) as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let available_gb = available as f64 / 1024.0 / 1024.0 / 1024.0;

        (usage, available_gb)
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}
