use anyhow::bail;
use clap::{Parser, Subcommand};
use console::style;
use model::{AllowedValues, EnergyPerformancePreference, ScalingGovernor};
use sysfs::Configuration;
mod model;
mod monitor;
mod sysfs;

#[derive(Parser)]
#[command(author, version, about = "Read or change the amd_pstate_epp kernel driver settings", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
#[command(about = "Read or change the amd_pstate_epp kernel driver settings")]
pub(crate) enum Commands {
    #[command(about = "interactive settings UI (default)")]
    UI,
    #[command(about = "get the current settings")]
    Get,
    #[command(about = "set new settings")]
    Set {
        #[arg(short)]
        scaling_governor: std::string::String,
        #[arg(short)]
        epp_preference: std::string::String,
    },
    #[command(id = "mon", about = "CPU Frequency monitor graph")]
    Monitor,
}

fn get_config(config: &Configuration) -> anyhow::Result<()> {
    println!("{}", style(" amd_pstate_epp ").on_blue().black());
    println!();
    println!("{}", style("ScalingGovernor").blue().underlined());
    for gov in ScalingGovernor::all() {
        if gov == config.scaling_governor.0 {
            println!("{}", style(format!(" ● {}", gov)).green().bold());
        } else {
            println!(" ○ {}", gov);
        }
    }
    println!();
    println!(
        "{}",
        style("EnergyPerformancePreference").blue().underlined()
    );
    for epp in EnergyPerformancePreference::all() {
        if epp == config.epp_preference.0 {
            println!("{}", style(format!(" ● {}", epp)).green().bold());
        } else {
            println!(" ○ {}", epp);
        }
    }
    println!();

    Ok(())
}

fn interactive_options(config: &Configuration) -> anyhow::Result<()> {
    cliclack::intro(style(" amd_pstate_epp ").on_blue().black())?;

    let mut sel_gov = cliclack::select(style("ScalingGovernor").underlined())
        .initial_value(config.scaling_governor.0.to_owned());
    for gov in ScalingGovernor::all() {
        let hint = if gov == config.scaling_governor.0 {
            "current"
        } else {
            ""
        };
        sel_gov = sel_gov.item(gov.to_string(), gov.to_string(), hint);
    }
    let gov = sel_gov.interact();

    let mut sel_epp = cliclack::select(style("EnergyPerformancePreference").underlined())
        .initial_value(config.epp_preference.0.to_owned());
    for epp in EnergyPerformancePreference::all() {
        let hint = if epp == config.epp_preference.0 {
            "current"
        } else {
            ""
        };
        sel_epp = sel_epp.item(epp.to_string(), epp.to_string(), hint);
    }
    let epp = sel_epp.interact();

    if let (Ok(gov), Ok(epp)) = (gov, epp) {
        let new_config = Configuration {
            scaling_governor: ScalingGovernor::new(gov)?,
            epp_preference: EnergyPerformancePreference::new(epp)?,
        };

        if new_config.scaling_governor != config.scaling_governor
            || new_config.epp_preference != config.epp_preference
        {
            let apply = cliclack::confirm("Apply new settings?")
                .initial_value(false)
                .interact()?;

            if apply {
                let saved = new_config.save();
                match &saved {
                    Ok(_) => cliclack::outro("Settings applied")?,
                    Err(e) => cliclack::outro_cancel(format!("Failed to apply settings, {} ", e))?,
                }
                return saved.map_err(anyhow::Error::from);
            }
        }
    }

    cliclack::outro_cancel("Nothing changed")?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    if !sysfs::is_amd_pstate_enabled() {
        bail!("`amd_pstate_epp` kernel driver is not in `active` mode or sysfs not accessible",);
    }

    let cfg = Configuration::read()?;

    match Cli::parse().command {
        None | Some(Commands::UI) => {
            ctrlc::set_handler(|| ()).unwrap(); // without this, cliclack can't abort selection
            let res = interactive_options(&cfg);
            console::Term::stdout().show_cursor()?; // Ensure the cursor is back!
            res
        }
        Some(Commands::Get) => get_config(&cfg),
        Some(Commands::Set {
            scaling_governor,
            epp_preference,
        }) => {
            let new_config = Configuration {
                scaling_governor: ScalingGovernor::new(scaling_governor)?,
                epp_preference: EnergyPerformancePreference::new(epp_preference)?,
            };
            let saved = new_config.save();
            match &saved {
                Ok(_) => println!("Configuration Saved"),
                Err(e) => {
                    println!("Failed to save configuration, perhaps you need ROOT access");
                    eprintln!("Error {}", e);
                }
            }
            saved.map_err(anyhow::Error::from)
        }
        Some(Commands::Monitor) => monitor::Monitor::start(),
    }
}
