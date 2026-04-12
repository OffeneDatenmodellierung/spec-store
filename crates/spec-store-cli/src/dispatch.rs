use crate::commands::*;
use clap::Parser;
use colored::Colorize;
use spec_store_core::{
    context::ContextOptions, coverage::checker, ops, scanner::quality, AppContext,
};

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init(args) => cmd_init(args),
        Command::Search(args) => {
            let ctx = AppContext::load()?;
            cmd_search(args, &ctx)
        }
        Command::Register(cmd) => {
            let mut ctx = AppContext::load()?;
            cmd_register(cmd, &mut ctx)
        }
        Command::Decision(cmd) => {
            let mut ctx = AppContext::load()?;
            cmd_decision(cmd, &mut ctx)
        }
        Command::Catchup(args) => {
            let ctx = AppContext::load()?;
            cmd_catchup(args, &ctx)
        }
        Command::Coverage(cmd) => {
            let mut ctx = AppContext::load()?;
            cmd_coverage(cmd, &mut ctx)
        }
        Command::Quality(cmd) => {
            let ctx = AppContext::load()?;
            cmd_quality(cmd, &ctx)
        }
        Command::Worktree(cmd) => {
            let mut ctx = AppContext::load()?;
            cmd_worktree(cmd, &mut ctx)
        }
        Command::Context(args) => {
            let ctx = AppContext::load()?;
            cmd_context(args, &ctx)
        }
        Command::Status => {
            let ctx = AppContext::load()?;
            cmd_status(&ctx)
        }
    }
}

fn cmd_init(_args: InitArgs) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    ops::init(&root)?;
    println!("{} spec-store initialised", crate::TICK.green());
    println!("  Git hooks:   .githooks/ (core.hooksPath set)");
    println!("  Config:      .spec-store/config.toml");
    Ok(())
}

fn cmd_search(args: SearchArgs, ctx: &AppContext) -> anyhow::Result<()> {
    let results = ops::search(ctx, &args.query, args.limit);
    if results.is_empty() {
        println!("No results for {:?}", args.query);
        return Ok(());
    }
    println!("\nSearch results for {:?}\n", args.query);
    for r in &results {
        let name = r.payload["name"].as_str().unwrap_or(&r.id);
        let file = r.payload["file"].as_str().unwrap_or("");
        println!(
            "  {} {:.2}  {} ({})",
            crate::TICK.green(),
            r.score,
            name.bold(),
            file.dimmed()
        );
    }
    Ok(())
}

fn cmd_register(cmd: RegisterCommand, ctx: &mut AppContext) -> anyhow::Result<()> {
    match cmd {
        RegisterCommand::Fn(args) => {
            ops::register_fn(ctx, &args.name, &args.file, args.line, &args.desc)?;
            println!("{} Registered fn `{}`", crate::TICK.green(), args.name);
        }
        RegisterCommand::Decision(args) => {
            ops::add_decision(ctx, &args.text, &args.tags)?;
            println!("{} Decision recorded", crate::TICK.green());
        }
    }
    Ok(())
}

fn cmd_decision(cmd: DecisionCommand, ctx: &mut AppContext) -> anyhow::Result<()> {
    match cmd {
        DecisionCommand::Add(args) => {
            ops::add_decision(ctx, &args.text, &args.tags)?;
            println!("{} Decision recorded", crate::TICK.green());
        }
        DecisionCommand::List => {
            let decisions = ops::list_decisions(ctx)?;
            for d in &decisions {
                println!("[{}] {}", &d.created_at[..10], d.text);
            }
        }
    }
    Ok(())
}

fn cmd_catchup(args: CatchupArgs, ctx: &AppContext) -> anyhow::Result<()> {
    let result = ops::catchup(ctx, args.path.as_deref(), args.staged)?;
    if result.missing.is_empty() {
        println!("{} All functions registered", crate::TICK.green());
        return Ok(());
    }
    if args.auto_register {
        let count = result.missing.len();
        for f in &result.missing {
            let _ =
                ctx.structured
                    .register_fn(&spec_store_core::store::structured::RegisterFnInput {
                        name: &f.name,
                        file: &f.file,
                        line: f.line,
                        desc: "(auto-registered)",
                        is_test: f.is_test,
                    });
        }
        println!(
            "{} Auto-registered {} functions",
            crate::TICK.green(),
            count
        );
        return Ok(());
    }
    println!(
        "UNREGISTERED ({} items)\n{}",
        result.missing.len(),
        "━".repeat(40)
    );
    for f in &result.missing {
        let tag = if f.is_test { " [test]" } else { "" };
        println!(
            "  {} {}(){tag} — no description",
            f.file.dimmed(),
            f.name.yellow()
        );
    }
    if args.fail_on_missing {
        anyhow::bail!("{} unregistered functions", result.missing.len());
    }
    Ok(())
}

