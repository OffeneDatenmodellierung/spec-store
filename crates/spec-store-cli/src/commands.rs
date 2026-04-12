use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "spec-store",
    version,
    about = "Codebase spec registry with quality gates and agent coordination"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialise spec-store in the current git repo
    Init(InitArgs),

    /// Search for functions, decisions, or features
    Search(SearchArgs),

    /// Register code entities explicitly
    #[command(subcommand)]
    Register(RegisterCommand),

    /// Record architectural decisions
    #[command(subcommand)]
    Decision(DecisionCommand),

    /// Scan for unregistered functions
    Catchup(CatchupArgs),

    /// Coverage gate commands
    #[command(subcommand)]
    Coverage(CoverageCommand),

    /// Quality gate commands
    #[command(subcommand)]
    Quality(QualityCommand),

    /// Worktree coordination
    #[command(subcommand)]
    Worktree(WorktreeCommand),

    /// Generate agent context file
    Context(ContextArgs),

    /// Project health summary
    Status,
}

#[derive(Args)]
pub struct InitArgs {}

#[derive(Args)]
pub struct SearchArgs {
    /// Natural language query
    pub query: String,
    /// Filter by type: fn | decision | feature
    #[arg(long)]
    pub r#type: Option<String>,
    /// Max results to return
    #[arg(long, default_value = "5")]
    pub limit: usize,
}

#[derive(Subcommand)]
pub enum RegisterCommand {
    /// Register a function
    Fn(RegisterFnArgs),
    /// Register a decision (alias: spec-store decision add)
    Decision(RegisterDecisionArgs),
}

#[derive(Args)]
pub struct RegisterFnArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub file: String,
    #[arg(long, default_value = "0")]
    pub line: usize,
    #[arg(long)]
    pub desc: String,
}

#[derive(Args)]
pub struct RegisterDecisionArgs {
    pub text: String,
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,
}

#[derive(Subcommand)]
pub enum DecisionCommand {
    /// Add an architectural decision
    Add(RegisterDecisionArgs),
    /// List all decisions
    List,
}

#[derive(Args)]
pub struct CatchupArgs {
    /// Only scan staged git files
    #[arg(long)]
    pub staged: bool,
    /// Scan a specific path
    #[arg(long)]
    pub path: Option<String>,
    /// Auto-register without prompting (for hooks)
    #[arg(long)]
    pub auto_register: bool,
    /// Exit non-zero if any unregistered functions found
    #[arg(long)]
    pub fail_on_missing: bool,
}

#[derive(Subcommand)]
pub enum CoverageCommand {
    /// Show full per-file coverage report
    Report(CoverageReportArgs),
    /// Check coverage gates (used by pre-push hook)
    Check(CoverageCheckArgs),
    /// Set or update baseline from current lcov.info
    Baseline(CoverageBaselineArgs),
}

#[derive(Args)]
pub struct CoverageReportArgs {
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub file: Option<String>,
}

#[derive(Args)]
pub struct CoverageCheckArgs {
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub fail_on_regression: bool,
}

#[derive(Args)]
pub struct CoverageBaselineArgs {
    #[arg(long)]
    pub from: Option<String>,
    /// Ratchet all files up (non-destructive)
    #[arg(long)]
    pub update: bool,
}

#[derive(Subcommand)]
pub enum QualityCommand {
    /// Check quality gates
    Check(QualityCheckArgs),
    /// Full quality report across the project
    Report,
}

#[derive(Args)]
pub struct QualityCheckArgs {
    #[arg(long)]
    pub staged: bool,
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long)]
    pub file: Option<String>,
}

#[derive(Subcommand)]
pub enum WorktreeCommand {
    /// Claim a worktree for exclusive development
    Claim(WorktreeClaimArgs),
    /// Release a claimed worktree
    Release(WorktreeReleaseArgs),
    /// List all active worktrees
    List,
    /// Verify no conflicts (used by pre-push hook)
    Verify,
}

#[derive(Args)]
pub struct WorktreeClaimArgs {
    pub branch: String,
    #[arg(long)]
    pub contract: Option<String>,
    #[arg(long)]
    pub owner: Option<String>,
}

#[derive(Args)]
pub struct WorktreeReleaseArgs {
    pub branch: String,
}

#[derive(Args)]
pub struct ContextArgs {
    #[arg(long)]
    pub worktree: Option<String>,
    #[arg(long, default_value = "AGENTS.md")]
    pub output: String,
}
