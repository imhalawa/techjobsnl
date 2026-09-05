namespace TechJobsNL.Core.Domain;

/// <summary>Reports a company scan that failed without affecting other Company Profiles.</summary>
public sealed record CompanyFailed(string CompanyId, SourceErrorKind Kind, string Diagnostic) : ScanEvent;
