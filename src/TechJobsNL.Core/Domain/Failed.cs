namespace TechJobsNL.Core.Domain;

/// <summary>Preserves the legacy source-level failure event contract.</summary>
public sealed record Failed(string CompanyId, SourceErrorKind Kind, string Diagnostic) : ScanEvent;
