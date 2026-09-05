namespace TechJobsNL.Core.Operations;

/// <summary>The durable health of a company's last source attempt.</summary>
public enum OperationalHealth
{
    Unknown,
    Healthy,
    Incomplete,
    Failed,
}
