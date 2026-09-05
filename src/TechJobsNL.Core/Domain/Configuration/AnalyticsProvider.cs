namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Identifies the retained analytics provider setting.</summary>
public enum AnalyticsProvider
{
    /// <summary>Uses deterministic local analysis.</summary>
    Local,

    /// <summary>Retains the Rust Claude setting without executing it.</summary>
    Claude,

    /// <summary>Retains the Rust Codex setting without executing it.</summary>
    Codex
}