fn cmd_coverage(cmd: CoverageCommand, ctx: &mut AppContext) -> anyhow::Result<()> {
    match cmd {
        CoverageCommand::Report(args) => {
            let report = ops::check_coverage(ctx, args.from.as_deref())?;
            let input = crate::reporter::ReportInput::new(
                &report.coverage,
                &report.results,
                &ctx.config.coverage,
                &ctx.baseline,
            );
            if args.json {
                crate::reporter::print_report_json(&input);
            } else {
                crate::reporter::print_report(&input);
            }
        }
        CoverageCommand::Check(args) => {
            let report = ops::check_coverage(ctx, args.from.as_deref())?;
            checker::assert_no_failures(&report.results).map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("{} Coverage gates passed", crate::TICK.green());
        }
        CoverageCommand::Baseline(args) => {
            let count = ops::update_baseline(ctx, args.from.as_deref())?;
            println!(
                "{} Baseline updated for {} files",
                crate::TICK.green(),
                count
            );
        }
    }
    Ok(())
}

fn cmd_quality(cmd: QualityCommand, ctx: &AppContext) -> anyhow::Result<()> {
    match cmd {
        QualityCommand::Check(args) => {
            let violations = if args.staged {
                ops::check_quality_staged(ctx)?
            } else {
                ops::check_quality(ctx, args.path.as_deref(), args.file.as_deref())?
            };
            crate::reporter::print_quality_report(&violations);
            if quality::has_errors(&violations, ctx.config.quality.warn_only) {
                anyhow::bail!("Quality gates failed");
            }
        }
        QualityCommand::Report => {
            let violations = ops::check_quality(ctx, None, None)?;
            crate::reporter::print_quality_report(&violations);
        }
    }
    Ok(())
}

fn cmd_worktree(cmd: WorktreeCommand, ctx: &mut AppContext) -> anyhow::Result<()> {
    match cmd {
        WorktreeCommand::Claim(args) => {
            ops::claim_worktree(
                ctx,
                &args.branch,
                args.contract.as_deref(),
                args.owner.as_deref(),
            )?;
            println!("{} Claimed worktree `{}`", crate::TICK.green(), args.branch);
        }
        WorktreeCommand::Release(args) => {
            ops::release_worktree(ctx, &args.branch)?;
            println!(
                "{} Released worktree `{}`",
                crate::TICK.green(),
                args.branch
            );
        }
        WorktreeCommand::List => {
            let worktrees = ops::list_worktrees(ctx)?;
            if worktrees.is_empty() {
                println!("No active worktrees");
                return Ok(());
            }
            for wt in &worktrees {
                println!(
                    "  {} ({})",
                    wt.branch.bold(),
                    wt.owner.as_deref().unwrap_or("unassigned")
                );
            }
        }
        WorktreeCommand::Verify => {
            let conflicts = ops::verify_worktrees(ctx)?;
            if conflicts.is_empty() {
                println!("{} No worktree conflicts detected", crate::TICK.green());
            } else {
                for c in &conflicts {
                    println!("  {} {}", "✗".red(), c);
                }
                anyhow::bail!("{} worktree conflict(s) detected", conflicts.len());
            }
        }
    }
    Ok(())
}

fn cmd_context(args: ContextArgs, ctx: &AppContext) -> anyhow::Result<()> {
    let opts = ContextOptions {
        worktree: args.worktree,
        output: args.output.clone(),
        min_coverage: ctx.config.coverage.min_per_file,
    };
    let content = ops::generate_context(ctx, &opts)?;
    ops::write_context(ctx, &content, &args.output)?;
    println!("{} Context written to {}", crate::TICK.green(), args.output);
    Ok(())
}

fn cmd_status(ctx: &AppContext) -> anyhow::Result<()> {
    let report = ops::status(ctx)?;
    println!("\n{}\n", "spec-store status".bold());
    println!("  Functions registered:   {}", report.function_count);
    println!("  Decisions recorded:     {}", report.decision_count);
    println!("  Active worktrees:       {}", report.worktree_count);
    println!("  Coverage baselines:     {}", report.baseline_count);
    println!(
        "  Git hooks installed:    {}",
        if report.hooks_installed {
            "yes".green()
        } else {
            "no (run spec-store init)".red()
        }
    );
    Ok(())
}
