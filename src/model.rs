use std::collections::VecDeque;

#[derive(Debug, Copy, Clone)]
pub struct CpuId(pub u8);

impl CpuId {
    pub(crate) fn path_for(&self, path: &str) -> std::string::String {
        path.replace("{}", &self.0.to_string())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CpuFrequencyHistory {
    pub(crate) running: bool,
    pub(crate) history: usize,
    pub(crate) min_value: u32,
    pub(crate) max_value: u32,
    pub(crate) data: Vec<VecDeque<u32>>,
}

impl CpuFrequencyHistory {
    pub(crate) fn new(cpu_count: usize, history: usize, min_value: u32, max_value: u32) -> Self {
        Self {
            running: true,
            history,
            min_value,
            max_value,
            data: vec![VecDeque::from(vec![0u32; history]); cpu_count],
        }
    }

    pub(crate) fn append(&mut self, values: Vec<u32>) {
        for (cpu, new_value) in values.into_iter().enumerate() {
            let cpu_data = self
                .data
                .get_mut(cpu)
                .expect("Mismatch between struct and append data shape");
            cpu_data.pop_front();
            cpu_data.push_back(new_value);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScalingGovernor(pub std::string::String);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EnergyPerformancePreference(pub std::string::String);

pub(crate) trait AllowedValues: Sized {
    fn new(value: std::string::String) -> anyhow::Result<Self>;

    fn all() -> Vec<std::string::String>;

    fn valid(value: &std::string::String) -> bool {
        Self::all().contains(value)
    }
}
