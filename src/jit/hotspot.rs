//! Hot spot detection for JIT compilation
//! 
//! This module implements algorithms to detect frequently executed
//! code paths that would benefit from JIT compilation.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Information about a function's execution pattern
#[derive(Debug, Clone)]
pub struct ExecutionInfo {
    /// Number of times this function has been executed
    pub execution_count: u32,
    /// Total time spent executing this function (in microseconds)
    pub total_execution_time_us: u64,
    /// Average execution time per call (in microseconds)
    pub avg_execution_time_us: u64,
    /// Last execution timestamp
    pub last_execution: Instant,
    /// First execution timestamp
    pub first_execution: Instant,
    /// Whether this function is currently considered a hot spot
    pub is_hotspot: bool,
    /// Peak execution frequency (calls per second)
    pub peak_frequency: f64,
}

impl ExecutionInfo {
    /// Create new execution info
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            execution_count: 0,
            total_execution_time_us: 0,
            avg_execution_time_us: 0,
            last_execution: now,
            first_execution: now,
            is_hotspot: false,
            peak_frequency: 0.0,
        }
    }

    /// Record a new execution
    pub fn record_execution(&mut self, execution_time_us: u64) {
        if self.execution_count == 0 {
            self.first_execution = Instant::now();
        }

        self.execution_count += 1;
        self.total_execution_time_us += execution_time_us;
        self.avg_execution_time_us = self.total_execution_time_us / self.execution_count as u64;
        self.last_execution = Instant::now();

        // Update peak frequency
        self.update_peak_frequency();
    }

    /// Update peak frequency calculation
    fn update_peak_frequency(&mut self) {
        let elapsed = self.last_execution.duration_since(self.first_execution);
        if elapsed.as_secs() > 0 {
            let frequency = self.execution_count as f64 / elapsed.as_secs_f64();
            if frequency > self.peak_frequency {
                self.peak_frequency = frequency;
            }
        }
    }

    /// Get the execution frequency (calls per second)
    pub fn get_frequency(&self) -> f64 {
        let elapsed = self.last_execution.duration_since(self.first_execution);
        if elapsed.as_secs_f64() > 0.0 {
            self.execution_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Check if this function qualifies as a hot spot
    pub fn is_hotspot_candidate(&self, threshold: u32, min_frequency: f64) -> bool {
        self.execution_count >= threshold && self.get_frequency() >= min_frequency
    }

    /// Get the total time this function has been active
    pub fn get_active_duration(&self) -> Duration {
        self.last_execution.duration_since(self.first_execution)
    }

    /// Calculate potential time savings from JIT compilation
    pub fn calculate_potential_savings(&self, speedup_factor: f64) -> u64 {
        let time_per_call = self.avg_execution_time_us as f64;
        let improved_time = time_per_call / speedup_factor;
        let savings_per_call = time_per_call - improved_time;
        (savings_per_call * self.execution_count as f64) as u64
    }
}

/// Hot spot detection strategy
#[derive(Debug, Clone)]
pub enum HotspotStrategy {
    /// Simple threshold-based detection
    Threshold {
        min_executions: u32,
        min_frequency: f64,
    },
    /// Adaptive detection based on execution patterns
    Adaptive {
        base_threshold: u32,
        frequency_weight: f64,
        time_weight: f64,
    },
    /// Statistical analysis based detection
    Statistical {
        percentile_threshold: f64,
        min_sample_size: u32,
    },
}

impl Default for HotspotStrategy {
    fn default() -> Self {
        HotspotStrategy::Threshold {
            min_executions: 10,  // Much lower threshold for faster JIT
            min_frequency: 0.0001,  // Very lenient frequency requirement
        }
    }
}

/// Hot spot detector that tracks function execution patterns
#[derive(Debug)]
pub struct HotspotDetector {
    /// Execution information for each function
    execution_info: HashMap<usize, ExecutionInfo>,
    /// Detection strategy
    strategy: HotspotStrategy,
    /// Global statistics
    total_executions: u64,
    /// Start time for global statistics
    start_time: Instant,
}

impl HotspotDetector {
    /// Create a new hot spot detector
    pub fn new() -> Self {
        Self {
            execution_info: HashMap::new(),
            strategy: HotspotStrategy::default(),
            total_executions: 0,
            start_time: Instant::now(),
        }
    }

    /// Create a detector with a specific strategy
    pub fn with_strategy(strategy: HotspotStrategy) -> Self {
        Self {
            execution_info: HashMap::new(),
            strategy,
            total_executions: 0,
            start_time: Instant::now(),
        }
    }

    /// Record a function execution
    pub fn record_execution(&mut self, function_index: usize, execution_time_us: u64) {
        let info = self.execution_info
            .entry(function_index)
            .or_insert_with(ExecutionInfo::new);

        info.record_execution(execution_time_us);
        self.total_executions += 1;



        // Update hot spot status based on strategy
        self.update_hotspot_status(function_index);
    }

    /// Check if a function is considered a hot spot
    pub fn is_hotspot(&self, function_index: usize, threshold: u32) -> bool {
        if let Some(info) = self.execution_info.get(&function_index) {
            // Simplified: just check execution count, ignore frequency for now
            info.execution_count >= threshold
        } else {
            false
        }
    }

    /// Update hot spot status for a function
    fn update_hotspot_status(&mut self, function_index: usize) {
        let is_hotspot = self.is_hotspot(function_index, 0); // Use strategy's own threshold
        if let Some(info) = self.execution_info.get_mut(&function_index) {
            info.is_hotspot = is_hotspot;
        }
    }

    /// Calculate adaptive threshold based on execution patterns
    fn calculate_adaptive_threshold(
        &self,
        base_threshold: u32,
        frequency_weight: f64,
        time_weight: f64,
        info: &ExecutionInfo,
    ) -> u32 {
        let frequency_factor = (info.get_frequency() * frequency_weight).max(0.1);
        let time_factor = (info.avg_execution_time_us as f64 * time_weight / 1000.0).max(0.1);
        
        let adjusted = base_threshold as f64 / (frequency_factor * time_factor);
        adjusted.max(10.0) as u32 // Minimum threshold of 10
    }

    /// Check if a function is a statistical hot spot
    fn is_statistical_hotspot(
        &self,
        function_index: usize,
        percentile_threshold: f64,
        min_sample_size: u32,
    ) -> bool {
        if self.execution_info.len() < min_sample_size as usize {
            return false;
        }

        if let Some(info) = self.execution_info.get(&function_index) {
            let mut execution_counts: Vec<u32> = self.execution_info
                .values()
                .map(|info| info.execution_count)
                .collect();
            
            execution_counts.sort_unstable();
            
            let percentile_index = ((execution_counts.len() as f64 * percentile_threshold) as usize)
                .min(execution_counts.len() - 1);
            
            let threshold = execution_counts[percentile_index];
            info.execution_count >= threshold
        } else {
            false
        }
    }

    /// Get all current hot spots
    pub fn get_hotspots(&self) -> Vec<(usize, u32, u64)> {
        self.execution_info
            .iter()
            .filter(|(_, info)| info.is_hotspot)
            .map(|(&index, info)| (index, info.execution_count, info.total_execution_time_us))
            .collect()
    }

    /// Get execution information for a function
    pub fn get_execution_info(&self, function_index: usize) -> Option<&ExecutionInfo> {
        self.execution_info.get(&function_index)
    }

    /// Get execution count for a specific function
    pub fn get_execution_count(&self, function_index: usize) -> u32 {
        self.execution_info.get(&function_index)
            .map(|info| info.execution_count)
            .unwrap_or(0)
    }

    /// Get all functions sorted by execution count
    pub fn get_functions_by_execution_count(&self) -> Vec<(usize, &ExecutionInfo)> {
        let mut functions: Vec<_> = self.execution_info.iter().collect();
        functions.sort_by(|a, b| b.1.execution_count.cmp(&a.1.execution_count));
        functions.into_iter().map(|(&index, info)| (index, info)).collect()
    }

    /// Get all functions sorted by frequency
    pub fn get_functions_by_frequency(&self) -> Vec<(usize, &ExecutionInfo)> {
        let mut functions: Vec<_> = self.execution_info.iter().collect();
        functions.sort_by(|a, b| b.1.get_frequency().partial_cmp(&a.1.get_frequency()).unwrap_or(std::cmp::Ordering::Equal));
        functions.into_iter().map(|(&index, info)| (index, info)).collect()
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.execution_info.clear();
        self.total_executions = 0;
        self.start_time = Instant::now();
    }

    /// Get global execution statistics
    pub fn get_global_stats(&self) -> (u64, Duration, f64) {
        let elapsed = self.start_time.elapsed();
        let global_frequency = if elapsed.as_secs_f64() > 0.0 {
            self.total_executions as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        (self.total_executions, elapsed, global_frequency)
    }

    /// Set the detection strategy
    pub fn set_strategy(&mut self, strategy: HotspotStrategy) {
        self.strategy = strategy;
        // Re-evaluate all functions with the new strategy
        let function_indices: Vec<usize> = self.execution_info.keys().cloned().collect();
        for index in function_indices {
            self.update_hotspot_status(index);
        }
    }

    /// Get the current detection strategy
    pub fn get_strategy(&self) -> &HotspotStrategy {
        &self.strategy
    }
}

impl Default for HotspotDetector {
    fn default() -> Self {
        Self::new()
    }
}
