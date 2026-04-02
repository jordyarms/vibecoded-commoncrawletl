use clap::Parser;
use tracing_subscriber::EnvFilter;

use commoncrawletl::checkpoint::Checkpoint;
use commoncrawletl::cli::{Cli, Command};
use commoncrawletl::domain::{scoring, signals};
use commoncrawletl::error::Result;
use commoncrawletl::extract::event;
use commoncrawletl::geo::filter;
use commoncrawletl::output::generate;
use commoncrawletl::parts::prioritize;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let checkpoint_path = cli.workdir.join(&cli.checkpoint);
    let mut checkpoint = Checkpoint::load(&checkpoint_path)?;

    match cli.command {
        Command::Analyze { lookup, stats } => {
            signals::run(&cli.workdir, &lookup, &stats, &mut checkpoint, &checkpoint_path)?;
        }
        Command::Prioritize => {
            prioritize::run(&cli.workdir, &mut checkpoint, &checkpoint_path)?;
        }
        Command::Extract { parts_dir, jobs } => {
            event::run(
                &cli.workdir,
                &parts_dir,
                jobs,
                &mut checkpoint,
                &checkpoint_path,
            )?;
        }
        Command::Geofilter => {
            filter::run(&cli.workdir, &mut checkpoint, &checkpoint_path)?;
        }
        Command::Score => {
            scoring::run(&cli.workdir, &mut checkpoint, &checkpoint_path)?;
        }
        Command::Output => {
            generate::run(&cli.workdir, &mut checkpoint, &checkpoint_path)?;
        }
        Command::Run {
            lookup,
            stats,
            parts_dir,
            jobs,
        } => {
            if !checkpoint.phase1_complete {
                signals::run(&cli.workdir, &lookup, &stats, &mut checkpoint, &checkpoint_path)?;
            }
            if !checkpoint.phase2_complete {
                prioritize::run(&cli.workdir, &mut checkpoint, &checkpoint_path)?;
            }
            if !checkpoint.phase3_complete {
                event::run(
                    &cli.workdir,
                    &parts_dir,
                    jobs,
                    &mut checkpoint,
                    &checkpoint_path,
                )?;
            }
            if !checkpoint.phase4_complete {
                filter::run(&cli.workdir, &mut checkpoint, &checkpoint_path)?;
            }
            if !checkpoint.phase5_complete {
                scoring::run(&cli.workdir, &mut checkpoint, &checkpoint_path)?;
            }
            if !checkpoint.phase6_complete {
                generate::run(&cli.workdir, &mut checkpoint, &checkpoint_path)?;
            }
        }
    }

    Ok(())
}
