pub mod analyzer;
pub mod engine;
pub mod executor;
pub mod loop_;
pub mod planner;
pub mod types;

pub use analyzer::SelfAnalyzer;
pub use engine::{EvolutionConfig, EvolutionEngine};
pub use executor::ExecutionEngine;
pub use loop_::EvolutionLoop;
pub use planner::ImprovementPlanner;
pub use types::{
    CodeAnalysis, CodeMetrics, ImprovementPlan, ImprovementType, Issue, IterationResult, Priority,
    Severity,
};
