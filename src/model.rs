#[derive(Debug, Clone)]
pub struct CpuId(pub u8);

impl CpuId {
    pub(crate) fn path_for(&self, path: &str) -> std::string::String {
        path.replace("{}", &self.0.to_string())
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
