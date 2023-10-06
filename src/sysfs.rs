use anyhow::{bail, Context};
use std::{
    fs::OpenOptions,
    io::{Error, ErrorKind},
    vec::Vec,
};

use crate::model::AllowedValues;
use crate::{
    model::{CpuId, EnergyPerformancePreference, ScalingGovernor},
    sysfs,
};

const AMD_PSTATE: &str = "/sys/devices/system/cpu/amd_pstate/status";

const CPU_PRESENT: &str = "/sys/devices/system/cpu/present";

const SCALING_GETSET: &str = "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_governor";
const SCALING_AVAIL: &str = "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_available_governors";

const EPP_GETSET: &str = "/sys/devices/system/cpu/cpu{}/cpufreq/energy_performance_preference";
const EPP_AVAIL: &str =
    "/sys/devices/system/cpu/cpu{}/cpufreq/energy_performance_available_preferences";

pub(crate) const CPU_MIN_FREQ: &str = "/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_min_freq";
pub(crate) const CPU_MAX_FREQ: &str = "/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq";
pub(crate) const CPU_CUR_FREQ: &str = "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq";

impl AllowedValues for ScalingGovernor {
    fn new(value: String) -> anyhow::Result<Self> {
        if ScalingGovernor::valid(&value) {
            Ok(ScalingGovernor(value))
        } else {
            bail!("Unsupported value {value} for ScalingGovernor")
        }
    }

    fn all() -> Vec<String> {
        let cpu = first_cpu();
        read_string_list_value(&cpu.path_for(SCALING_AVAIL)).unwrap_or(vec![])
    }
}

impl AllowedValues for EnergyPerformancePreference {
    fn new(value: String) -> anyhow::Result<Self> {
        if EnergyPerformancePreference::valid(&value) {
            Ok(EnergyPerformancePreference(value))
        } else {
            bail!("Unsupported value {value} for EnergyPerformancePreference")
        }
    }

    fn all() -> Vec<String> {
        let cpu = first_cpu();
        read_string_list_value(&cpu.path_for(EPP_AVAIL)).unwrap_or(vec![])
    }
}

fn read_file(path: &str) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

fn write_file_string(path: &str, value: &str) -> std::io::Result<()> {
    std::fs::write(path, value)
}

fn read_string_value(path: &str) -> std::io::Result<String> {
    read_file(path).map(|s| s.trim().to_owned())
}

pub(crate) fn read_int_value(path: &str) -> anyhow::Result<u32> {
    (read_string_value(path)?)
        .parse::<u32>()
        .context("invalid int")
}

fn read_string_list_value(path: &str) -> std::io::Result<Vec<String>> {
    let value = read_string_value(path)?;
    let values = value.split(' ').map(String::from).collect();
    Ok(values)
}

fn read_int_range_value(path: &str) -> std::io::Result<(u8, u8)> {
    if let Some((from, to)) = read_string_value(path)?.split_once('-') {
        return Ok((
            str::parse::<u8>(from).unwrap(),
            str::parse::<u8>(to).unwrap(),
        ));
    }
    Err(Error::new(
        ErrorKind::InvalidData,
        format!("Not an int range under: {}", path),
    ))
}

pub(crate) fn is_amd_pstate_enabled() -> bool {
    let value = read_string_value(AMD_PSTATE).unwrap_or("n/a".to_owned());
    value == "active"
}

fn first_cpu() -> CpuId {
    let all_cpus = get_cpus().expect("List of CPUs expected");
    let cpu = all_cpus
        .first()
        .expect("At least 1 CPU expected")
        .to_owned();
    cpu
}

pub(crate) fn is_governor_and_epp_writable() -> std::io::Result<bool> {
    let cpu = first_cpu();

    let path1 = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(cpu.path_for(SCALING_GETSET));

    let path2 = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(cpu.path_for(EPP_GETSET));

    if path1.is_ok() && path2.is_ok() {
        return Ok(true);
    }

    Err(Error::new(
        ErrorKind::PermissionDenied,
        "Can't write settings, perhaps you need to be ROOT?",
    ))
}

pub(crate) fn get_cpus() -> std::io::Result<Vec<CpuId>> {
    let (from, to) = read_int_range_value(CPU_PRESENT)?;
    Ok((from..=to).map(CpuId).collect())
}

#[derive(Debug)]
pub(crate) struct Configuration {
    pub scaling_governor: ScalingGovernor,
    pub epp_preference: EnergyPerformancePreference,
}

impl Configuration {
    pub(crate) fn read() -> anyhow::Result<Self> {
        let cpu = first_cpu();

        let scaling_value = read_string_value(&cpu.path_for(SCALING_GETSET))?;
        let governor = ScalingGovernor::new(scaling_value)?;

        let epp_value = read_string_value(&cpu.path_for(EPP_GETSET))?;
        let epp = EnergyPerformancePreference::new(epp_value)?;

        Ok(Self {
            scaling_governor: governor,
            epp_preference: epp,
        })
    }

    pub(crate) fn save(&self) -> std::io::Result<()> {
        if !is_governor_and_epp_writable()? {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "No permission to update settings, perhaps you need to be ROOT",
            ));
        }
        let all_cpu: Vec<CpuId> = get_cpus()?;
        for cpu in all_cpu.iter() {
            sysfs::write_file_string(&cpu.path_for(SCALING_GETSET), &self.scaling_governor.0)?;
            sysfs::write_file_string(&cpu.path_for(EPP_GETSET), &self.epp_preference.0)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_sysfs() {
        assert!(read_file(CPU_PRESENT).unwrap().starts_with("0-"))
    }

    #[test]
    fn read_int_range() {
        let cpus = read_int_range_value(CPU_PRESENT).unwrap();
        assert_eq!(cpus.0, 0);
        assert!(cpus.1 >= 1);
    }
}
